use crate::database::admin::reset_password::AdminResetPasswordQueryView;
use crate::endpoints::v1::admin::users::id::password::view::{
    AdminResetPasswordView, MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH,
};
use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    InvalidPassword,
    UnknownUser,
    DatabaseError,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResetPasswordError::InvalidPassword => write!(
                f,
                "The password must contain between {} and {} characters",
                MIN_PASSWORD_LENGTH, MAX_PASSWORD_LENGTH
            ),
            ResetPasswordError::UnknownUser => write!(f, "Unknown user"),
            ResetPasswordError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            ResetPasswordError::InvalidPassword => StatusCode::BAD_REQUEST,
            ResetPasswordError::UnknownUser => StatusCode::NOT_FOUND,
            ResetPasswordError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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

    let updated: bool = state
        .get_smart_db()
        .fetch_scalar(&AdminResetPasswordQueryView::new(
            user_id,
            view.new_password(),
        ))
        .await
        .map_err(|error| {
            eprintln!("{:?}", error);
            ResetPasswordError::DatabaseError
        })?;

    if updated {
        Ok(())
    } else {
        Err(ResetPasswordError::UnknownUser)
    }
}

#[utoipa::path(
    patch,
    path = "password",
    params(
        ("userId" = u64, Path, description = "User ID")
    ),
    request_body = AdminResetPasswordView,
    responses(
        (status = 204, description = "Password reset and active sessions revoked"),
        (status = 400, description = "Invalid password"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Unknown user"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/password")]
pub async fn admin_reset_user_password(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: web::Json<AdminResetPasswordView>,
) -> Result<impl Responder, ResetPasswordError> {
    reset_password(state, path.into_inner(), view.into_inner()).await?;

    Ok(HttpResponse::NoContent().finish())
}
