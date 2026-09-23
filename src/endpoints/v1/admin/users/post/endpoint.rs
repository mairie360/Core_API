use crate::database::auth::register::RegisterUserQueryView;
use crate::database::get_user_id::GetUserIdQueryView;
use crate::endpoints::v1::admin::users::post::view::CreateUserView;
use crate::keycloak::sync::{
    discard_account, export_user, invite, link_account, reserve_account, sync_roles,
    ReservedAccount, SyncError,
};
use crate::keycloak::{KeycloakAdminClient, KeycloakUserProfile};
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
enum CreateUserError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
    /// Keycloak refused or failed before the Core account was written: nothing was created.
    Keycloak(SyncError),
    /// The Core account exists but its link, roles or invitation could not be completed in
    /// Keycloak.
    KeycloakIncomplete(SyncError),
}

impl std::fmt::Display for CreateUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData => write!(f, "Invalid data provided"),
            Self::UserAlreadyExists => write!(f, "User already exists"),
            Self::DatabaseError => write!(f, "Database error occurred"),
            Self::Keycloak(error) => write!(f, "{error}"),
            Self::KeycloakIncomplete(error) => write!(
                f,
                "The account was created in Mairie 360 but not fully synchronised with Keycloak ({error}). Run POST /api/v1/admin/keycloak/migration to complete it."
            ),
        }
    }
}

impl ResponseError for CreateUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidData => StatusCode::BAD_REQUEST,
            Self::UserAlreadyExists | Self::Keycloak(SyncError::EmailTaken) => StatusCode::CONFLICT,
            Self::DatabaseError
            | Self::Keycloak(SyncError::Database | SyncError::LinkedToAnotherUser) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Keycloak(SyncError::NotConfigured) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Keycloak(SyncError::KeycloakForbidden | SyncError::KeycloakUnavailable)
            | Self::KeycloakIncomplete(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn is_valid_email(email: &str) -> bool {
    if email.is_empty() {
        return false;
    }
    email.find('@').is_some_and(|index| {
        let domain = &email[index + 1..];
        !domain.is_empty() && domain.contains('.')
    })
}

const fn is_valid_password(password: &str) -> bool {
    //Need to be more complex and based on requirements
    password.len() >= 8
}

fn is_valid_phone_number(phone_number: Option<&str>) -> bool {
    //Need to be more complex and based on requirements
    phone_number.is_none_or(|num| num.len() >= 10 && num.chars().all(|c| c.is_ascii_digit()))
}

async fn can_be_registered(
    register_view: &CreateUserView,
    smart_db: &SmartDatabase,
) -> Result<(), CreateUserError> {
    if !is_valid_email(register_view.email()) {
        return Err(CreateUserError::InvalidData);
    }

    let exists: bool = smart_db
        .fetch_scalar(&DoesUserExistByEmailQueryView::new(
            register_view.email().to_string(),
        ))
        .await
        .map_err(|_| CreateUserError::DatabaseError)?;

    if exists {
        return Err(CreateUserError::UserAlreadyExists);
    }

    if !is_valid_password(register_view.password()) {
        return Err(CreateUserError::InvalidData);
    }
    if !is_valid_phone_number(register_view.phone_number()) {
        return Err(CreateUserError::InvalidData);
    }
    Ok(())
}

async fn insert_user(
    register_view: &CreateUserView,
    smart_db: &SmartDatabase,
) -> Result<(), CreateUserError> {
    let view = RegisterUserQueryView::new(
        register_view.first_name(),
        register_view.last_name(),
        register_view.email(),
        register_view.password(),
        register_view.phone_number(),
    );

    let success: bool = smart_db.fetch_scalar(&view).await.map_err(|e| {
        eprintln!("Database error: {e}");
        CreateUserError::DatabaseError
    })?;

    if success {
        Ok(())
    } else {
        Err(CreateUserError::DatabaseError)
    }
}

/// Reserves the Keycloak account of the user being created and, for a brand-new account,
/// sends the password set-up invitation. Both happen before the Core insert, so a Keycloak
/// failure leaves nothing behind.
async fn reserve_keycloak_account(
    admin: &KeycloakAdminClient,
    register_view: &CreateUserView,
) -> Result<ReservedAccount, CreateUserError> {
    let profile = KeycloakUserProfile {
        email: register_view.email().to_string(),
        first_name: register_view.first_name().to_string(),
        last_name: register_view.last_name().to_string(),
        enabled: true,
    };
    let account = reserve_account(admin, &profile)
        .await
        .map_err(CreateUserError::Keycloak)?;
    if account.created {
        if let Err(error) = invite(admin, &account.keycloak_id).await {
            discard_account(admin, &account).await;
            return Err(CreateUserError::Keycloak(error));
        }
    }
    Ok(account)
}

/// Links the freshly inserted Core account to its Keycloak account and mirrors its roles (the
/// schema grants a default one).
async fn complete_keycloak_account(
    admin: &KeycloakAdminClient,
    smart_db: &SmartDatabase,
    email: &str,
    account: &ReservedAccount,
) -> Result<(), SyncError> {
    let user_id: i32 = smart_db
        .fetch_scalar(&GetUserIdQueryView::new(email))
        .await
        .map_err(|e| {
            eprintln!("Database error: {e}");
            SyncError::Database
        })?;
    link_account(smart_db, user_id, &account.keycloak_id).await?;
    let roles = export_user(smart_db, user_id)
        .await?
        .map(|user| user.roles)
        .unwrap_or_default();
    sync_roles(admin, &mut HashMap::new(), &account.keycloak_id, &roles).await?;
    Ok(())
}

async fn register_user(
    register_view: &CreateUserView,
    state: web::Data<AppState>,
    admin: Option<&KeycloakAdminClient>,
) -> Result<(), CreateUserError> {
    let smart_db = state.get_smart_db();
    can_be_registered(register_view, smart_db).await?;

    let Some(admin) = admin else {
        return insert_user(register_view, smart_db).await;
    };

    let account = reserve_keycloak_account(admin, register_view).await?;
    if let Err(error) = insert_user(register_view, smart_db).await {
        discard_account(admin, &account).await;
        return Err(error);
    }
    complete_keycloak_account(admin, smart_db, register_view.email(), &account)
        .await
        .map_err(|error| {
            eprintln!(
                "Keycloak sync: account {} created for {} but not completed: {error}",
                account.keycloak_id,
                register_view.email()
            );
            CreateUserError::KeycloakIncomplete(error)
        })
}

#[utoipa::path(
    post,
    path = "",
    summary = "Create a user account (administration)",
    description = "Creates an account on behalf of an administrator, without the person having \
                   to register. Reserved to administrators.\n\n\
                   Same validation rules as `POST /api/v1/auth/register`: e-mail of the form \
                   `local@domain.tld`, password of at least 8 characters, optional phone number \
                   of at least 10 digits. They all share the same `400`.\n\n\
                   The password given here is provisional: the account is flagged as first \
                   connection, and the user's first `POST /api/v1/auth/login` answers `412` to \
                   make them choose their own.\n\n\
                   **Keycloak (MAIR-142).** When Core runs with a confidential Keycloak client, \
                   the account is mirrored in the realm **before** it is written to Core: an \
                   account with the same e-mail is adopted (profile overwritten), otherwise one is \
                   created (e-mail as username, e-mail verified, no credential) and Keycloak is \
                   asked to e-mail it a link to set its password (`UPDATE_PASSWORD` action, \
                   realm SMTP required). The Core account is then created, linked \
                   (`user_identities`) and its default role mapped as a realm role. If Keycloak \
                   refuses or fails before the Core insert, the answer is `409` or `502` and \
                   nothing is created on either side (an account created in the realm by this \
                   call is deleted again). Without Keycloak, or with a public client, only Core \
                   is written.",
    request_body(
        content = CreateUserView,
        description = "Identity, provisional credentials and optional phone number of the account to create.",
        example = json!({
            "first_name": "Jean",
            "last_name": "Dupont",
            "email": "jean.dupont@mairie360.fr",
            "password": "MotDePasse!123",
            "phone_number": "0612345678"
        })
    ),
    responses(
        (
            status = 201,
            description = "Account created (and, with Keycloak, mirrored, linked and invited), waiting for the password change at first login.",
            body = String,
            content_type = "text/plain",
            example = json!("User created successfully!")
        ),
        (
            status = 400,
            description = "Malformed JSON body, or e-mail, password or phone number breaking the rules above.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid data provided")
        ),
        (
            status = 401,
            description = "`Authorization` header missing, JWT invalid or expired, or session revoked.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 409,
            description = "A Core account already uses this e-mail, or Keycloak refuses the e-mail because it is the username of an account carrying another e-mail. Nothing is created.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Core" = (value = json!("User already exists"))),
                ("Keycloak" = (value = json!("Another Keycloak account already uses this e-mail address.")))
            )
        ),
        (
            status = 500,
            description = "Database error during the uniqueness check or the insert (the reserved Keycloak account is deleted again), or the Keycloak account found by e-mail is already linked to another Core account.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Database" = (value = json!("Database error occurred"))),
                ("Linked elsewhere" = (value = json!("The Keycloak account is already linked to another Mairie 360 account.")))
            )
        ),
        (
            status = 502,
            description = "Keycloak could not be reached, refused Core's service account, or could not send the invitation: nothing is created. Or, rarer, the Core account was created but its link or roles could not be completed in Keycloak: the message says so, and replaying `POST /api/v1/admin/keycloak/migration` completes it.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Unavailable" = (value = json!("Keycloak is unavailable."))),
                ("Service account refused" = (value = json!("Keycloak refused Core's service account."))),
                ("Incomplete" = (value = json!("The account was created in Mairie 360 but not fully synchronised with Keycloak (Keycloak is unavailable.). Run POST /api/v1/admin/keycloak/migration to complete it.")))
            )
        ),
        (
            status = 503,
            description = "Keycloak administration is not configured (no confidential client). Not produced in practice: the synchronisation is simply skipped in that case.",
            body = String,
            content_type = "text/plain",
            example = json!("Keycloak administration is not configured: a confidential client (KEYCLOAK_CLIENT_SECRET) is required.")
        )
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn admin_post_user(
    payload: web::Json<CreateUserView>,
    state: web::Data<AppState>,
    admin: Option<web::Data<KeycloakAdminClient>>,
) -> Result<impl Responder, CreateUserError> {
    let register_view = payload.into_inner();

    register_user(
        &register_view,
        state,
        admin.as_ref().map(web::Data::get_ref),
    )
    .await?;

    Ok(HttpResponse::Created().body("User created successfully!"))
}
