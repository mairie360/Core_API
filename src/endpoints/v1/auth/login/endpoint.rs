use super::view::{LoginResponseView, LoginView};
use crate::database::auth::login::{login_query, LoginUserQueryView};
use crate::database::sessions::create_session::CreateSessionQueryView;
use crate::endpoints::v1::auth::create_new_session;
use crate::endpoints::v1::auth::login::view::LoginFirstConnectionResponseView;
use actix_web::{
    dev::ConnectionInfo, http::StatusCode, post, web, HttpResponse, Responder, ResponseError,
};
use base64::{engine::general_purpose, Engine as _};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::pool::redis::simple_key::secured::{handle_secure_get, handle_secure_post};
use mairie360_api_lib::pool::AppState;
use rand::fill;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum LoginError {
    DatabaseError,
    FirstConnectError(String),
    InvalidCredentials,
    RedisError,
    TokenGenerationError,
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginError::InvalidCredentials => write!(f, "Invalid credentials provided."),
            LoginError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            LoginError::TokenGenerationError => write!(f, "Failed to generate JWT token."),
            LoginError::FirstConnectError(token) => {
                write!(f, "{}", token.to_string())
            }
            LoginError::RedisError => write!(f, "Internal Redis error."),
        }
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            LoginError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::FirstConnectError(_) => StatusCode::PRECONDITION_FAILED,
            LoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            LoginError::RedisError => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::TokenGenerationError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if self.status_code() == StatusCode::PRECONDITION_FAILED {
            return HttpResponse::build(self.status_code())
                .json(LoginFirstConnectionResponseView::new(self.to_string()));
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn generate_refresh_token() -> String {
    // 32 octets (256 bits) est un standard de sécurité solide
    let mut buffer = [0u8; 32];

    // Remplissage avec des données aléatoires sécurisées
    fill(&mut buffer);

    // Encodage en Base64 pour avoir une String lisible
    general_purpose::URL_SAFE_NO_PAD.encode(buffer)
}

pub async fn generate_session(
    user_id: u64,
    device_info: &str,
    ip_adress: std::net::IpAddr,
    state: web::Data<AppState>,
) -> Result<(String, String), LoginError> {
    let refresh_token = generate_refresh_token();
    let view = CreateSessionQueryView::new(user_id, &refresh_token, device_info, ip_adress);
    create_new_session(state, user_id, view).await;
    let jwt = generate_jwt(user_id.to_string().as_str()).map_err(|e| {
        eprintln!("JWT Generation Error: {}", e);
        LoginError::TokenGenerationError
    })?;
    Ok((jwt, refresh_token))
}

async fn generate_first_connection_token(
    user_id: u64,
    state: web::Data<AppState>,
) -> Result<String, LoginError> {
    match handle_secure_get(
        state.get_redis_conn().await.unwrap(),
        &format!("{}/first_connection_token", user_id),
    )
    .await
    {
        Ok(token) => return Ok(token),
        _ => {}
    };
    let token = Uuid::new_v4().to_string();
    println!("{}", format!("{}/first_connection_id", token));
    println!("{}", &format!("{}/first_connection_token", user_id));
    handle_secure_post(
        state.get_redis_conn().await.unwrap(),
        &format!("{}/first_connection_token", user_id),
        &token,
    )
    .await
    .map_err(|e| {
        eprintln!("Redis Error: {}", e);
        LoginError::RedisError
    })?;
    handle_secure_post(
        state.get_redis_conn().await.unwrap(),
        &format!("{}/first_connection_id", token),
        &format!("{}", user_id),
    )
    .await
    .map_err(|e| {
        eprintln!("Redis Error: {}", e);
        LoginError::RedisError
    })?;
    Ok(token)
}

async fn login_user(
    login_view: &LoginView,
    state: web::Data<AppState>,
    ip_adress: std::net::IpAddr,
) -> Result<(String, String), LoginError> {
    let view = LoginUserQueryView::new(login_view.email(), login_view.password());

    let user_record = login_query(view, state.db_pool.clone().unwrap())
        .await
        .map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            LoginError::DatabaseError
        })?;

    match user_record {
        Some(user) if user.first_connect() => Err(LoginError::FirstConnectError(
            generate_first_connection_token(user.user_id() as u64, state).await?,
        )),
        Some(user) if login_view.password() == user.password().trim() => {
            generate_session(
                user.user_id() as u64,
                &login_view.device_info(),
                ip_adress,
                state,
            )
            .await
        }
        _ => {
            eprintln!(
                "Login failed: Invalid credentials for {}",
                login_view.email()
            );
            Err(LoginError::InvalidCredentials)
        }
    }
}

#[utoipa::path(
    post,
    path = "",
    request_body = LoginView,
    responses(
        (status = 200, description = "User login successfully!", body = LoginResponseView),
        (status = 401, description = "Invalid credentials provided."),
        (status = 412, description = "User needs to change password because first login", body = LoginFirstConnectionResponseView),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
#[post("/login")]
pub async fn login(
    payload: web::Json<LoginView>,
    state: web::Data<AppState>,
    conn: ConnectionInfo,
) -> Result<impl Responder, LoginError> {
    let login_view = payload.into_inner();
    let ip_str = conn.realip_remote_addr().unwrap_or("unknown").to_string();
    let ip_address = std::net::IpAddr::from(
        ip_str
            .parse::<std::net::IpAddr>()
            .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
    );

    let (jwt, refresh_token) = login_user(&login_view, state, ip_address).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {}", jwt)))
        .json(LoginResponseView::from(refresh_token)))
}
