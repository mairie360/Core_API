use crate::database::auth::register::register_query;
use crate::database::auth::register::RegisterUserQueryView;
use crate::endpoints::v1::admin::users::post::view::CreateUserView;
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::queries::does_user_exist_by_email_query;
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::pool::AppState;
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq)]
enum CreateUserError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for CreateUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateUserError::InvalidData => write!(f, "Invalid data provided"),
            CreateUserError::UserAlreadyExists => write!(f, "User already exists"),
            CreateUserError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for CreateUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            CreateUserError::InvalidData => StatusCode::BAD_REQUEST,
            CreateUserError::UserAlreadyExists => StatusCode::CONFLICT,
            CreateUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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
        Some(num) => num.len() >= 10 && num.chars().all(|c| c.is_digit(10)),
        None => true,
    }
}

async fn can_be_registered(
    register_view: &CreateUserView,
    pool: &PgPool,
) -> Result<(), CreateUserError> {
    if !is_valid_email(register_view.email()) {
        return Err(CreateUserError::InvalidData);
    }

    let exists = does_user_exist_by_email_query(
        DoesUserExistByEmailQueryView::new(register_view.email().to_string()),
        pool.clone(),
    )
    .await
    .map_err(|_| CreateUserError::DatabaseError)?;

    if exists {
        return Err(CreateUserError::UserAlreadyExists);
    }

    if !is_valid_password(register_view.password()) {
        return Err(CreateUserError::InvalidData);
    }
    if !is_valid_phone_number(register_view.phone_number()) {
        return Err(CreateUserError::InvalidData);
    }
    Ok(())
}

async fn register_user(
    register_view: &CreateUserView,
    state: web::Data<AppState>,
) -> Result<(), CreateUserError> {
    can_be_registered(register_view, &state.db_pool.clone().unwrap()).await?;

    let view = RegisterUserQueryView::new(
        register_view.first_name(),
        register_view.last_name(),
        register_view.email(),
        register_view.password(),
        register_view.phone_number().map(|s| s),
    );

    let success = register_query(view, state.db_pool.clone().unwrap())
        .await
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            CreateUserError::DatabaseError
        })?;

    if success {
        Ok(())
    } else {
        Err(CreateUserError::DatabaseError)
    }
}

#[utoipa::path(
    post,
    path = "",
    request_body = CreateUserView,
    responses(
        (status = 201, description = "User registered successfully"),
        (status = 400, description = "Invalid data provided"),
        (status = 409, description = "User already exists"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[post("/")]
pub async fn post(
    payload: web::Json<CreateUserView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, CreateUserError> {
    let register_view = payload.into_inner();

    register_user(&register_view, state).await?;

    Ok(HttpResponse::Created().body("User registered successfully!"))
}
