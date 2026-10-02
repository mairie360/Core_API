use crate::database::ids::id_to_sql;
use crate::database::roles::get_roles_by_id::{GetRolesByIdQueryView, Role};
use crate::database::users::add_role::AddRolesQueryView;
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::db_error::{self, DbFailure};
use crate::endpoints::v1::admin::users::id::roles::post::view::AddRoleToUserView;
use crate::keycloak::sync::{export_user, find_account, map_role, unmap_role, SyncError};
use crate::keycloak::KeycloakAdminClient;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum AddRoleToUserError {
    UserMismatch,
    NotFound,
    AlreadyGranted,
    DatabaseError,
    Keycloak(SyncError),
}

impl std::fmt::Display for AddRoleToUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserMismatch => {
                write!(f, "The body `user_id` does not match the user of the path.")
            }
            Self::NotFound => {
                write!(f, "User or role not found.")
            }
            Self::AlreadyGranted => write!(f, "The user already holds this role."),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::Keycloak(error) => write!(f, "{error}"),
        }
    }
}

impl ResponseError for AddRoleToUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UserMismatch => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::AlreadyGranted | Self::Keycloak(SyncError::EmailTaken) => StatusCode::CONFLICT,
            Self::DatabaseError
            | Self::Keycloak(SyncError::Database | SyncError::LinkedToAnotherUser) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Keycloak(SyncError::NotConfigured) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Keycloak(SyncError::KeycloakForbidden | SyncError::KeycloakUnavailable) => {
                StatusCode::BAD_GATEWAY
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn grant_in_core(
    smart_db: &SmartDatabase,
    user_id: u64,
    view: &AddRoleToUserView,
) -> Result<(), AddRoleToUserError> {
    let view = AddRolesQueryView::new(view.role_id(), user_id);
    smart_db
        .execute(view)
        .await
        .map_err(|e| match db_error::log("grant role", &e) {
            // `user_id` or `role_id` matches nothing (foreign key).
            DbFailure::NotFound => AddRoleToUserError::NotFound,
            DbFailure::Conflict => AddRoleToUserError::AlreadyGranted,
            _ => AddRoleToUserError::DatabaseError,
        })?;

    Ok(())
}

/// Name of the Core role `role_id`, `None` for an unknown role.
///
/// # Errors
///
/// [`SyncError::Database`].
pub(in crate::endpoints::v1::admin::users::id::roles) async fn role_name(
    smart_db: &SmartDatabase,
    role_id: u64,
) -> Result<Option<String>, SyncError> {
    let roles: Vec<Role> = smart_db
        .fetch_all(&GetRolesByIdQueryView::new(vec![id_to_sql(role_id)]))
        .await
        .map_err(|e| {
            tracing::error!("Keycloak sync: cannot read role {role_id}: {e}");
            SyncError::Database
        })?;
    Ok(roles.first().map(|role| role.name().to_string()))
}

async fn add_role_to_user(
    state: web::Data<AppState>,
    admin: Option<&KeycloakAdminClient>,
    user_id: u64,
    view: AddRoleToUserView,
) -> Result<(), AddRoleToUserError> {
    // The user is the one of the path (MAIR-390).
    if view
        .body_user_id()
        .is_some_and(|body_user| body_user != user_id)
    {
        return Err(AddRoleToUserError::UserMismatch);
    }
    let smart_db = state.get_smart_db();
    let Some(admin) = admin else {
        return grant_in_core(smart_db, user_id, &view).await;
    };
    // Unknown role or user (Core answers 404), or account unknown to Keycloak: Core alone.
    let Some(name) = role_name(smart_db, view.role_id())
        .await
        .map_err(AddRoleToUserError::Keycloak)?
    else {
        return grant_in_core(smart_db, user_id, &view).await;
    };
    let Some(user) = export_user(smart_db, id_to_sql(user_id))
        .await
        .map_err(AddRoleToUserError::Keycloak)?
    else {
        return grant_in_core(smart_db, user_id, &view).await;
    };
    let Some(keycloak_id) = find_account(smart_db, admin, &user)
        .await
        .map_err(AddRoleToUserError::Keycloak)?
    else {
        return grant_in_core(smart_db, user_id, &view).await;
    };

    let mapped = map_role(admin, &keycloak_id, &name)
        .await
        .map_err(AddRoleToUserError::Keycloak)?;
    if let Err(error) = grant_in_core(smart_db, user_id, &view).await {
        if mapped {
            if let Err(restore) = unmap_role(admin, &keycloak_id, &name).await {
                tracing::error!(
                    "Keycloak sync: Core refused role {name} for user {user_id} and it could not be unmapped from {keycloak_id}: {restore}"
                );
            }
        }
        return Err(error);
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "/",
    summary = "Grant a role to a user",
    description = "Attaches a role to a user account. Reserved to administrators.\n\n\
                   The target user is always the `userId` of the **path**. The body `user_id` is \
                   deprecated and optional: when sent, it must equal the path's, otherwise the \
                   answer is `400`.\n\n\
                   The response has an empty body. Every Core write failure is reported as \
                   `404`.\n\n\
                   **Keycloak (MAIR-142).** When Core runs with a confidential Keycloak client \
                   and the account is known to Keycloak (recorded link, then e-mail), the role is \
                   mapped as a realm role of the same name (created in the realm when missing) \
                   **before** Core writes it; if Core then refuses (`404`), the mapping added by \
                   this call is removed. An unknown role or user, or an account unknown to \
                   Keycloak, only goes through Core. Without Keycloak, or with a public client, \
                   only Core is written.",
    request_body(
        content = AddRoleToUserView,
        description = "Role to grant.",
        example = json!({ "role_id": 2 })
    ),
    params(
        ("userId" = u64, Path, description = "Id of the user receiving the role.", example = 42)
    ),
    responses(
        (
            status = 200,
            description = "Role granted (and mapped in Keycloak when mirrored). Empty body.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing `role_id`, or body `user_id` different from the path's `userId`.",
            body = String,
            content_type = "text/plain",
            example = json!("The body `user_id` does not match the user of the path.")
        ),
        (
            status = 401,
            description = "`Authorization` header missing, JWT invalid or expired, or session revoked.",
            body = String,
            content_type = "text/plain",
            example = json!("Unauthorized")
        ),
        (
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 404,
            description = "`user_id` or `role_id` matches nothing. The Keycloak mapping, if it had been added, is removed.",
            body = String,
            content_type = "text/plain",
            example = json!("User or role not found.")
        ),
        (
            status = 409,
            description = "The user already holds the role. The Keycloak mapping, if it had been added, is removed.",
            body = String,
            content_type = "text/plain",
            example = json!("The user already holds this role.")
        ),
        (
            status = 500,
            description = "The role or the account could not be read, the Keycloak account found by e-mail is already linked to another Core account (nothing is changed), or the write failed (the Keycloak mapping, if it had been added, is removed). The cause is logged by the server.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 502,
            description = "Keycloak could not be reached or refused Core's service account. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Unavailable" = (value = json!("Keycloak is unavailable."))),
                ("Service account refused" = (value = json!("Keycloak refused Core's service account.")))
            )
        ),
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Users"
)]
#[post("/")]
pub async fn admin_add_role_to_user(
    _: AdminUser,
    state: web::Data<AppState>,
    admin: Option<web::Data<KeycloakAdminClient>>,
    path: web::Path<u64>,
    view: web::Json<AddRoleToUserView>,
) -> Result<impl Responder, AddRoleToUserError> {
    add_role_to_user(
        state,
        admin.as_ref().map(web::Data::get_ref),
        path.into_inner(),
        view.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok())
}
