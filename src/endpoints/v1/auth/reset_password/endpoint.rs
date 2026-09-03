use crate::database::auth::change_password::ChangePasswordQueryView;
use crate::database::get_user_id::GetUserIdQueryView;
use crate::endpoints::v1::auth::login::endpoint::generate_session;
use crate::endpoints::v1::auth::reset_password::view::{
    ResetPasswordResponseView, ResetPasswordView,
};
use actix_web::dev::ConnectionInfo;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    DatabaseError,
    RedisError,
    TokenGenerationError,
    UnknownToken,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResetPasswordError::DatabaseError => {
                write!(f, "Internal server error")
            }
            ResetPasswordError::RedisError => {
                write!(f, "Internal server error")
            }
            ResetPasswordError::TokenGenerationError => {
                write!(f, "Internal server error")
            }
            ResetPasswordError::UnknownToken => {
                write!(f, "Unknown token")
            }
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            ResetPasswordError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::RedisError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::TokenGenerationError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::UnknownToken => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_id(smart_db: &SmartDatabase, email: &str) -> Result<u64, ResetPasswordError> {
    let view = GetUserIdQueryView::new(email);
    match smart_db.fetch_scalar::<i32, _>(&view).await {
        Ok(user_id) => Ok(user_id as u64),
        Err(_) => Err(ResetPasswordError::DatabaseError),
    }
}

async fn reset_pwd(
    smart_db: &SmartDatabase,
    new_password: &str,
    user_id: u64,
) -> Result<(), ResetPasswordError> {
    let view = ChangePasswordQueryView::new(new_password, user_id);
    smart_db
        .execute(view)
        .await
        .map_err(|_| ResetPasswordError::DatabaseError)?;
    Ok(())
}

async fn reset_password_trigger(
    state: web::Data<AppState>,
    view: ResetPasswordView,
    ip_adress: std::net::IpAddr,
) -> Result<(String, String), ResetPasswordError> {
    let smart_db = state.get_smart_db();
    let redis = state.get_redis();

    let key = format!("{}/forgot_password_email", view.token());
    let email: String = match redis.secure_get::<String>(&key).await {
        Ok(Some(email)) => email,
        other => {
            eprintln!("Failed to get email from Redis: {:?}", other);
            return Err(ResetPasswordError::UnknownToken);
        }
    };
    let user_id = match get_user_id(smart_db, &email).await {
        Ok(user_id) => user_id,
        Err(e) => {
            eprintln!("Failed to get user ID: {:?}", e);
            return Err(e);
        }
    };

    let reversed_key = format!("{}/forgot_password_token", email);
    if let Err(e) = redis.delete(&reversed_key).await {
        eprintln!("Failed to delete reversed key: {:?}", e);
        return Err(ResetPasswordError::RedisError);
    }
    if let Err(e) = redis.delete(&key).await {
        eprintln!("Failed to delete key: {:?}", e);
        return Err(ResetPasswordError::RedisError);
    }

    reset_pwd(smart_db, view.new_password(), user_id).await?;

    match generate_session(user_id, &view.device_info(), ip_adress, state).await {
        Ok((jwt, refresh_token)) => Ok((jwt, refresh_token)),
        Err(e) => {
            eprintln!("Failed to generate session: {:?}", e);
            Err(ResetPasswordError::TokenGenerationError)
        }
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
    let ip_address = ip_str
        .parse::<std::net::IpAddr>()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));
    let (jwt, refresh_token) = reset_password_trigger(state, body.into_inner(), ip_address).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {}", jwt)))
        .json(ResetPasswordResponseView::from(refresh_token)))
}
