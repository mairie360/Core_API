use crate::database::auth::is_first_time::{is_first_time_query, IsFirstTimeQueryView};
use crate::database::get_user_id::{get_user_id_query, GetUserIdQueryView};
use crate::endpoints::v1::auth::forgot_password::view::ForgotPasswordView;
use crate::{build_email, get_email_sender, send_email, EmailDestination};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    AlreadyRequested,
    DatabaseError,
    MailError,
    RedisError,
    UserFirstTimeError,
    UserNotFound,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResetPasswordError::AlreadyRequested => {
                write!(f, "Password reset already requested.")
            }
            ResetPasswordError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            ResetPasswordError::MailError => {
                write!(f, "An error occurred while sending the email.")
            }
            ResetPasswordError::RedisError => {
                write!(f, "An error occurred while accessing Redis.")
            }
            ResetPasswordError::UserFirstTimeError => {
                write!(f, "User not valid.")
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
            ResetPasswordError::AlreadyRequested => StatusCode::CONFLICT,
            ResetPasswordError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::MailError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::RedisError => StatusCode::INTERNAL_SERVER_ERROR,
            ResetPasswordError::UserFirstTimeError => StatusCode::UNAUTHORIZED,
            ResetPasswordError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn check_user(smart_db: &SmartDatabase, email: &str) -> Result<(), ResetPasswordError> {
    println!("email: {}", email);
    let view = DoesUserExistByEmailQueryView::new(email.to_string());
    let result: Result<bool, _> = smart_db.fetch_scalar(&view).await;
    match result {
        Ok(true) => {}
        _ => return Err(ResetPasswordError::UserNotFound),
    };

    let view = GetUserIdQueryView::new(email);
    let user_id = match get_user_id_query(view, smart_db).await {
        Ok(user_id) => user_id,
        Err(_) => return Err(ResetPasswordError::DatabaseError),
    };

    let view = IsFirstTimeQueryView::new(user_id as u64);
    let result = is_first_time_query(view, smart_db).await.unwrap();
    if result {
        Ok(())
    } else {
        Err(ResetPasswordError::UserFirstTimeError)
    }
}

async fn handle_forgot_password(
    temporary_token: String,
    dest: &str,
) -> Result<(), ResetPasswordError> {
    // Étape 1 : On récupère où envoyer le mail (les adresses fixes de la CI)
    let destination = EmailDestination {
        from: match get_email_sender() {
            Ok(sender) => sender,
            Err(e) => {
                eprintln!("Email Sender Error: {}", e);
                return Err(ResetPasswordError::MailError);
            }
        },
        to: dest.to_string(),
    };

    // Préparation du contenu du mail
    let subject = "Réinitialisation de votre mot de passe";
    let body = format!(
        "Bonjour, voici votre jeton de réinitialisation : {}",
        temporary_token
    );

    // Étape 2 : On construit le mail avec les bonnes infos
    let email = match build_email(&destination, subject, &body) {
        Ok(email) => email,
        Err(e) => {
            eprintln!("Email Build Error: {}", e);
            return Err(ResetPasswordError::MailError);
        }
    };

    // Étape 3 : On l'envoie via le serveur SMTP
    match send_email(email).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Mail Error: {}", e);
            Err(ResetPasswordError::MailError)
        }
    }
}

async fn trigger(state: &AppState, email: &str) -> Result<(), ResetPasswordError> {
    let token = Uuid::new_v4().to_string();
    let redis = state.get_redis();
    redis
        .secure_set(&format!("{}/forgot_password_token", email), &token)
        .await
        .map_err(|e| {
            eprintln!("Redis Error: {}", e);
            ResetPasswordError::RedisError
        })?;
    redis
        .secure_set(
            &format!("{}/forgot_password_email", token),
            &email.to_string(),
        )
        .await
        .map_err(|e| {
            eprintln!("Redis Error: {}", e);
            ResetPasswordError::RedisError
        })?;
    match handle_forgot_password(token, email).await {
        Ok(_) => Ok(()),
        Err(_) => Err(ResetPasswordError::MailError),
    }
}

async fn forgot_password_trigger(
    state: web::Data<AppState>,
    view: ForgotPasswordView,
) -> Result<(), ResetPasswordError> {
    let smart_db = state.get_smart_db();

    let token = state
        .get_redis()
        .secure_get::<String>(&format!("{}/forgot_password_token", view.email()))
        .await;
    if matches!(token, Ok(Some(_))) {
        return Err(ResetPasswordError::AlreadyRequested);
    }

    match check_user(smart_db, view.email()).await {
        Err(err) => Err(err),
        _ => trigger(&state, view.email()).await,
    }
}

#[utoipa::path(
    post,
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
#[post("/forgot_password")]
pub async fn forgot_password(
    state: web::Data<AppState>,
    body: web::Json<ForgotPasswordView>,
) -> Result<impl Responder, ResetPasswordError> {
    forgot_password_trigger(state, body.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
