use super::view::{
    PasskeyListResponseView, PasskeyRegistrationOptionsResponseView, RegisterPasskeyView,
};
use crate::database::passkeys::delete::DeletePasskeyQueryView;
use crate::database::passkeys::insert::InsertPasskeyQueryView;
use crate::database::passkeys::list::ListPasskeysQueryView;
use crate::database::passkeys::list_credential_ids::{CredentialIdRow, ListCredentialIdsQueryView};
use crate::database::passkeys::{credential_id_hex, PasskeySummary};
use crate::database::users::get_user_by_id::{GetUserByIdQueryResultView, GetUserByIdQueryView};
use crate::endpoints::db_error::{self, DbFailure};
use crate::endpoints::validation::ValidatedJson;
use crate::webauthn::{user_handle, ChallengeStore, PendingRegistration};
use actix_web::{delete, get, http::StatusCode, post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;
use webauthn_rs::prelude::{CredentialID, Webauthn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasskeyManagementError {
    /// Unknown, expired, already used `challenge_id`, or one opened by another account.
    ChallengeExpired,
    /// The attestation does not answer the ceremony (origin, challenge, format, algorithm...).
    InvalidRegistration,
    /// The credential is already registered (this account or another one).
    AlreadyRegistered,
    NotFound,
    DatabaseError,
    NotConfigured,
    RedisError,
}

impl std::fmt::Display for PasskeyManagementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChallengeExpired => {
                write!(f, "Unknown or expired registration challenge.")
            }
            Self::InvalidRegistration => write!(f, "Invalid passkey registration."),
            Self::AlreadyRegistered => write!(f, "This passkey is already registered."),
            Self::NotFound => write!(f, "Passkey not found."),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
            Self::NotConfigured => write!(f, "Passkeys are not configured."),
            Self::RedisError => write!(f, "Internal Redis error."),
        }
    }
}

impl ResponseError for PasskeyManagementError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ChallengeExpired | Self::InvalidRegistration => StatusCode::BAD_REQUEST,
            Self::AlreadyRegistered => StatusCode::CONFLICT,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            Self::DatabaseError | Self::RedisError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

fn database_failure(context: &str, error: &ApiLibError) -> PasskeyManagementError {
    match db_error::log(context, error) {
        DbFailure::Conflict => PasskeyManagementError::AlreadyRegistered,
        DbFailure::NotFound => PasskeyManagementError::NotFound,
        // A value Postgres refuses (credential id length, label): the attestation is not one
        // Core can store, which the client cannot fix by retrying as is.
        DbFailure::Invalid => PasskeyManagementError::InvalidRegistration,
        DbFailure::Unavailable | DbFailure::Internal => PasskeyManagementError::DatabaseError,
    }
}

fn redis_failure(context: &str, error: &impl std::fmt::Display) -> PasskeyManagementError {
    tracing::error!("Passkey registration: {context}: {error}");
    PasskeyManagementError::RedisError
}

#[utoipa::path(
    post,
    path = "/passkeys/options",
    summary = "Start the registration of a passkey",
    description = "Opens a passkey (WebAuthn) registration ceremony for the user carried by the \
                   JWT and returns the options to hand to `navigator.credentials.create()`: the \
                   account's e-mail and full name as the user entity, a stable user handle \
                   derived from the account id, user verification required, no attestation \
                   required (synchronised passkeys accepted) and the passkeys already registered \
                   excluded, so the authenticator refuses a duplicate.\n\n\
                   The ceremony is identified by `challenge_id`, to send back with the attestation \
                   to `POST /api/v1/user/me/passkeys/`. It is single use, expires after two \
                   minutes and can only be finished by the same account.",
    responses(
        (
            status = 200,
            description = "Ceremony opened: the creation options and the id of the ceremony.",
            body = PasskeyRegistrationOptionsResponseView,
            example = json!({
                "challenge_id": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35",
                "public_key": {
                    "publicKey": {
                        "rp": { "name": "Mairie 360", "id": "mairie360.fr" },
                        "user": {
                            "id": "m3q0Nq1xSfe2x4y7q0cNhA",
                            "name": "jean.dupont@mairie360.fr",
                            "displayName": "Jean Dupont"
                        },
                        "challenge": "xLqpXkT3oF6Q2oFqaX0S5nHyR8yq9b2N7xG4w0a5r1M",
                        "pubKeyCredParams": [
                            { "type": "public-key", "alg": -7 },
                            { "type": "public-key", "alg": -257 }
                        ],
                        "timeout": 60000,
                        "attestation": "none",
                        "excludeCredentials": [],
                        "authenticatorSelection": {
                            "requireResidentKey": false,
                            "userVerification": "required"
                        }
                    }
                }
            })
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Internal error: database or Redis.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "Passkeys are disabled on this instance (`WEBAUTHN_RP_ID` or `WEBAUTHN_RP_ORIGIN` not set).",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not configured.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[post("/passkeys/options")]
pub async fn passkey_registration_options(
    state: web::Data<AppState>,
    webauthn: Option<web::Data<Webauthn>>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PasskeyManagementError> {
    let webauthn = webauthn.ok_or(PasskeyManagementError::NotConfigured)?;
    let smart_db = state.get_smart_db();

    let user: GetUserByIdQueryResultView = smart_db
        .fetch_one(&GetUserByIdQueryView::new(auth_user.id))
        .await
        .map_err(|e| database_failure("passkey registration: read account", &e))?;
    let registered: Vec<CredentialIdRow> = smart_db
        .fetch_all(&ListCredentialIdsQueryView::new(auth_user.id))
        .await
        .map_err(|e| database_failure("passkey registration: list credentials", &e))?;
    let exclude: Vec<CredentialID> = registered
        .iter()
        .filter_map(|row| row.bytes().ok())
        .map(CredentialID::from)
        .collect();

    let display_name = format!("{} {}", user.first_name(), user.last_name());
    let (public_key, registration) = webauthn
        .start_passkey_registration(
            user_handle(auth_user.id),
            user.email(),
            display_name.trim(),
            Some(exclude),
        )
        .map_err(|e| redis_failure("cannot open the ceremony", &e))?;
    let challenge_id = ChallengeStore::new(state.get_redis())
        .store_registration(&PendingRegistration {
            user_id: auth_user.id,
            state: registration,
        })
        .await
        .map_err(|e| redis_failure("cannot store the ceremony", &e))?;

    Ok(
        HttpResponse::Ok().json(PasskeyRegistrationOptionsResponseView::new(
            challenge_id,
            public_key,
        )),
    )
}

#[utoipa::path(
    post,
    path = "/passkeys/",
    summary = "Register a passkey",
    description = "Completes the registration opened by `POST /api/v1/user/me/passkeys/options`: \
                   checks the attestation (origin, relying party, challenge, algorithm) and stores \
                   the new passkey of the user carried by the JWT under the given label. The \
                   passkey can then sign in through `POST /api/v1/auth/passkey`.",
    request_body(
        content = RegisterPasskeyView,
        description = "The ceremony id, a label and the credential as serialised by `PublicKeyCredential.toJSON()`.",
        example = json!({
            "challenge_id": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35",
            "label": "iPhone de Jean",
            "credential": {
                "id": "vJd5R8m2oA7N_3kQ1eF2hWfYbZcTx9L0",
                "rawId": "vJd5R8m2oA7N_3kQ1eF2hWfYbZcTx9L0",
                "type": "public-key",
                "response": {
                    "attestationObject": "o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YVik...",
                    "clientDataJSON": "eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIiwiY2hhbGxlbmdlIjoi...",
                    "transports": ["internal", "hybrid"]
                },
                "clientExtensionResults": {}
            }
        })
    ),
    responses(
        (
            status = 201,
            description = "Passkey registered.",
            body = PasskeySummary,
            example = json!({
                "id": 12,
                "label": "iPhone de Jean",
                "created_at": "2026-10-08T09:30:00Z",
                "last_used_at": null
            })
        ),
        (
            status = 400,
            description = "Malformed body (missing field, credential that is not a WebAuthn `PublicKeyCredential`), `label` empty, longer than 100 characters or holding a control character, unknown, expired or already used `challenge_id` (or one opened by another account), or attestation refused (origin, challenge, relying party, algorithm, user verification). Start the registration again.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Invalid label" = (value = json!("Invalid `label`: must not be empty"))),
                ("Expired challenge" = (value = json!("Unknown or expired registration challenge."))),
                ("Refused attestation" = (value = json!("Invalid passkey registration.")))
            )
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 409,
            description = "This credential is already registered, on this account or another one.",
            body = String,
            content_type = "text/plain",
            example = json!("This passkey is already registered.")
        ),
        (
            status = 500,
            description = "Internal error: database or Redis.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "Passkeys are disabled on this instance (`WEBAUTHN_RP_ID` or `WEBAUTHN_RP_ORIGIN` not set).",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not configured.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[post("/passkeys/")]
pub async fn register_passkey(
    payload: ValidatedJson<RegisterPasskeyView>,
    state: web::Data<AppState>,
    webauthn: Option<web::Data<Webauthn>>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PasskeyManagementError> {
    let webauthn = webauthn.ok_or(PasskeyManagementError::NotConfigured)?;
    let view = payload.into_inner();

    let pending = ChallengeStore::new(state.get_redis())
        .take_registration(view.challenge_id())
        .await
        .map_err(|e| redis_failure("cannot read the ceremony", &e))?
        // A ceremony opened by another account is as good as none: refused the same way.
        .filter(|pending| pending.user_id == auth_user.id)
        .ok_or_else(|| {
            tracing::warn!("Passkey registration refused: unknown, expired or foreign challenge");
            PasskeyManagementError::ChallengeExpired
        })?;

    let passkey = webauthn
        .finish_passkey_registration(view.credential(), &pending.state)
        .map_err(|e| {
            tracing::warn!("Passkey registration refused: {e}");
            PasskeyManagementError::InvalidRegistration
        })?;
    let serialised = serde_json::to_string(&passkey).map_err(|e| {
        tracing::error!("Passkey registration: cannot serialise the credential: {e}");
        PasskeyManagementError::DatabaseError
    })?;

    let summary: PasskeySummary = state
        .get_smart_db()
        .fetch_one(&InsertPasskeyQueryView::new(
            auth_user.id,
            &credential_id_hex(passkey.cred_id()),
            &serialised,
            view.label(),
        ))
        .await
        .map_err(|e| database_failure("passkey registration: insert", &e))?;

    Ok(HttpResponse::Created().json(summary))
}

#[utoipa::path(
    get,
    path = "/passkeys/",
    summary = "List one's passkeys",
    description = "The passkeys registered by the user carried by the JWT, oldest first: id, \
                   label and dates. The public keys are never returned.",
    responses(
        (
            status = 200,
            description = "The registered passkeys, possibly none.",
            body = PasskeyListResponseView,
            example = json!({
                "passkeys": [{
                    "id": 12,
                    "label": "iPhone de Jean",
                    "created_at": "2026-10-08T09:30:00Z",
                    "last_used_at": "2026-10-08T14:02:11Z"
                }]
            })
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error while reading the passkeys.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/passkeys/")]
pub async fn list_passkeys(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PasskeyManagementError> {
    let passkeys: Vec<PasskeySummary> = state
        .get_smart_db()
        .fetch_all(&ListPasskeysQueryView::new(auth_user.id))
        .await
        .map_err(|e| database_failure("list passkeys", &e))?;
    Ok(HttpResponse::Ok().json(PasskeyListResponseView::new(passkeys)))
}

#[utoipa::path(
    delete,
    path = "/passkeys/{id}/",
    summary = "Delete one of one's passkeys",
    description = "Removes a passkey of the user carried by the JWT: it can no longer sign in. \
                   A passkey of another account answers `404`, like an unknown one. The \
                   authenticator keeps its copy; the user deletes it there themselves.",
    params(
        ("id" = u64, Path, description = "Id of the passkey, from `GET /api/v1/user/me/passkeys/`.", example = 12)
    ),
    responses(
        (status = 204, description = "Passkey deleted."),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "No passkey with this id on this account.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkey not found.")
        ),
        (
            status = 500,
            description = "Database error while deleting the passkey.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[delete("/passkeys/{id}/")]
pub async fn delete_passkey(
    id: web::Path<u64>,
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PasskeyManagementError> {
    let deleted: i32 = state
        .get_smart_db()
        .fetch_scalar(&DeletePasskeyQueryView::new(id.into_inner(), auth_user.id))
        .await
        .map_err(|e| database_failure("delete passkey", &e))?;
    if deleted == 0 {
        return Err(PasskeyManagementError::NotFound);
    }
    Ok(HttpResponse::NoContent().finish())
}
