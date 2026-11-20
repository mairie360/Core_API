use crate::database::auth::register::RegisterUserQueryView;
use crate::database::get_user_id::GetUserIdQueryView;
use crate::database::ids::id_from_sql;
use crate::emails::AccountEmails;
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::db_error;
use crate::endpoints::v1::admin::users::post::view::{CreateUserView, CreatedUserView};
use crate::endpoints::validation::ValidatedJson;
use crate::passwords;
use crate::redis_keys::{set_token, INVITATION_TTL_SECONDS};
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
enum CreateUserError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for CreateUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData => write!(f, "Invalid data provided"),
            Self::UserAlreadyExists => write!(f, "User already exists"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for CreateUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidData => StatusCode::BAD_REQUEST,
            Self::UserAlreadyExists => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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
        .map_err(|e| {
            db_error::log("create user: e-mail check", &e);
            CreateUserError::DatabaseError
        })?;

    if exists {
        return Err(CreateUserError::UserAlreadyExists);
    }

    if !is_valid_password(register_view.password()) {
        return Err(CreateUserError::InvalidData);
    }
    Ok(())
}

async fn insert_user(
    register_view: &CreateUserView,
    smart_db: &SmartDatabase,
) -> Result<(), CreateUserError> {
    let hashed_password = passwords::hash(register_view.password())
        .await
        .map_err(|e| {
            tracing::error!("Password hashing error: {e}");
            CreateUserError::DatabaseError
        })?;

    let phone = register_view
        .phone()
        .map_err(|_| CreateUserError::InvalidData)?;
    let view = RegisterUserQueryView::new(
        register_view.first_name(),
        register_view.last_name(),
        register_view.email(),
        &hashed_password,
        phone.as_ref(),
    );

    let success: bool = smart_db.fetch_scalar(&view).await.map_err(|e| {
        tracing::error!("Database error: {e}");
        CreateUserError::DatabaseError
    })?;

    if success {
        Ok(())
    } else {
        Err(CreateUserError::DatabaseError)
    }
}

/// E-mails the invitation of the new account: a one-time link to the password page of the login
/// front, valid [`INVITATION_TTL_SECONDS`], consumed by `POST /auth/reset_password` like a
/// forgot-password token. Only the `token -> e-mail` mapping is stored: a pending invitation must
/// not silence a later `POST /auth/forgot_password` of the same address.
///
/// A failure is logged and never undoes the account: the user can still ask for a reset link on
/// the login page.
async fn invite_user(
    state: &AppState,
    emails: Option<&AccountEmails>,
    email: &str,
    first_name: &str,
) {
    let Some(emails) = emails else {
        tracing::error!("Create user: no mailer is registered, the invitation cannot be sent");
        return;
    };
    let token = Uuid::new_v4().to_string();
    let redis = state.get_redis();
    let email_key = format!("{token}/forgot_password_email");
    if let Err(e) = set_token(redis, &email_key, email, INVITATION_TTL_SECONDS).await {
        tracing::error!("Create user: the invitation token could not be stored: {e}");
        return;
    }
    if let Err(e) = emails
        .send_account_invitation(email, first_name, &token)
        .await
    {
        tracing::error!("Create user: the invitation e-mail could not be sent: {e}");
        if let Err(e) = redis.secure_delete(&email_key).await {
            tracing::warn!("Create user: could not drop an unsent invitation token: {e}");
        }
    }
}

async fn register_user(
    register_view: &CreateUserView,
    state: &AppState,
    emails: Option<&AccountEmails>,
) -> Result<(), CreateUserError> {
    let smart_db = state.get_smart_db();
    can_be_registered(register_view, smart_db).await?;
    insert_user(register_view, smart_db).await?;
    invite_user(
        state,
        emails,
        register_view.email(),
        register_view.first_name(),
    )
    .await;
    Ok(())
}

#[utoipa::path(
    post,
    path = "",
    summary = "Create a user account (administration)",
    description = "Creates an account on behalf of an administrator. Reserved to administrators: \
                   this is the only way to create an account, there is no public sign-up route.\n\n\
                   Validation: e-mail of the form `local@domain.tld`, password of at least 8 \
                   characters, optional phone number valid for its `phone_country` (national \
                   format with the country, or E.164). All share the same \
                   `400`.\n\n\
                   The password given here is temporary: the account is flagged as first \
                   connection, and the user's first `POST /api/v1/auth/login` answers `412` so \
                   they choose their own.\n\n\
                   An invitation e-mail (Resend template `account-invitation`) holds a link \
                   to the password page of the login front, valid **7 days**: it is consumed \
                   by `POST /api/v1/auth/reset_password`. A failed e-mail never undoes the \
                   account (the user can ask for a reset link on the login page).",
    request_body(
        content = CreateUserView,
        description = "Identity, provisional credentials and optional phone number of the account to create.",
        example = json!({
            "first_name": "Jean",
            "last_name": "Dupont",
            "email": "jean.dupont@mairie360.fr",
            "password": "MotDePasse!123",
            "phone_number": "06 12 34 56 78",
            "phone_country": "FR"
        })
    ),
    responses(
        (
            status = 201,
            description = "Account created and invited by e-mail, waiting for the password change at first login. The body gives the id of the new account, so the caller does not have to search for it.",
            body = CreatedUserView,
            content_type = "application/json",
            example = json!({ "id": 42, "message": "User created successfully!" })
        ),
        (
            status = 400,
            description = "Malformed JSON body, or a field breaking its rules: `first_name` / `last_name` 1 to 64 characters, no control character; `email` a valid address of at most 320 characters; `password` 8 to 255 characters without control character; `phone_number` not a valid number of `phone_country` (or a national number without `phone_country`, 32 characters at most); `phone_country` not an ISO 3166-1 alpha-2 code, or sent without `phone_number`. The body names the first invalid field.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `password`: must be at least 8 characters")
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
            description = "An account already uses this e-mail. Nothing is created.",
            body = String,
            content_type = "text/plain",
            example = json!("User already exists")
        ),
        (
            status = 500,
            description = "Database error during the uniqueness check or the insert.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        ),
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn admin_post_user(
    _: AdminUser,
    payload: ValidatedJson<CreateUserView>,
    state: web::Data<AppState>,
    emails: Option<web::Data<AccountEmails>>,
) -> Result<impl Responder, CreateUserError> {
    let register_view = payload.into_inner();

    register_user(
        &register_view,
        &state,
        emails.as_ref().map(web::Data::get_ref),
    )
    .await?;

    let id: i32 = state
        .get_smart_db()
        .fetch_scalar(&GetUserIdQueryView::new(register_view.email()))
        .await
        .map_err(|e| {
            tracing::error!("Database error reading the id of the new account: {e}");
            CreateUserError::DatabaseError
        })?;
    Ok(HttpResponse::Created().json(CreatedUserView {
        id: id_from_sql(id),
        message: "User created successfully!".to_string(),
    }))
}
