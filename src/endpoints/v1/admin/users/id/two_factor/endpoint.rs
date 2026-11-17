use crate::database::second_factor::{
    AccountEmail, DeleteBackupCodesQueryView, DeleteTotpQueryView, GetAccountEmailQueryView,
};
use crate::endpoints::admin_guard::AdminUser;
use crate::session_revocation::{publish_revoked_sessions, revoke_all_user_sessions_in};
use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResetTwoFactorError {
    UnknownUser,
    DatabaseError,
}

impl std::fmt::Display for ResetTwoFactorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for ResetTwoFactorError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn database_error(e: &ApiLibError) -> ResetTwoFactorError {
    tracing::error!("Reset two-factor: {e}");
    ResetTwoFactorError::DatabaseError
}

#[utoipa::path(
    delete,
    path = "2fa",
    summary = "Reset the two-factor authentication of a user (administration)",
    description = "Removes the TOTP secret and the backup codes of a user who lost their phone, \
                   and **closes every session of that user** (here and in every API). Their next \
                   login asks for the password only, until they enrol again (or must, when the \
                   instance made the second factor mandatory). Idempotent: a user without a second \
                   factor answers `204` too. Reserved to administrators; each call is logged with \
                   the administrator and the user.",
    params(
        ("userId" = u64, Path, description = "Id of the user.", example = 42)
    ),
    responses(
        (status = 204, description = "Second factor removed (or there was none). Sessions closed. Empty body."),
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
            description = "No active user has this id.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 500,
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Users",
    security(("jwt" = []))
)]
#[delete("/2fa")]
pub async fn admin_reset_user_two_factor(
    admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, ResetTwoFactorError> {
    let user_id = path.into_inner();
    let db = state.get_smart_db();
    match db
        .fetch_one::<AccountEmail, _>(&GetAccountEmailQueryView::new(user_id))
        .await
    {
        Ok(_) => {}
        Err(ApiLibError::Database(DbError::NotFound)) => {
            return Err(ResetTwoFactorError::UnknownUser)
        }
        Err(e) => return Err(database_error(&e)),
    }
    let mut tx = db.begin().await.map_err(|e| database_error(&e))?;
    tx.execute(&DeleteBackupCodesQueryView::new(user_id))
        .await
        .map_err(|e| database_error(&e))?;
    tx.execute(&DeleteTotpQueryView::new(user_id))
        .await
        .map_err(|e| database_error(&e))?;
    // The agent may have lost their phone because it was stolen: no session survives the reset.
    let revoked = revoke_all_user_sessions_in(&mut tx, user_id)
        .await
        .map_err(|e| database_error(&e))?;
    tx.commit().await.map_err(|e| database_error(&e))?;
    publish_revoked_sessions(state.get_redis(), &revoked).await;
    tracing::warn!(
        admin_id = admin.id,
        user_id,
        revoked_sessions = revoked.len(),
        "AUDIT: two-factor authentication reset by an administrator"
    );
    Ok(HttpResponse::NoContent().finish())
}
