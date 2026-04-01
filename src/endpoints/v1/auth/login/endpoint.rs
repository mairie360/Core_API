use crate::database::queries::auth::login::{login_query, LoginUserQueryView};
use crate::database::queries::sessions::create_session::CreateSessionQueryView;
use crate::endpoints::v1::auth::create_new_session;
use actix_web::{
    dev::ConnectionInfo, http::StatusCode, post, web, HttpResponse, Responder, ResponseError,
};
use base64::{engine::general_purpose, Engine as _};
use mairie360_api_lib::pool::AppState;
use rand::{thread_rng, RngCore};

use super::login_response_view::LoginResponseView;
use super::login_view::LoginView;
use mairie360_api_lib::jwt_manager::generate_jwt;

#[derive(Debug, Clone, PartialEq)]
enum LoginError {
    InvalidCredentials,
    DatabaseError,
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
        }
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            LoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            LoginError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::TokenGenerationError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn generate_refresh_token() -> String {
    // 32 octets (256 bits) est un standard de sécurité solide
    let mut buffer = [0u8; 32];

    // Remplissage avec des données aléatoires sécurisées
    thread_rng().fill_bytes(&mut buffer);

    // Encodage en Base64 pour avoir une String lisible
    general_purpose::URL_SAFE_NO_PAD.encode(buffer)
}

async fn login_user(
    login_view: &LoginView,
    state: web::Data<AppState>,
    ip_adress: std::net::IpAddr,
) -> Result<(String, String), LoginError> {
    let view = LoginUserQueryView::new(login_view.email(), login_view.password());

    let user_record = login_query(view, state.db_pool.clone())
        .await
        .map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            LoginError::DatabaseError
        })?;

    match user_record {
        Some(user) if login_view.password() == user.password().trim() => {
            let refresh_token = generate_refresh_token();
            let view = CreateSessionQueryView::new(
                user.user_id() as u64,
                &refresh_token,
                &login_view.device_info(),
                ip_adress,
            );
            create_new_session(state.clone(), user.user_id() as u64, view).await;
            let jwt = generate_jwt(user.user_id().to_string().as_str()).map_err(|e| {
                eprintln!("JWT Generation Error: {}", e);
                LoginError::TokenGenerationError
            })?;
            Ok((jwt, refresh_token))
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
        (status = 500, description = "Internal server error")
    ),
    tag = "Authentication"
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
    println!("ip_address: {}", ip_address);

    let (jwt, refresh_token) = login_user(&login_view, state, ip_address).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {}", jwt)))
        .json(LoginResponseView::from(refresh_token)))
}
