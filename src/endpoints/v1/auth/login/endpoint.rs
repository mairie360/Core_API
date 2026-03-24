use crate::database::queries::login_query;
use crate::database::query_views::LoginUserQueryView;
use actix_web::{http::StatusCode, post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

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

async fn login_user(
    login_view: &LoginView,
    state: web::Data<AppState>,
) -> Result<String, LoginError> {
    let view = LoginUserQueryView::new(login_view.email(), login_view.password());

    let user_record = login_query(view, state.db_pool.clone())
        .await
        .map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            LoginError::DatabaseError
        })?;

    println!("Login view: {}", login_view);
    println!("User record: {:?}", user_record);
    match user_record {
        Some(user) if login_view.password() == user.password().trim() => {
            generate_jwt(user.user_id().to_string().as_str()).map_err(|e| {
                eprintln!("JWT Generation Error: {}", e);
                LoginError::TokenGenerationError
            })
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
        (status = 200, description = "User login successfully!", body = String),
        (status = 401, description = "Invalid credentials provided."),
        (status = 500, description = "Internal server error")
    ),
    tag = "Authentication"
)]
#[post("/login")]
pub async fn login(
    payload: web::Json<LoginView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, LoginError> {
    let login_view = payload.into_inner();

    let jwt = login_user(&login_view, state).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {}", jwt)))
        .body("User login successfully!"))
}
