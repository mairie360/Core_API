use crate::client_ip::client_ip;
use crate::database::auth::active_user_id::GetActiveUserIdByEmailQueryView;
use crate::endpoints::v1::auth::forgot_password::view::ForgotPasswordView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{email_key, ip_key, too_many_requests_response, RateLimits};
use crate::redis_keys::{set_token, FORGOT_PASSWORD_TTL_SECONDS};
use crate::{build_email, get_email_sender, send_email, EmailDestination};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
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
    TooManyRequests(u64),
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
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Whether a reset e-mail may be sent to `email`: a non-archived account uses it. Activated
/// accounts are eligible too (MAIR-390): before, only accounts still waiting for their first
/// connection were, so a user who forgot their password silently received nothing.
async fn is_eligible(smart_db: &SmartDatabase, email: &str) -> Result<bool, ResetPasswordError> {
    smart_db
        .fetch_scalar::<i32, _>(&GetActiveUserIdByEmailQueryView::new(email))
        .await
        .map(|user_id| user_id > 0)
        .map_err(|e| {
            eprintln!("Forgot password DB Error: {e}");
            ResetPasswordError::Database
        })
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
    set_token(
        redis,
        &format!("{email}/forgot_password_token"),
        &token,
        FORGOT_PASSWORD_TTL_SECONDS,
    )
    .await
    .map_err(|e| {
        eprintln!("Redis Error: {e}");
        ResetPasswordError::Redis
    })?;
    set_token(
        redis,
        &format!("{token}/forgot_password_email"),
        email,
        FORGOT_PASSWORD_TTL_SECONDS,
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
                   failures answer `500`.\n\n\
                   Every non-archived account is eligible, whether it was activated or not; an \
                   archived account is treated like an unknown address.\n\n\
                   Rate limited: 10 requests per 15 minutes per client address and 3 per hour per \
                   e-mail address; beyond that the answer is `429`.",
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
            description = "Malformed JSON body, missing `email`, or `email` that is not a valid address of at most 320 characters.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `email`: must be a valid e-mail address")
        ),
        (
            status = 429,
            description = "Too many requests from this client address or for this e-mail address. The `Retry-After` header gives the number of seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(
                ("Retry-After" = u64, description = "Seconds to wait before the next request.")
            ),
            example = json!("Too many requests, please try again later.")
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
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn forgot_password(
    state: web::Data<AppState>,
    body: ValidatedJson<ForgotPasswordView>,
    limits: Option<web::Data<RateLimits>>,
    request: HttpRequest,
) -> Result<impl Responder, ResetPasswordError> {
    let view = body.into_inner();
    if let Some(limits) = limits {
        limits
            .forgot_password_per_ip
            .hit(&ip_key(client_ip(&request)))
            .and_then(|()| {
                limits
                    .forgot_password_per_email
                    .hit(&email_key(view.email()))
            })
            .map_err(|refused| ResetPasswordError::TooManyRequests(refused.retry_after_seconds))?;
    }
    forgot_password_trigger(state, view).await?;
    Ok(HttpResponse::Ok())
}
