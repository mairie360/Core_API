//! Suspension and reactivation of an account by an administrator (MAIR-904).
//!
//! Suspending sets `users.status = 'suspended'` and revokes every session of the account in one
//! transaction (MAIR-420), then publishes them to the revocation list: the agent loses access to
//! every API at once. While suspended, the sign-in (password, passkey, password reset) answers
//! like an unknown account, and `is_admin()` / `check_access()` deny the account. Its roles,
//! groups and second factor are kept: reactivating gives everything back (status `offline`).

use crate::database::admin::suspension::{ReactivateUserQueryView, SuspendUserQueryView};
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::last_admin::{self, LAST_ADMIN_MESSAGE};
use crate::session_revocation::{publish_revoked_sessions, revoke_all_user_sessions_in};
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

/// Body of the `409` answered to an administrator suspending their own account.
pub const OWN_ACCOUNT_MESSAGE: &str = "You cannot suspend your own account.";

#[derive(Debug, Clone, PartialEq, Eq)]
enum SuspensionError {
    UnknownUser,
    OwnAccount,
    LastAdmin,
    DatabaseError,
}

impl std::fmt::Display for SuspensionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::OwnAccount => write!(f, "{OWN_ACCOUNT_MESSAGE}"),
            Self::LastAdmin => write!(f, "{LAST_ADMIN_MESSAGE}"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for SuspensionError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::OwnAccount | Self::LastAdmin => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

fn database_error(context: &str, e: &ApiLibError) -> SuspensionError {
    if matches!(e, ApiLibError::Database(DbError::NotFound)) {
        return SuspensionError::UnknownUser;
    }
    tracing::error!("{context}: {e}");
    SuspensionError::DatabaseError
}

#[utoipa::path(
    post,
    path = "suspend",
    summary = "Suspend a user (administration)",
    description = "Suspends the account (MAIR-904): its status becomes `suspended` and **every \
                   session of the user is closed**, here and in every API. Until it is \
                   reactivated, the agent cannot sign in (password, passkey or password reset \
                   answer like an unknown account) and is denied everything, administration \
                   included. Nothing else changes: roles, groups and second factor are kept for \
                   the reactivation. Unlike `DELETE /api/v1/admin/users/{userId}/` (archiving), it \
                   is reversible with `POST /api/v1/admin/users/{userId}/reactivate`.\n\n\
                   Idempotent: an account already suspended answers `204`. An administrator cannot \
                   suspend their own account, nor the last active administrator (MAIR-678): both \
                   answer `409` and change nothing. Reserved to administrators; each call is logged.",
    params(
        ("userId" = u64, Path, description = "Id of the user.", example = 42)
    ),
    responses(
        (status = 204, description = "Account suspended, its sessions closed. Empty body."),
        (
            status = 400,
            description = "`userId` is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid path: ids are integers.")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
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
            description = "No active user has this id (unknown or archived).",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 409,
            description = "The administrator targets their own account, or the user is the last active administrator. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Own account" = (value = json!("You cannot suspend your own account."))),
                ("Last administrator" = (value = json!("This user is the last active administrator: give the Admin role to another active user first.")))
            )
        ),
        (
            status = 500,
            description = "Database error. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Users",
    security(("jwt" = []))
)]
#[post("/suspend")]
pub async fn admin_suspend_user(
    admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, SuspensionError> {
    let user_id = path.into_inner();
    if admin.id == user_id {
        return Err(SuspensionError::OwnAccount);
    }
    let context = "Admin suspend user";
    let mut tx = state
        .get_smart_db()
        .begin()
        .await
        .map_err(|e| database_error(context, &e))?;
    // Same order as every write guarded against leaving no administrator (MAIR-678).
    last_admin::lock(&mut tx)
        .await
        .map_err(|e| database_error(context, &e))?;
    let _changed: bool = tx
        .fetch_scalar(&SuspendUserQueryView::new(user_id))
        .await
        .map_err(|e| database_error(context, &e))?;
    let revoked = revoke_all_user_sessions_in(&mut tx, user_id)
        .await
        .map_err(|e| database_error(context, &e))?;
    if !last_admin::any_left(&mut tx)
        .await
        .map_err(|e| database_error(context, &e))?
    {
        return Err(SuspensionError::LastAdmin);
    }
    tx.commit().await.map_err(|e| database_error(context, &e))?;
    // Logged rather than answered: the suspension is committed and Core already refuses the
    // sessions; a later "close every session" call publishes them again (MAIR-584).
    publish_revoked_sessions(state.get_redis(), &revoked).await;
    tracing::warn!(
        admin_id = admin.id,
        user_id,
        revoked_sessions = revoked.len(),
        "AUDIT: account suspended by an administrator"
    );
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    path = "reactivate",
    summary = "Reactivate a suspended user (administration)",
    description = "Ends the suspension of the account (MAIR-904): its status becomes `offline` and \
                   the agent can sign in again, with the roles, groups and second factor it had. \
                   Idempotent: an account that is not suspended answers `204` and is left as is. \
                   Reserved to administrators; each call is logged.",
    params(
        ("userId" = u64, Path, description = "Id of the user.", example = 42)
    ),
    responses(
        (status = 204, description = "Account reactivated (or it was not suspended). Empty body."),
        (
            status = 400,
            description = "`userId` is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid path: ids are integers.")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
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
            description = "No active user has this id (unknown or archived).",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 500,
            description = "Database error. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Users",
    security(("jwt" = []))
)]
#[post("/reactivate")]
pub async fn admin_reactivate_user(
    admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, SuspensionError> {
    let user_id = path.into_inner();
    let changed: bool = state
        .get_smart_db()
        .fetch_scalar(&ReactivateUserQueryView::new(user_id))
        .await
        .map_err(|e| database_error("Admin reactivate user", &e))?;
    if changed {
        tracing::warn!(
            admin_id = admin.id,
            user_id,
            "AUDIT: account reactivated by an administrator"
        );
    }
    Ok(HttpResponse::NoContent().finish())
}
