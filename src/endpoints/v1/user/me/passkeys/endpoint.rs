use super::view::{PasskeyRegisterOptionsView, PasskeyRegisterView, PasskeyView};
use crate::client_ip::client_ip;
use crate::database::passkeys::{
    DeletePasskeyQueryView, DeletedPasskeyRecord, InsertPasskeyQueryView,
    ListPasskeySecretsQueryView, ListPasskeysQueryView, PasskeyRecord, PasskeySecretRecord,
    UserContactQueryView, UserContactRecord,
};
use crate::emails::{AccountEmails, PasskeyChange};
use crate::endpoints::db_error::{self, DbFailure};
use crate::endpoints::validation::ValidatedJson;
use crate::passkeys::{self, Passkeys};
use crate::rate_limit::{too_many_requests_response, user_key, RateLimits};
use crate::two_factor;
use actix_web::http::StatusCode;
use actix_web::{delete, get, post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use base64::{engine::general_purpose, Engine as _};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;
use webauthn_rs::prelude::{Passkey, RegisterPublicKeyCredential};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasskeyError {
    /// The instance has no `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN`.
    Unavailable,
    /// The administrator does not accept passkeys.
    NotAllowed,
    /// The registration was not started, expired, was already used or the answer does not verify.
    InvalidRegistration,
    /// No such passkey among the caller's.
    NotFound,
    /// This credential is already registered.
    AlreadyRegistered,
    TooManyRequests(u64),
    DatabaseError,
}

impl std::fmt::Display for PasskeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "Passkeys are not configured."),
            Self::NotAllowed => write!(f, "Passkeys are not allowed on this instance."),
            Self::InvalidRegistration => write!(f, "Invalid or expired passkey registration."),
            Self::NotFound => write!(f, "Passkey not found."),
            Self::AlreadyRegistered => write!(f, "This passkey is already registered."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for PasskeyError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::NotAllowed => StatusCode::FORBIDDEN,
            Self::InvalidRegistration => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::AlreadyRegistered => StatusCode::CONFLICT,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

pub(crate) fn db_failure(context: &str, e: &ApiLibError) -> PasskeyError {
    match db_error::log(context, e) {
        DbFailure::Conflict => PasskeyError::AlreadyRegistered,
        DbFailure::NotFound => PasskeyError::NotFound,
        _ => PasskeyError::DatabaseError,
    }
}

pub(crate) fn spend_budget(limits: Option<&RateLimits>, user_id: u64) -> Result<(), PasskeyError> {
    if let Some(limits) = limits {
        limits
            .second_factor_per_user
            .hit(&user_key(user_id))
            .map_err(|refused| PasskeyError::TooManyRequests(refused.retry_after_seconds))?;
    }
    Ok(())
}

/// Passkeys are configured (`503` otherwise) and accepted by the administrator (`403` otherwise).
pub(crate) async fn ensure_available<'a>(
    state: &AppState,
    relying_party: Option<&'a web::Data<Passkeys>>,
) -> Result<&'a Passkeys, PasskeyError> {
    let relying_party = relying_party
        .map(web::Data::get_ref)
        .ok_or(PasskeyError::Unavailable)?;
    let policy = two_factor::load_policy(state.get_smart_db())
        .await
        .map_err(|e| db_failure("passkey policy", &e))?;
    if !policy.passkey_allowed {
        return Err(PasskeyError::NotAllowed);
    }
    Ok(relying_party)
}

/// Tells the agent, by e-mail, that a passkey was added or removed: device, date (in the mail) and
/// address. A failure is logged and never undoes the change.
#[allow(clippy::future_not_send)]
pub(crate) async fn notify(
    state: &AppState,
    emails: Option<&AccountEmails>,
    user_id: u64,
    change: PasskeyChange,
    label: &str,
    request: &HttpRequest,
) {
    let Some(emails) = emails else {
        tracing::warn!(
            user_id,
            "Passkey change: no mailer is registered, nobody is told"
        );
        return;
    };
    let contact: UserContactRecord = match state
        .get_smart_db()
        .fetch_one(&UserContactQueryView::new(user_id))
        .await
    {
        Ok(contact) => contact,
        Err(e) => {
            tracing::warn!(user_id, "Passkey change: the recipient is unknown: {e}");
            return;
        }
    };
    let device = request
        .headers()
        .get("User-Agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if let Err(e) = emails
        .send_passkey_changed(
            &contact.email,
            &contact.first_name,
            change,
            label,
            device,
            &client_ip(request).to_string(),
        )
        .await
    {
        tracing::warn!(user_id, "Passkey change: the e-mail could not be sent: {e}");
    }
}

#[utoipa::path(
    get,
    path = "/passkeys/",
    summary = "List my passkeys",
    description = "The passkeys of the caller, oldest first, with their label and dates. The \
                   public key is never returned. An agent without passkeys gets an empty list.",
    responses(
        (
            status = 200,
            description = "The caller's passkeys.",
            body = Vec<PasskeyView>,
            example = json!([{
                "id": 12,
                "label": "Town hall laptop",
                "created_at": "2026-11-17T09:30:00+00:00",
                "last_used_at": null
            }])
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[get("/passkeys/")]
pub async fn list_my_passkeys(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PasskeyError> {
    let rows: Vec<PasskeyRecord> = state
        .get_smart_db()
        .fetch_all(&ListPasskeysQueryView::new(auth_user.id))
        .await
        .map_err(|e| db_failure("list passkeys", &e))?;
    let views: Vec<PasskeyView> = rows
        .into_iter()
        .map(|row| PasskeyView {
            id: row.id,
            label: row.label,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
        })
        .collect();
    Ok(HttpResponse::Ok().json(views))
}

#[utoipa::path(
    post,
    path = "/passkeys/options/",
    summary = "Register a passkey: get the challenge",
    description = "Starts the registration of a passkey for the caller: returns the WebAuthn \
                   options (user verification required, no attestation, so synchronised passkeys \
                   are accepted; the passkeys already registered are excluded). The ceremony is \
                   kept two minutes and read once: send the browser's answer to \
                   `POST /api/v1/user/me/passkeys/`. Starting again replaces the pending one.\n\n\
                   Rate limited per user, like the second-factor codes.",
    responses(
        (
            status = 200,
            description = "Challenge created.",
            body = PasskeyRegisterOptionsView,
            example = json!({
                "options": {
                    "publicKey": {
                        "rp": { "name": "Mairie 360", "id": "mairie360-eip.fr" },
                        "challenge": "q1w2e3r4t5y6u7i8o9p0a1s2d3f4g5h6j7k8l9z0x1c",
                        "pubKeyCredParams": [{ "type": "public-key", "alg": -7 }],
                        "authenticatorSelection": { "userVerification": "required" },
                        "attestation": "none"
                    }
                }
            })
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "The administrator does not accept passkeys (two-factor policy).",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not allowed on this instance.")
        ),
        (
            status = 429,
            description = "Too many attempts. The `Retry-After` header gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(("Retry-After" = u64, description = "Seconds to wait before the next attempt.")),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Database or Redis error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "The instance has no `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN`.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not configured.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[post("/passkeys/options/")]
pub async fn passkey_register_options(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
    limits: Option<web::Data<RateLimits>>,
    relying_party: Option<web::Data<Passkeys>>,
) -> Result<impl Responder, PasskeyError> {
    spend_budget(limits.as_ref().map(web::Data::get_ref), auth_user.id)?;
    let relying_party = ensure_available(&state, relying_party.as_ref()).await?;
    let options = start_registration(&state, relying_party, auth_user.id).await?;
    Ok(HttpResponse::Ok().json(options))
}

/// Starts the registration ceremony of `user_id`: the options for the browser, the ceremony kept in
/// Redis for the answer. Shared by the signed-in route and the mandatory enrolment.
pub(crate) async fn start_registration(
    state: &AppState,
    relying_party: &Passkeys,
    user_id: u64,
) -> Result<PasskeyRegisterOptionsView, PasskeyError> {
    let db = state.get_smart_db();
    let contact: UserContactRecord = db
        .fetch_one(&UserContactQueryView::new(user_id))
        .await
        .map_err(|e| db_failure("passkey registration recipient", &e))?;
    let existing: Vec<PasskeySecretRecord> = db
        .fetch_all(&ListPasskeySecretsQueryView::new(user_id))
        .await
        .map_err(|e| db_failure("list passkey credentials", &e))?;
    let exclude: Vec<_> = existing
        .into_iter()
        .filter_map(|row| serde_json::from_value::<Passkey>(row.passkey).ok())
        .map(|passkey| passkey.cred_id().clone())
        .collect();
    let (options, ceremony) = relying_party
        .webauthn()
        .start_passkey_registration(
            passkeys::user_handle(user_id),
            &contact.email,
            &contact.first_name,
            Some(exclude),
        )
        .map_err(|e| {
            tracing::error!("Passkey registration options: {e}");
            PasskeyError::DatabaseError
        })?;
    passkeys::begin_register(state.get_redis(), user_id, &ceremony)
        .await
        .map_err(|e| {
            tracing::error!("Passkey registration Redis Error: {e}");
            PasskeyError::DatabaseError
        })?;
    let options = serde_json::to_value(options).map_err(|e| {
        tracing::error!("Passkey registration serialization: {e}");
        PasskeyError::DatabaseError
    })?;
    Ok(PasskeyRegisterOptionsView { options })
}

#[utoipa::path(
    post,
    path = "/passkeys/",
    summary = "Register a passkey: send the browser's answer",
    description = "Verifies the answer to the challenge of `POST /api/v1/user/me/passkeys/options/` \
                   (origin, `rpId`, user verification) and stores the passkey under the given \
                   label. The pending registration is burnt by the first attempt, right or \
                   wrong. The agent receives a security e-mail (device, date, address).\n\n\
                   Rate limited per user, like the second-factor codes.",
    request_body = PasskeyRegisterView,
    responses(
        (
            status = 201,
            description = "Passkey registered.",
            body = PasskeyView,
            example = json!({
                "id": 12,
                "label": "Town hall laptop",
                "created_at": "2026-11-17T09:30:00+00:00",
                "last_used_at": null
            })
        ),
        (
            status = 400,
            description = "Malformed body, empty or too long label, or no pending registration (never started, expired, already used) or an answer that does not verify.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid or expired passkey registration.")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "The administrator does not accept passkeys (two-factor policy).",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not allowed on this instance.")
        ),
        (
            status = 409,
            description = "This credential is already registered.",
            body = String,
            content_type = "text/plain",
            example = json!("This passkey is already registered.")
        ),
        (
            status = 429,
            description = "Too many attempts. The `Retry-After` header gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(("Retry-After" = u64, description = "Seconds to wait before the next attempt.")),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Database or Redis error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "The instance has no `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN`.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not configured.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[post("/passkeys/")]
#[allow(clippy::too_many_arguments, clippy::future_not_send)]
pub async fn register_passkey(
    payload: ValidatedJson<PasskeyRegisterView>,
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
    limits: Option<web::Data<RateLimits>>,
    relying_party: Option<web::Data<Passkeys>>,
    emails: Option<web::Data<AccountEmails>>,
    request: HttpRequest,
) -> Result<impl Responder, PasskeyError> {
    let view = payload.into_inner();
    spend_budget(limits.as_ref().map(web::Data::get_ref), auth_user.id)?;
    let relying_party = ensure_available(&state, relying_party.as_ref()).await?;
    let stored = finish_registration(
        &state,
        relying_party,
        auth_user.id,
        view.label(),
        view.credential(),
    )
    .await?;
    notify(
        &state,
        emails.as_ref().map(web::Data::get_ref),
        auth_user.id,
        PasskeyChange::Added,
        &stored.label,
        &request,
    )
    .await;
    Ok(HttpResponse::Created().json(PasskeyView {
        id: stored.id,
        label: stored.label,
        created_at: stored.created_at,
        last_used_at: stored.last_used_at,
    }))
}

/// Checks the browser's answer against the pending ceremony of `user_id` and stores the passkey. The
/// ceremony is burnt by the first attempt, right or wrong.
pub(crate) async fn finish_registration(
    state: &AppState,
    relying_party: &Passkeys,
    user_id: u64,
    label: &str,
    credential: &RegisterPublicKeyCredential,
) -> Result<PasskeyRecord, PasskeyError> {
    // Taken first: a wrong answer burns the challenge.
    let ceremony = passkeys::take_register(state.get_redis(), user_id)
        .await
        .ok_or(PasskeyError::InvalidRegistration)?;
    let passkey = relying_party
        .webauthn()
        .finish_passkey_registration(credential, &ceremony)
        .map_err(|e| {
            tracing::warn!(user_id = user_id, "Passkey registration refused: {e}");
            PasskeyError::InvalidRegistration
        })?;
    let serialized = serde_json::to_string(&passkey).map_err(|e| {
        tracing::error!("Passkey serialization: {e}");
        PasskeyError::DatabaseError
    })?;
    let stored: PasskeyRecord = state
        .get_smart_db()
        .fetch_one(&InsertPasskeyQueryView::new(
            user_id,
            &general_purpose::STANDARD.encode(passkey.cred_id()),
            &serialized,
            label,
        ))
        .await
        .map_err(|e| db_failure("register passkey", &e))?;
    Ok(stored)
}

#[utoipa::path(
    delete,
    path = "/passkeys/{id}/",
    summary = "Delete one of my passkeys",
    description = "Removes a passkey of the caller. A passkey of another account answers `404`, \
                   like an unknown one: nothing tells whether it exists. The agent receives a \
                   security e-mail (device, date, address). The password and the other passkeys \
                   keep working.",
    params(("id" = u64, Path, description = "Identifier of the passkey, from `GET /api/v1/user/me/passkeys/`.", example = 12)),
    responses(
        (status = 204, description = "Passkey removed."),
        (
            status = 400,
            description = "The id is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid path: ids are integers.")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "No such passkey among the caller's.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkey not found.")
        ),
        (
            status = 500,
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[delete("/passkeys/{id}/")]
#[allow(clippy::future_not_send)]
pub async fn delete_my_passkey(
    path: web::Path<u64>,
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
    emails: Option<web::Data<AccountEmails>>,
    request: HttpRequest,
) -> Result<impl Responder, PasskeyError> {
    let deleted: DeletedPasskeyRecord = state
        .get_smart_db()
        .fetch_one(&DeletePasskeyQueryView::new(
            auth_user.id,
            path.into_inner(),
        ))
        .await
        .map_err(|e| match db_failure("delete passkey", &e) {
            // No row also covers a foreign key: only "not yours / unknown" is worth saying.
            PasskeyError::NotFound => PasskeyError::NotFound,
            other => other,
        })?;
    notify(
        &state,
        emails.as_ref().map(web::Data::get_ref),
        auth_user.id,
        PasskeyChange::Removed,
        &deleted.label,
        &request,
    )
    .await;
    Ok(HttpResponse::NoContent().finish())
}
