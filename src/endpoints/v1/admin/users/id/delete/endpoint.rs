use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

use crate::database::sessions::get_active_session_ids::GetActiveSessionIdsQueryView;
use crate::database::users::delete_user::{DeleteUserQueryView, IsUserActiveQueryView};
use crate::keycloak::sync::{
    disable_account, enable_account, export_user, find_account, SyncError,
};
use crate::keycloak::KeycloakAdminClient;
use crate::session_revocation::publish_revoked_sessions;
use uuid::Uuid;

/// SQLSTATE raised by `fn_check_can_delete_user` when the user still owns resources.
const RESTRICT_VIOLATION: &str = "23001";

#[derive(Debug, Clone, PartialEq, Eq)]
enum DeleteUserError {
    UnknownUser,
    OwnsResources,
    DatabaseError,
    Keycloak(SyncError),
}

impl std::fmt::Display for DeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::OwnsResources => write!(
                f,
                "The user still owns groups, events or projects: transfer them first"
            ),
            Self::DatabaseError => write!(f, "Database error occurred"),
            Self::Keycloak(error) => write!(f, "{error}"),
        }
    }
}

impl ResponseError for DeleteUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::OwnsResources | Self::Keycloak(SyncError::EmailTaken) => StatusCode::CONFLICT,
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

fn is_restrict_violation(error: &ApiLibError) -> bool {
    matches!(
        error,
        ApiLibError::Database(DbError::Sqlx(sqlx::Error::Database(db_error)))
            if db_error.code().as_deref() == Some(RESTRICT_VIOLATION)
    )
}

async fn archive_in_core(state: &AppState, user_id: u64) -> Result<(), DeleteUserError> {
    let smart_db = state.get_smart_db();
    // The database drops the user's sessions with the account: read them first, to publish them
    // to the revocation list once the deletion succeeded (MAIR-264).
    let sessions: Vec<Uuid> = smart_db
        .fetch_all(&GetActiveSessionIdsQueryView::new(user_id))
        .await
        .unwrap_or_default();

    smart_db
        .execute(DeleteUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            if is_restrict_violation(&e) {
                DeleteUserError::OwnsResources
            } else {
                eprintln!("Error: {e}");
                DeleteUserError::DatabaseError
            }
        })?;

    publish_revoked_sessions(state.get_redis(), &sessions).await;

    Ok(())
}

async fn delete_user(
    state: web::Data<AppState>,
    admin: Option<&KeycloakAdminClient>,
    user_id: u64,
) -> Result<(), DeleteUserError> {
    let smart_db = state.get_smart_db();
    let active: bool = smart_db
        .fetch_scalar(&IsUserActiveQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("Error: {e}");
            DeleteUserError::DatabaseError
        })?;
    if !active {
        return Err(DeleteUserError::UnknownUser);
    }

    // Without Keycloak, or for an account unknown to Keycloak: Core alone.
    let Some(admin) = admin else {
        return archive_in_core(&state, user_id).await;
    };
    let Some(user) = export_user(smart_db, user_id as i32)
        .await
        .map_err(DeleteUserError::Keycloak)?
    else {
        return archive_in_core(&state, user_id).await;
    };
    let Some(keycloak_id) = find_account(smart_db, admin, &user)
        .await
        .map_err(DeleteUserError::Keycloak)?
    else {
        return archive_in_core(&state, user_id).await;
    };

    disable_account(admin, &keycloak_id)
        .await
        .map_err(DeleteUserError::Keycloak)?;
    if let Err(error) = archive_in_core(&state, user_id).await {
        if let Err(restore) = enable_account(admin, &keycloak_id).await {
            eprintln!(
                "Keycloak sync: Core refused to archive user {user_id} and the Keycloak account {keycloak_id} could not be re-enabled: {restore}"
            );
        }
        return Err(error);
    }
    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    summary = "Delete a user (administration)",
    description = "Archives a user account: it is flagged `archived` and leaves the active users \
                   (directory, admin listing), but is never hard-deleted. Administrators only.\n\n\
                   Refused with `409` while the user still owns groups, events or projects: \
                   transfer them first. An unknown or already archived user answers `404`.\n\n\
                   **Keycloak (MAIR-142).** When Core runs with a confidential Keycloak client and \
                   the account is known to Keycloak (recorded link, then e-mail), its Keycloak \
                   account is **disabled** (never deleted) and its Keycloak sessions are ended \
                   **before** Core archives it. If Core then refuses, the Keycloak account is \
                   re-enabled. An account unknown to Keycloak only goes through Core, as does any \
                   call without a confidential client.",
    params(
        ("userId" = u64, Path, description = "User id.", example = 42)
    ),
    responses(
        (
            status = 204,
            description = "Account archived. Empty body.",
        ),
        (
            status = 400,
            description = "`userId` in the path is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("can not parse \"abc\" to a u64")
        ),
        (
            status = 401,
            description = "`Authorization` header missing, invalid or expired JWT, or revoked session.",
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
            status = 404,
            description = "No active user matches `userId` (unknown or already archived).",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 409,
            description = "The user still owns groups, events or projects.",
            body = String,
            content_type = "text/plain",
            example = json!("The user still owns groups, events or projects: transfer them first")
        ),
        (
            status = 500,
            description = "Database error while checking or archiving the user, or the Keycloak account found by e-mail is already linked to another Core account. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
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
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn admin_delete_user(
    state: web::Data<AppState>,
    admin: Option<web::Data<KeycloakAdminClient>>,
    path: web::Path<u64>,
) -> Result<impl Responder, DeleteUserError> {
    delete_user(
        state,
        admin.as_ref().map(web::Data::get_ref),
        path.into_inner(),
    )
    .await?;

    Ok(HttpResponse::NoContent())
}
