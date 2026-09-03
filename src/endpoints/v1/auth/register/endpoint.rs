use crate::database::auth::register::RegisterUserQueryView;
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

use super::register_view::RegisterView;

#[derive(Debug, Clone, PartialEq)]
enum RegisterError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterError::InvalidData => write!(f, "Invalid data provided"),
            RegisterError::UserAlreadyExists => write!(f, "User already exists"),
            RegisterError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for RegisterError {
    fn status_code(&self) -> StatusCode {
        match self {
            RegisterError::InvalidData => StatusCode::BAD_REQUEST,
            RegisterError::UserAlreadyExists => StatusCode::CONFLICT,
            RegisterError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn is_valid_email(email: &str) -> bool {
    if email.is_empty() {
        return false;
    }
    match email.find('@') {
        Some(index) => {
            let domain = &email[index + 1..];
            !domain.is_empty() && domain.contains('.')
        }
        None => false,
    }
}

fn is_valid_password(password: &str) -> bool {
    //Need to be more complex and based on requirements
    password.len() >= 8
}

fn is_valid_phone_number(phone_number: Option<&str>) -> bool {
    //Need to be more complex and based on requirements
    match phone_number {
        Some(num) => num.len() >= 10 && num.chars().all(|c| c.is_ascii_digit()),
        None => true,
    }
}

async fn can_be_registered(
    register_view: &RegisterView,
    smart_db: &SmartDatabase,
) -> Result<(), RegisterError> {
    if !is_valid_email(register_view.email()) {
        return Err(RegisterError::InvalidData);
    }

    let exists: bool = smart_db
        .fetch_scalar(&DoesUserExistByEmailQueryView::new(
            register_view.email().to_string(),
        ))
        .await
        .map_err(|_| RegisterError::DatabaseError)?;

    if exists {
        return Err(RegisterError::UserAlreadyExists);
    }

    if !is_valid_password(register_view.password()) {
        return Err(RegisterError::InvalidData);
    }
    if !is_valid_phone_number(register_view.phone_number()) {
        return Err(RegisterError::InvalidData);
    }
    Ok(())
}

async fn register_user(
    register_view: &RegisterView,
    state: web::Data<AppState>,
) -> Result<(), RegisterError> {
    can_be_registered(register_view, state.get_smart_db()).await?;

    let view = RegisterUserQueryView::new(
        register_view.first_name(),
        register_view.last_name(),
        register_view.email(),
        register_view.password(),
        register_view.phone_number(),
    );

    let success: bool = state
        .get_smart_db()
        .fetch_scalar(&view)
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            RegisterError::DatabaseError
        })?;

    if success {
        Ok(())
    } else {
        Err(RegisterError::DatabaseError)
    }
}

#[utoipa::path(
    post,
    path = "",
    request_body = RegisterView,
    responses(
        (status = 201, description = "User registered successfully", body = String),
        (status = 400, description = "Invalid data provided", body = String),
        (status = 409, description = "User already exists", body = String),
        (status = 500, description = "Database error occurred", body = String)
    ),
    tag = "Authentication"
)]
#[post("/register")]
pub async fn register(
    payload: web::Json<RegisterView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, RegisterError> {
    let register_view = payload.into_inner();

    register_user(&register_view, state).await?;

    Ok(HttpResponse::Created().body("User registered successfully!"))
}
