use super::view::KeycloakLoginView;
use crate::database::auth::sso_login::{SsoLoginUserQueryResultView, SsoLoginUserQueryView};
use crate::endpoints::v1::auth::login::endpoint::{generate_session, LoginError};
use crate::endpoints::v1::auth::login::view::LoginResponseView;
use crate::keycloak::{AuthorizationCode, KeycloakClient, KeycloakError};
use actix_web::{
    dev::ConnectionInfo, http::StatusCode, post, web, HttpResponse, Responder, ResponseError,
};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeycloakLoginError {
    AccountArchived,
    DatabaseError,
    EmailNotVerified,
    InvalidGrant,
    InvalidIdToken,
    KeycloakUnavailable,
    NotConfigured,
    TokenGenerationError,
    UnknownAccount,
}

impl std::fmt::Display for KeycloakLoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccountArchived => write!(f, "This Mairie 360 account is archived."),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
            Self::EmailNotVerified => {
                write!(f, "{}", KeycloakError::EmailNotVerified)
            }
            Self::InvalidGrant => write!(f, "{}", KeycloakError::InvalidGrant),
            Self::InvalidIdToken => write!(f, "{}", KeycloakError::InvalidIdToken),
            Self::KeycloakUnavailable => write!(f, "{}", KeycloakError::Unavailable),
            Self::NotConfigured => write!(f, "Keycloak sign-in is not configured."),
            Self::TokenGenerationError => write!(f, "Failed to generate JWT token."),
            Self::UnknownAccount => write!(
                f,
                "No Mairie 360 account matches this Keycloak e-mail address."
            ),
        }
    }
}

impl ResponseError for KeycloakLoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidGrant | Self::InvalidIdToken => StatusCode::UNAUTHORIZED,
            Self::AccountArchived | Self::EmailNotVerified | Self::UnknownAccount => {
                StatusCode::FORBIDDEN
            }
            Self::KeycloakUnavailable => StatusCode::BAD_GATEWAY,
            Self::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            Self::DatabaseError | Self::TokenGenerationError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

impl From<KeycloakError> for KeycloakLoginError {
    fn from(error: KeycloakError) -> Self {
        match error {
            KeycloakError::InvalidGrant => Self::InvalidGrant,
            KeycloakError::InvalidIdToken => Self::InvalidIdToken,
            KeycloakError::EmailNotVerified => Self::EmailNotVerified,
            KeycloakError::Unavailable => Self::KeycloakUnavailable,
        }
    }
}

async fn find_account(
    email: &str,
    state: &web::Data<AppState>,
) -> Result<SsoLoginUserQueryResultView, KeycloakLoginError> {
    match state
        .get_smart_db()
        .fetch_one::<SsoLoginUserQueryResultView, _>(&SsoLoginUserQueryView::new(email))
        .await
    {
        Ok(user) if user.is_archived() => Err(KeycloakLoginError::AccountArchived),
        Ok(user) => Ok(user),
        Err(ApiLibError::Database(DbError::NotFound)) => {
            eprintln!("Keycloak login refused: no account for {email}");
            Err(KeycloakLoginError::UnknownAccount)
        }
        Err(e) => {
            eprintln!("Keycloak login DB Error: {e}");
            Err(KeycloakLoginError::DatabaseError)
        }
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Sign in with Keycloak",
    description = "Completes the Keycloak single sign-on (OpenID Connect authorization code \
                   flow): Core redeems `code` at the realm's token endpoint, verifies the \
                   returned ID token (signature against the realm keys, issuer, audience = \
                   Core's client id, expiry, and `nonce` when one is sent), then opens a \
                   session for the Mairie 360 account whose e-mail matches the token's verified \
                   e-mail (case-insensitive).\n\n\
                   The response is the same as `POST /api/v1/auth/login`: a Core JWT in the \
                   `Authorization` header and a refresh token in the body, so the session cookie, \
                   the refresh route and every API keep working unchanged, and the user keeps the \
                   roles and groups stored in Core. No account is created: a Keycloak identity \
                   without a matching account is refused with `403`. The first-connection \
                   password change of the password login does not apply here, and the account's \
                   password is left untouched, so `POST /api/v1/auth/login` stays usable during \
                   the transition.\n\n\
                   Public route (the `JwtMiddleware` lets everything under `/auth` through). An \
                   authorization code is single use: replaying it returns `401`. The session is \
                   bound to the caller's IP address, like a password login.",
    request_body(
        content = KeycloakLoginView,
        description = "Authorization code returned by Keycloak on the redirect URI, with the \
                       values of the authorization request needed to redeem and check it.",
        example = json!({
            "code": "7c1e0f5a-2b8d-4f3e-9a61-d4c2b7e8f901.3b5d9e2a-6f14-4c8b-a7d0-1e9f2c4b6a83",
            "redirect_uri": "https://login.mairie360.fr/auth/callback",
            "code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
            "nonce": "n-0S6_WzA2Mj",
            "device_info": "Firefox 142 on Ubuntu 24.04"
        })
    ),
    responses(
        (
            status = 200,
            description = "Signed in. The JWT is in the `Authorization` header, the refresh token in the body.",
            body = LoginResponseView,
            headers(
                ("Authorization" = String, description = "Access JWT, prefixed with `Bearer `.")
            ),
            example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
        ),
        (
            status = 400,
            description = "Malformed body: missing `code`, `redirect_uri` or `device_info`, or a field of the wrong type.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `code` at line 1 column 42")
        ),
        (
            status = 401,
            description = "Keycloak refused the code (unknown, expired, already used, or `redirect_uri` / `code_verifier` not matching the authorization request), or the ID token failed verification (signature, issuer, audience, expiry or `nonce`). Restart the sign-in from Keycloak.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Invalid grant" = (value = json!("Keycloak rejected the authorization code."))),
                ("Invalid ID token" = (value = json!("Invalid Keycloak ID token.")))
            )
        ),
        (
            status = 403,
            description = "The Keycloak identity cannot be mapped to an active Mairie 360 account: its e-mail is missing or not verified in Keycloak, no account has this e-mail, or the account is archived.",
            body = String,
            content_type = "text/plain",
            examples(
                ("E-mail not verified" = (value = json!("The Keycloak account has no verified e-mail address."))),
                ("Unknown account" = (value = json!("No Mairie 360 account matches this Keycloak e-mail address."))),
                ("Archived account" = (value = json!("This Mairie 360 account is archived.")))
            )
        ),
        (
            status = 500,
            description = "Internal error: database or JWT generation.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 502,
            description = "Keycloak could not be reached, answered unexpectedly, or rejected Core's client credentials (`KEYCLOAK_CLIENT_ID` / `KEYCLOAK_CLIENT_SECRET`). Retrying later may succeed if the code has not expired.",
            body = String,
            content_type = "text/plain",
            example = json!("Keycloak is unavailable.")
        ),
        (
            status = 503,
            description = "Keycloak sign-in is disabled on this instance (`KEYCLOAK_REALM_URL` or `KEYCLOAK_CLIENT_ID` not set). Use `POST /api/v1/auth/login` instead.",
            body = String,
            content_type = "text/plain",
            example = json!("Keycloak sign-in is not configured.")
        )
    ),
    tag = "Auth"
)]
#[post("/keycloak")]
pub async fn keycloak_login(
    payload: web::Json<KeycloakLoginView>,
    state: web::Data<AppState>,
    keycloak: Option<web::Data<KeycloakClient>>,
    conn: ConnectionInfo,
) -> Result<impl Responder, KeycloakLoginError> {
    let keycloak = keycloak.ok_or(KeycloakLoginError::NotConfigured)?;
    let login_view = payload.into_inner();
    let ip_address = conn
        .realip_remote_addr()
        .and_then(|ip| ip.parse::<std::net::IpAddr>().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    let identity = keycloak
        .authenticate(&AuthorizationCode {
            code: login_view.code(),
            redirect_uri: login_view.redirect_uri(),
            code_verifier: login_view.code_verifier(),
            nonce: login_view.nonce(),
        })
        .await?;
    let user = find_account(&identity.email, &state).await?;

    let (jwt, refresh_token) = generate_session(
        user.user_id() as u64,
        login_view.device_info(),
        ip_address,
        state,
    )
    .await
    .map_err(|e| match e {
        LoginError::TokenGenerationError => KeycloakLoginError::TokenGenerationError,
        _ => KeycloakLoginError::DatabaseError,
    })?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(LoginResponseView::from(refresh_token)))
}
