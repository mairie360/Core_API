use crate::database::admin::reset_password::{
    AdminResetPasswordQueryView, AdminResetPasswordResult,
};
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::password_identity::{self, IdentityChanges, IdentityCheckError};
use crate::endpoints::v1::admin::users::id::password::view::{
    AdminResetPasswordView, MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH,
};
use crate::endpoints::validation::ValidatedJson;
use crate::passwords;
use crate::session_revocation::publish_revoked_sessions;
use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    InvalidPassword,
    /// The password breaks the identity rule (MAIR-757); the text names it.
    Rejected(String),
    UnknownUser,
    DatabaseError,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPassword => write!(
                f,
                "The password must contain between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH} characters"
            ),
            Self::Rejected(reason) => f.write_str(reason),
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidPassword | Self::Rejected(_) => StatusCode::BAD_REQUEST,
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn reset_password(
    state: web::Data<AppState>,
    user_id: u64,
    view: AdminResetPasswordView,
) -> Result<(), ResetPasswordError> {
    let length = view.new_password().chars().count();
    if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH).contains(&length) {
        return Err(ResetPasswordError::InvalidPassword);
    }
    password_identity::check(
        state.get_smart_db(),
        user_id,
        "new_password",
        view.new_password(),
        IdentityChanges::default(),
    )
    .await
    .map_err(|e| match e {
        IdentityCheckError::Rejected(e) => ResetPasswordError::Rejected(e.to_string()),
        IdentityCheckError::UnknownUser => ResetPasswordError::UnknownUser,
        IdentityCheckError::Database => ResetPasswordError::DatabaseError,
    })?;

    let hashed_password = passwords::hash(view.new_password())
        .await
        .map_err(|error| {
            tracing::error!("{error:?}");
            ResetPasswordError::DatabaseError
        })?;

    let result: AdminResetPasswordResult = state
        .get_smart_db()
        .fetch_one(&AdminResetPasswordQueryView::new(user_id, &hashed_password))
        .await
        .map_err(|error| {
            tracing::error!("{error:?}");
            ResetPasswordError::DatabaseError
        })?;

    if result.updated() {
        // The reset revoked every session of the user: other APIs refuse their JWTs too.
        publish_revoked_sessions(state.get_redis(), result.revoked_sessions()).await;
        Ok(())
    } else {
        Err(ResetPasswordError::UnknownUser)
    }
}

#[utoipa::path(
    patch,
    path = "password",
    summary = "Reset the password of a user (administration)",
    description = "Gives an account a new password, without going through the reset \
                   e-mail. Used to unblock a user who no longer has access to their \
                   mailbox. Reserved to administrators.\n\n\
                   The password must hold 12 to 255 characters, counted in \
                   characters, not in bytes, must not be a commonly used password and must not \
                   contain the account's e-mail, first name or last name (MAIR-757). The response has an empty body.",
    params(
        ("userId" = u64, Path, description = "Id of the user.", example = 42)
    ),
    request_body(
        content = AdminResetPasswordView,
        description = "New password, 12 to 255 characters, not a commonly used password, not containing the account's e-mail, first name or last name.",
        example = json!({ "new_password": "NouveauMotDePasse!123" })
    ),
    responses(
        (
            status = 204,
            description = "Password reset. Empty body.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, or `new_password` shorter than 12 characters, longer than 255 characters, containing a control character, a commonly used password or containing the account's e-mail, first name or last name.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `new_password`: must be at least 12 characters")
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
            description = "No user has this id.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 500,
            description = "Database error while saving the password.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/password")]
pub async fn admin_reset_user_password(
    _: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: ValidatedJson<AdminResetPasswordView>,
) -> Result<impl Responder, ResetPasswordError> {
    reset_password(state, path.into_inner(), view.into_inner()).await?;

    Ok(HttpResponse::NoContent().finish())
}
