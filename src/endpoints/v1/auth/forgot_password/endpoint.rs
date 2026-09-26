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

/// Only infrastructure failures are reported: whether the e-mail matches an account, is already
/// waiting for a reset or is not eligible never changes the response (no account enumeration).
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResetPasswordError {
    Database,
    Mail,
    Redis,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::Mail => {
                write!(f, "An error occurred while sending the email.")
            }
            Self::Redis => {
                write!(f, "An error occurred while accessing Redis.")
            }
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Whether a reset e-mail may be sent to `email`: an account exists and is eligible.
async fn is_eligible(smart_db: &SmartDatabase, email: &str) -> Result<bool, ResetPasswordError> {
    let view = DoesUserExistByEmailQueryView::new(email.to_string());
    let exists: bool = smart_db
        .fetch_scalar(&view)
        .await
        .map_err(|_| ResetPasswordError::Database)?;
    if !exists {
        return Ok(false);
    }

    let view = GetUserIdQueryView::new(email);
    let user_id = smart_db
        .fetch_scalar::<i32, _>(&view)
        .await
        .map_err(|_| ResetPasswordError::Database)?;
    let user_id = u64::try_from(user_id).map_err(|_| ResetPasswordError::Database)?;

    smart_db
        .fetch_scalar::<bool, _>(&IsFirstTimeQueryView::new(user_id))
        .await
        .map_err(|_| ResetPasswordError::Database)
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
                return Err(ResetPasswordError::Mail);
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
            return Err(ResetPasswordError::Mail);
        }
    };

    // Étape 3 : On l'envoie via le serveur SMTP
    match send_email(email).await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Mail Error: {e}");
            Err(ResetPasswordError::Mail)
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
            ResetPasswordError::Redis
        })?;
    redis
        .secure_set(
            &format!("{token}/forgot_password_email"),
            &email.to_string(),
        )
        .await
        .map_err(|e| {
            eprintln!("Redis Error: {e}");
            ResetPasswordError::Redis
        })?;
    match handle_forgot_password(token, email).await {
        Ok(()) => Ok(()),
        Err(_) => Err(ResetPasswordError::Mail),
    }
}

async fn forgot_password_trigger(
    state: web::Data<AppState>,
    view: ForgotPasswordView,
) -> Result<(), ResetPasswordError> {
    let token = state
        .get_redis()
        .secure_get::<String>(&format!("{}/forgot_password_token", view.email()))
        .await;
    // A reset is already waiting for this address: answer as if a new e-mail had been sent.
    if matches!(token, Ok(Some(_))) {
        return Ok(());
    }

    if is_eligible(state.get_smart_db(), view.email()).await? {
        trigger(&state, view.email()).await?;
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "",
    summary = "Request a password reset",
    description = "If the e-mail matches an account, generates a one-time reset token, stores it \
                   in Redis and e-mails it to the user. The token is then sent to \
                   `POST /api/v1/auth/reset_password`.\n\n\
                   Public route: `JwtMiddleware` lets everything under `/auth` through.\n\n\
                   To avoid revealing which e-mails have an account, the answer is **always the \
                   same empty `200`**, whether the address is unknown, already has a pending \
                   reset (no second e-mail is sent until the first token is used) or matches an \
                   account. The token is never returned in the response. Only infrastructure \
                   failures answer `500`.",
    request_body(
        content = ForgotPasswordView,
        description = "E-mail address of the account to reset.",
        example = json!({ "email": "jean.dupont@mairie360.fr" })
    ),
    responses(
        (
            status = 200,
            description = "Request accepted. Empty body. Does not tell whether an e-mail was sent.",
        ),
        (
            status = 400,
            description = "Malformed JSON body or missing `email` field.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `email`")
        ),
        (
            status = 500,
            description = "Database or Redis failure, or the e-mail could not be sent.",
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
