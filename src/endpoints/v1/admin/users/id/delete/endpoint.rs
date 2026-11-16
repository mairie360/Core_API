use crate::database::ids::id_to_sql;
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::last_admin::{self, LAST_ADMIN_MESSAGE};
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
    LastAdmin,
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
            Self::LastAdmin => write!(f, "{LAST_ADMIN_MESSAGE}"),
            Self::DatabaseError => write!(f, "Database error occurred"),
            Self::Keycloak(error) => write!(f, "{error}"),
        }
    }
}

impl ResponseError for DeleteUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::OwnsResources | Self::LastAdmin | Self::Keycloak(SyncError::EmailTaken) => {
                StatusCode::CONFLICT
            }
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
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

fn is_restrict_violation(error: &ApiLibError) -> bool {
    matches!(
        error,
        ApiLibError::Database(DbError::Sqlx(sqlx::Error::Database(db_error)))
            if db_error.code().as_deref() == Some(RESTRICT_VIOLATION)
    )
}

/// Archives the account in one transaction (MAIR-420): the user's row is locked first, so no
/// session can be opened between reading the active sessions and the archiving, which drops
/// them. The sessions read are published to the revocation list once committed (MAIR-264).
///
/// The `Admin` role is locked before the user (MAIR-678), like every write guarded against
/// leaving no active administrator, and the archiving is rolled back with
/// [`DeleteUserError::LastAdmin`] when it archived the last one.
async fn archive_in_core(state: &AppState, user_id: u64) -> Result<(), DeleteUserError> {
    let database_error = |e: ApiLibError| {
        if is_restrict_violation(&e) {
            DeleteUserError::OwnsResources
        } else {
            tracing::error!("Admin delete user {user_id} DB Error: {e}");
            DeleteUserError::DatabaseError
        }
    };
    let mut tx = state.get_smart_db().begin().await.map_err(database_error)?;
    last_admin::lock(&mut tx).await.map_err(database_error)?;
    let active: bool = tx
        .fetch_scalar(&IsUserActiveQueryView::locked(user_id))
        .await
        .map_err(database_error)?;
    if !active {
        return Err(DeleteUserError::UnknownUser);
    }
    // A failure here is reported rather than ignored: archiving without publishing the sessions
    // would leave their JWTs valid in the other APIs (MAIR-390).
    let sessions: Vec<Uuid> = tx
        .fetch_all(&GetActiveSessionIdsQueryView::new(user_id))
        .await
        .map_err(database_error)?;
    tx.execute(&DeleteUserQueryView::new(user_id))
        .await
        .map_err(database_error)?;
    if !last_admin::any_left(&mut tx)
        .await
        .map_err(database_error)?
    {
        return Err(DeleteUserError::LastAdmin);
    }
    tx.commit().await.map_err(database_error)?;

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
            tracing::error!("Error: {e}");
            DeleteUserError::DatabaseError
        })?;
    if !active {
        return Err(DeleteUserError::UnknownUser);
    }

    // Without Keycloak, or for an account unknown to Keycloak: Core alone.
    let Some(admin) = admin else {
        return archive_in_core(&state, user_id).await;
    };
    let Some(user) = export_user(smart_db, id_to_sql(user_id))
        .await
        .map_err(DeleteUserError::Keycloak)?
    else {
        return archive_in_core(&state, user_id).await;
    };
    // Refused before the Keycloak account is disabled (which ends its Keycloak sessions); the
    // transaction of the archiving checks again.
    if last_admin::is_last(smart_db, user_id).await.map_err(|e| {
        tracing::error!("Admin delete user {user_id} DB Error: {e}");
        DeleteUserError::DatabaseError
    })? {
        return Err(DeleteUserError::LastAdmin);
    }
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
            tracing::error!(
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
                   call without a confidential client.\n\n\
                   **Last administrator (MAIR-678).** An instance keeps at least one active \
                   administrator (role `Admin`, account not archived): archiving the only one \
                   answers `409` and changes nothing (its Keycloak account is not disabled). The \
                   check and the archiving run in one transaction locking the `Admin` role, so \
                   two concurrent archivings of the two last administrators cannot both succeed.",
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
            example = json!("Invalid path: ids are integers.")
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
            description = "The user still owns groups, events or projects, or the user is the last active administrator (MAIR-678). Nothing is changed.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Owns resources" = (value = json!("The user still owns groups, events or projects: transfer them first"))),
                ("Last administrator" = (value = json!("This user is the last active administrator: give the Admin role to another active user first.")))
            )
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
    _: AdminUser,
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
