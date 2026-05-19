use crate::endpoints::v1::auth::login::endpoint::generate_session;
use crate::endpoints::v1::auth::reset_password::view::{
    ResetPasswordResponseView, ResetPasswordView,
};
use actix_web::dev::ConnectionInfo;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    DatabaseError,
    TokenGenerationError,
    Unauthorized,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResetPasswordError::DatabaseError => {
                write!(f, "Internal server error")
            }
            ResetPasswordError::TokenGenerationError => {
                write!(f, "Internal server error")
            }
            ResetPasswordError::Unauthorized => {
                write!(f, "Unauthorized, invalid token.")
            }
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            ResetPasswordError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::TokenGenerationError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_id(pool: &sqlx::pool::Pool<sqlx::Postgres>, token: &str) -> Option<u64> {
    todo!();
    Some(0)
}

async fn reset_pwd(
    pool: &sqlx::pool::Pool<sqlx::Postgres>,
    new_password: &str,
) -> Result<(), sqlx::Error> {
    todo!();
    Ok(())
}

async fn reset_password_trigger(
    state: web::Data<AppState>,
    view: ResetPasswordView,
    ip_adress: std::net::IpAddr,
) -> Result<(String, String), ResetPasswordError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(ResetPasswordError::DatabaseError),
    };

    let user_id = match get_user_id(&pool, view.token()).await {
        Some(user_id) => user_id,
        None => return Err(ResetPasswordError::Unauthorized),
    };

    reset_pwd(&pool, view.new_password())
        .await
        .map_err(|e| ResetPasswordError::DatabaseError)?;

    match generate_session(user_id, &view.device_info(), ip_adress, state).await {
        Ok((jwt, refresh_token)) => Ok((jwt, refresh_token)),
        _ => Err(ResetPasswordError::TokenGenerationError),
    }
}

#[utoipa::path(
    post,
    path = "/",
    responses(
        (status = 200, description = "Password reset successfully", body = ResetPasswordResponseView),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized, invalid token"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth",
)]
#[post("/reset_password")]
pub async fn reset_password(
    state: web::Data<AppState>,
    body: web::Json<ResetPasswordView>,
    conn: ConnectionInfo,
) -> Result<impl Responder, ResetPasswordError> {
    let ip_str = conn.realip_remote_addr().unwrap_or("unknown").to_string();
    let ip_address = std::net::IpAddr::from(
        ip_str
            .parse::<std::net::IpAddr>()
            .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
    );
    let (jwt, refresh_token) = reset_password_trigger(state, body.into_inner(), ip_address).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {}", jwt)))
        .json(ResetPasswordResponseView::from(refresh_token)))
}
