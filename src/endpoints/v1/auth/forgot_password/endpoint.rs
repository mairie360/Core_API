use crate::database::roles::get_roles::{get_roles_query, GetRolesQueryView};
use crate::endpoints::v1::auth::forgot_password::view::ForgotPasswordView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    DatabaseError,
    UserNotFound,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResetPasswordError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            ResetPasswordError::UserNotFound => {
                write!(f, "User not found.")
            }
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            ResetPasswordError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn check_user(pool: &sqlx::Pool<sqlx::Postgres>, email: &str) -> bool {
    todo!()
}

async fn trigger(pool: &sqlx::Pool<sqlx::Postgres>, email: &str) -> Result<(), ResetPasswordError> {
    todo!()
}

async fn forgot_password_trigger(
    state: web::Data<AppState>,
    view: ForgotPasswordView,
) -> Result<(), ResetPasswordError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(ResetPasswordError::DatabaseError),
    };

    if !check_user(&pool, view.email()).await {
        return Err(ResetPasswordError::UserNotFound);
    }

    trigger(&pool, view.email()).await?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Forgot password request sent successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth",
    security(
        ("jwt" = [])
    )
)]
#[get("/forgot_password")]
pub async fn forgot_password(
    state: web::Data<AppState>,
    body: web::Json<ForgotPasswordView>,
) -> Result<impl Responder, ResetPasswordError> {
    forgot_password_trigger(state, body.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
