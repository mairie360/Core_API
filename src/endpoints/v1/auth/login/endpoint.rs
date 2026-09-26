use super::view::{LoginResponseView, LoginView};
use crate::database::auth::change_password::ChangePasswordQueryView;
use crate::database::auth::login::LoginUserQueryView;
use crate::database::sessions::create_session::CreateSessionQueryView;
use crate::endpoints::v1::auth::login::view::LoginFirstConnectionResponseView;
use crate::redis_keys::{set_token, FIRST_CONNECTION_TTL_SECONDS};
use crate::session_jwt::generate_session_jwt;
use actix_web::{
    dev::ConnectionInfo, http::StatusCode, post, web, HttpResponse, Responder, ResponseError,
};
use base64::{engine::general_purpose, Engine as _};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::password::{hash_password, is_hashed, verify_password};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use rand::fill;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginError {
    DatabaseError,
    FirstConnectError(String),
    InvalidCredentials,
    RedisError,
    TokenGenerationError,
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "Invalid credentials provided."),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::TokenGenerationError => write!(f, "Failed to generate JWT token."),
            Self::FirstConnectError(token) => {
                write!(f, "{token}")
            }
            Self::RedisError => write!(f, "Internal Redis error."),
        }
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError | Self::RedisError | Self::TokenGenerationError => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::FirstConnectError(_) => StatusCode::PRECONDITION_FAILED,
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if self.status_code() == StatusCode::PRECONDITION_FAILED {
            return HttpResponse::build(self.status_code())
                .json(LoginFirstConnectionResponseView::new(self.to_string()));
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn generate_refresh_token() -> String {
    // 32 octets (256 bits) est un standard de sécurité solide
    let mut buffer = [0u8; 32];

    // Remplissage avec des données aléatoires sécurisées
    fill(&mut buffer);

    // Encodage en Base64 pour avoir une String lisible
    general_purpose::URL_SAFE_NO_PAD.encode(buffer)
}

/// # Errors
///
/// Retourne une erreur si la génération du JWT échoue.
pub async fn generate_session(
    user_id: u64,
    device_info: &str,
    ip_adress: std::net::IpAddr,
    state: web::Data<AppState>,
) -> Result<(String, String), LoginError> {
    let refresh_token = generate_refresh_token();
    // The session id is chosen here so the JWT can carry it (`sid` claim): logout revokes exactly
    // this session, and the JWT stops working once it is revoked (MAIR-226). Each login keeps its
    // own session: other devices' sessions stay active.
    let session_id = Uuid::new_v4();
    let view = CreateSessionQueryView::with_id(
        session_id,
        user_id,
        &refresh_token,
        device_info,
        ip_adress,
    );
    state.get_smart_db().execute(view).await.map_err(|e| {
        eprintln!("Create Session DB Error: {e}");
        LoginError::DatabaseError
    })?;
    // The role claim stays empty: neither the lib nor Core reads it.
    let jwt = generate_session_jwt(user_id, session_id).map_err(|e| {
        eprintln!("JWT Generation Error: {e}");
        LoginError::TokenGenerationError
    })?;
    Ok((jwt, refresh_token))
}

async fn generate_first_connection_token(
    user_id: u64,
    state: web::Data<AppState>,
) -> Result<String, LoginError> {
    let redis = state.get_redis();

    if let Ok(Some(token)) = redis
        .secure_get::<String>(&format!("{user_id}/first_connection_token"))
        .await
    {
        return Ok(token);
    }
    let token = Uuid::new_v4().to_string();
    set_token(
        redis,
        &format!("{user_id}/first_connection_token"),
        &token,
        FIRST_CONNECTION_TTL_SECONDS,
    )
    .await
    .map_err(|e| {
        eprintln!("Redis Error: {e}");
        LoginError::RedisError
    })?;
    set_token(
        redis,
        &format!("{token}/first_connection_id"),
        &user_id.to_string(),
        FIRST_CONNECTION_TTL_SECONDS,
    )
    .await
    .map_err(|e| {
        eprintln!("Redis Error: {e}");
        LoginError::RedisError
    })?;
    Ok(token)
}

/// Rehashes a legacy plaintext password into an argon2id hash once its owner has proven they
/// know it. A failure here (hashing or write) is logged and swallowed: the login itself already
/// succeeded and must not fail because the opportunistic migration didn't.
async fn migrate_plaintext_password(smart_db: &SmartDatabase, user_id: u64, plaintext: &str) {
    let hashed = match hash_password(plaintext) {
        Ok(hashed) => hashed,
        Err(e) => {
            eprintln!("Failed to hash password while migrating user {user_id}: {e}");
            return;
        }
    };
    if let Err(e) = smart_db
        .execute(ChangePasswordQueryView::new(&hashed, user_id))
        .await
    {
        eprintln!("Failed to persist migrated password for user {user_id}: {e}");
    }
}

async fn login_user(
    login_view: &LoginView,
    state: web::Data<AppState>,
    ip_adress: std::net::IpAddr,
) -> Result<(String, String), LoginError> {
    let view = LoginUserQueryView::new(login_view.email(), login_view.password());

    let user_record = match state
        .get_smart_db()
        .fetch_one::<crate::database::auth::login::LoginUserQueryResultView, _>(&view)
        .await
    {
        Ok(result) => Some(result),
        Err(ApiLibError::Database(DbError::NotFound)) => None,
        Err(e) => {
            eprintln!("Login DB Error: {e}");
            return Err(LoginError::DatabaseError);
        }
    };

    let Some(user) = user_record else {
        eprintln!(
            "Login failed: Invalid credentials for {}",
            login_view.email()
        );
        return Err(LoginError::InvalidCredentials);
    };

    if user.first_connect() {
        return Err(LoginError::FirstConnectError(
            generate_first_connection_token(user.user_id() as u64, state).await?,
        ));
    }

    let stored_password = user.password();
    // Accounts created before this migration still hold a plaintext password: compare it
    // directly and, on success, replace it with a hash so the plaintext value is never read
    // again. Everything hashed already goes through `verify_password`, which only accepts a
    // value `is_hashed` agrees is an argon2id PHC string.
    let credentials_valid = if is_hashed(stored_password) {
        verify_password(&login_view.password(), stored_password).unwrap_or_else(|e| {
            eprintln!(
                "Failed to verify password hash for {}: {e}",
                login_view.email()
            );
            false
        })
    } else {
        login_view.password() == stored_password.trim()
    };

    if !credentials_valid {
        eprintln!(
            "Login failed: Invalid credentials for {}",
            login_view.email()
        );
        return Err(LoginError::InvalidCredentials);
    }

    if !is_hashed(stored_password) {
        migrate_plaintext_password(
            state.get_smart_db(),
            user.user_id() as u64,
            &login_view.password(),
        )
        .await;
    }

    generate_session(
        user.user_id() as u64,
        &login_view.device_info(),
        ip_adress,
        state,
    )
    .await
}

#[utoipa::path(
    post,
    path = "",
    summary = "Se connecter",
    description = "Authentifie un utilisateur par e-mail et mot de passe, ouvre une session et \
                   renvoie un JWT dans l'en-tête `Authorization` ainsi qu'un jeton de \
                   rafraîchissement dans le corps. Route publique : le `JwtMiddleware` laisse \
                   passer tout ce qui est sous `/auth`.\n\n\
                   La session est liée à l'adresse IP d'origine : le rafraîchissement et la \
                   révocation devront repasser par la même adresse.\n\n\
                   Tant que l'utilisateur n'a pas choisi son propre mot de passe, la réponse est \
                   `412` et non `200` : elle porte alors un jeton de première connexion à \
                   présenter à `POST /api/v1/auth/force_change_password`. C'est le seul statut \
                   d'erreur de cette opération dont le corps est du JSON et non du texte brut.",
    request_body(
        content = LoginView,
        description = "Identifiants de l'utilisateur et description de l'appareil utilisé.",
        example = json!({
            "email": "jean.dupont@mairie360.fr",
            "password": "MotDePasse!123",
            "device_info": "Chrome 140 sur Windows 11"
        })
    ),
    responses(
        (
            status = 200,
            description = "Connexion réussie. Le JWT est renvoyé dans l'en-tête `Authorization`, le jeton de rafraîchissement dans le corps.",
            body = LoginResponseView,
            headers(
                ("Authorization" = String, description = "JWT d'accès, préfixé par `Bearer `.")
            ),
            example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
        ),
        (
            status = 401,
            description = "Adresse e-mail inconnue ou mot de passe incorrect.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid credentials provided.")
        ),
        (
            status = 412,
            description = "Première connexion : le mot de passe doit être changé via `/api/v1/auth/force_change_password` en utilisant le jeton renvoyé.",
            body = LoginFirstConnectionResponseView,
            example = json!({ "token": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35" })
        ),
        (
            status = 500,
            description = "Erreur interne : base de données, Redis ou génération du JWT.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Auth"
)]
#[post("/login")]
pub async fn login(
    payload: web::Json<LoginView>,
    state: web::Data<AppState>,
    conn: ConnectionInfo,
) -> Result<impl Responder, LoginError> {
    let login_view = payload.into_inner();
    let ip_str = conn.realip_remote_addr().unwrap_or("unknown").to_string();
    let ip_address = ip_str
        .parse::<std::net::IpAddr>()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    let (jwt, refresh_token) = login_user(&login_view, state, ip_address).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(LoginResponseView::from(refresh_token)))
}
