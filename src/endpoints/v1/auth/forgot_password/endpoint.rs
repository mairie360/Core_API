use crate::database::auth::is_first_time::IsFirstTimeQueryView;
use crate::database::get_user_id::GetUserIdQueryView;
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
            Self::AlreadyRequested => {
                write!(f, "Password reset already requested.")
            }
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::MailError => {
                write!(f, "An error occurred while sending the email.")
            }
            Self::RedisError => {
                write!(f, "An error occurred while accessing Redis.")
            }
            Self::UserFirstTimeError => {
                write!(f, "User not valid.")
            }
            Self::UserNotFound => {
                write!(f, "User not found.")
            }
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::AlreadyRequested => StatusCode::CONFLICT,
            Self::DatabaseError | Self::MailError | Self::RedisError => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::UserFirstTimeError => StatusCode::UNAUTHORIZED,
            Self::UserNotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn check_user(smart_db: &SmartDatabase, email: &str) -> Result<(), ResetPasswordError> {
    println!("email: {email}");
    let view = DoesUserExistByEmailQueryView::new(email.to_string());
    let result: Result<bool, _> = smart_db.fetch_scalar(&view).await;
    match result {
        Ok(true) => {}
        _ => return Err(ResetPasswordError::UserNotFound),
    }

    let view = GetUserIdQueryView::new(email);
    let Ok(user_id) = smart_db.fetch_scalar::<i32, _>(&view).await else {
        return Err(ResetPasswordError::DatabaseError);
    };

    let view = IsFirstTimeQueryView::new(user_id as u64);
    let result = smart_db.fetch_scalar(&view).await.unwrap();
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
                eprintln!("Email Sender Error: {e}");
                return Err(ResetPasswordError::MailError);
            }
        },
        to: dest.to_string(),
    };

    // Préparation du contenu du mail
    let subject = "Réinitialisation de votre mot de passe";
    let body = format!("Bonjour, voici votre jeton de réinitialisation : {temporary_token}");

    // Étape 2 : On construit le mail avec les bonnes infos
    let email = match build_email(&destination, subject, &body) {
        Ok(email) => email,
        Err(e) => {
            eprintln!("Email Build Error: {e}");
            return Err(ResetPasswordError::MailError);
        }
    };

    // Étape 3 : On l'envoie via le serveur SMTP
    match send_email(email).await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Mail Error: {e}");
            Err(ResetPasswordError::MailError)
        }
    }
}

async fn trigger(state: &AppState, email: &str) -> Result<(), ResetPasswordError> {
    let token = Uuid::new_v4().to_string();
    let redis = state.get_redis();
    redis
        .secure_set(&format!("{email}/forgot_password_token"), &token)
        .await
        .map_err(|e| {
            eprintln!("Redis Error: {e}");
            ResetPasswordError::RedisError
        })?;
    redis
        .secure_set(
            &format!("{token}/forgot_password_email"),
            &email.to_string(),
        )
        .await
        .map_err(|e| {
            eprintln!("Redis Error: {e}");
            ResetPasswordError::RedisError
        })?;
    match handle_forgot_password(token, email).await {
        Ok(()) => Ok(()),
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
    path = "",
    summary = "Demander la réinitialisation d'un mot de passe",
    description = "Génère un jeton de réinitialisation à usage unique, le stocke dans Redis et \
                   l'envoie par e-mail à l'utilisateur. Le jeton est ensuite à présenter à \
                   `POST /api/v1/auth/reset_password`.\n\n\
                   Route publique : le `JwtMiddleware` laisse passer tout ce qui est sous `/auth`.\n\n\
                   Le jeton n'est **jamais** renvoyé dans la réponse : un `200` signifie seulement \
                   que l'e-mail a été remis au serveur SMTP. Une demande déjà en cours pour cette \
                   adresse est refusée en `409` tant que le jeton précédent n'a pas été consommé.",
    request_body(
        content = ForgotPasswordView,
        description = "Adresse e-mail du compte à réinitialiser.",
        example = json!({ "email": "jean.dupont@mairie360.fr" })
    ),
    responses(
        (
            status = 200,
            description = "E-mail de réinitialisation envoyé. Corps vide.",
        ),
        (
            status = 400,
            description = "Corps JSON malformé ou champ `email` absent.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `email`")
        ),
        (
            status = 401,
            description = "Le compte n'est pas éligible à la réinitialisation dans son état actuel.",
            body = String,
            content_type = "text/plain",
            example = json!("User not valid.")
        ),
        (
            status = 404,
            description = "Aucun compte ne correspond à cette adresse e-mail.",
            body = String,
            content_type = "text/plain",
            example = json!("User not found.")
        ),
        (
            status = 409,
            description = "Une demande de réinitialisation est déjà en cours pour cette adresse.",
            body = String,
            content_type = "text/plain",
            example = json!("Password reset already requested.")
        ),
        (
            status = 500,
            description = "Erreur de base de données, de Redis, ou échec de l'envoi de l'e-mail.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while sending the email.")
        )
    ),
    tag = "Auth"
)]
#[post("/forgot_password")]
pub async fn forgot_password(
    state: web::Data<AppState>,
    body: web::Json<ForgotPasswordView>,
) -> Result<impl Responder, ResetPasswordError> {
    forgot_password_trigger(state, body.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
