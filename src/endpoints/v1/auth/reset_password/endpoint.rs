use crate::database::auth::change_password::ChangePasswordQueryView;
use crate::database::get_user_id::GetUserIdQueryView;
use crate::endpoints::v1::auth::login::endpoint::generate_session;
use crate::endpoints::v1::auth::reset_password::view::{
    ResetPasswordResponseView, ResetPasswordView,
};
use crate::redis_keys::redis_key;
use crate::session_revocation::revoke_all_user_sessions;
use actix_web::dev::ConnectionInfo;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    DatabaseError,
    RedisError,
    TokenGenerationError,
    UnknownToken,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError | Self::RedisError | Self::TokenGenerationError => {
                write!(f, "Internal server error")
            }
            Self::UnknownToken => {
                write!(f, "Unknown token")
            }
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError | Self::RedisError | Self::TokenGenerationError => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::UnknownToken => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_id(smart_db: &SmartDatabase, email: &str) -> Result<u64, ResetPasswordError> {
    let view = GetUserIdQueryView::new(email);
    smart_db
        .fetch_scalar::<i32, _>(&view)
        .await
        .map_or(Err(ResetPasswordError::DatabaseError), |user_id| {
            Ok(user_id as u64)
        })
}

async fn reset_pwd(
    smart_db: &SmartDatabase,
    new_password: &str,
    user_id: u64,
) -> Result<(), ResetPasswordError> {
    let view = ChangePasswordQueryView::new(new_password, user_id);
    smart_db
        .execute(view)
        .await
        .map_err(|_| ResetPasswordError::DatabaseError)?;
    Ok(())
}

async fn reset_password_trigger(
    state: web::Data<AppState>,
    view: ResetPasswordView,
    ip_adress: std::net::IpAddr,
) -> Result<(String, String), ResetPasswordError> {
    let smart_db = state.get_smart_db();
    let redis = state.get_redis();

    let key = redis_key(&format!("{}/forgot_password_email", view.token()));
    let email: String = match redis.secure_get::<String>(&key).await {
        Ok(Some(email)) => email,
        other => {
            eprintln!("Failed to get email from Redis: {other:?}");
            return Err(ResetPasswordError::UnknownToken);
        }
    };
    let user_id = match get_user_id(smart_db, &email).await {
        Ok(user_id) => user_id,
        Err(e) => {
            eprintln!("Failed to get user ID: {e:?}");
            return Err(e);
        }
    };

    let reversed_key = redis_key(&format!("{email}/forgot_password_token"));
    if let Err(e) = redis.delete(&reversed_key).await {
        eprintln!("Failed to delete reversed key: {e:?}");
        return Err(ResetPasswordError::RedisError);
    }
    if let Err(e) = redis.delete(&key).await {
        eprintln!("Failed to delete key: {e:?}");
        return Err(ResetPasswordError::RedisError);
    }

    reset_pwd(smart_db, view.new_password(), user_id).await?;

    // Sessions opened with the old password end here, in every API (MAIR-264). The session opened
    // below is the only one left.
    revoke_all_user_sessions(&state, user_id)
        .await
        .map_err(|e| {
            eprintln!("Failed to revoke the previous sessions: {e}");
            ResetPasswordError::DatabaseError
        })?;

    match generate_session(user_id, &view.device_info(), ip_adress, state).await {
        Ok((jwt, refresh_token)) => Ok((jwt, refresh_token)),
        Err(e) => {
            eprintln!("Failed to generate session: {e:?}");
            Err(ResetPasswordError::TokenGenerationError)
        }
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Réinitialiser un mot de passe avec un jeton",
    description = "Consomme le jeton envoyé par `POST /api/v1/auth/forgot_password`, enregistre le \
                   nouveau mot de passe et ouvre immédiatement une session : la réponse contient un \
                   JWT dans l'en-tête `Authorization` et un jeton de rafraîchissement dans le corps, \
                   comme un login. Un second appel avec le même jeton répond `401`.\n\n\
                   Route publique : le `JwtMiddleware` laisse passer tout ce qui est sous `/auth`.",
    request_body(
        content = ResetPasswordView,
        description = "Jeton reçu par e-mail, nouveau mot de passe et description de l'appareil.",
        example = json!({
            "token": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35",
            "new_password": "NouveauMotDePasse!123",
            "device_info": "Chrome 140 sur Windows 11"
        })
    ),
    responses(
        (
            status = 200,
            description = "Mot de passe réinitialisé et session ouverte.",
            body = ResetPasswordResponseView,
            headers(
                ("Authorization" = String, description = "JWT d'accès, préfixé par `Bearer `.")
            ),
            example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
        ),
        (
            status = 400,
            description = "Corps JSON malformé ou champ obligatoire absent.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `token`")
        ),
        (
            status = 401,
            description = "Jeton inconnu, expiré ou déjà consommé.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown token")
        ),
        (
            status = 500,
            description = "Erreur de base de données, de Redis, ou échec de génération du JWT.",
            body = String,
            content_type = "text/plain",
            example = json!("Internal server error")
        )
    ),
    tag = "Auth",
)]
#[post("/reset_password")]
pub async fn reset_password(
    state: web::Data<AppState>,
    body: web::Json<ResetPasswordView>,
    conn: ConnectionInfo,
) -> Result<impl Responder, ResetPasswordError> {
    let ip_str = conn.realip_remote_addr().unwrap_or("unknown").to_string();
    let ip_address = ip_str
        .parse::<std::net::IpAddr>()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
    let (jwt, refresh_token) = reset_password_trigger(state, body.into_inner(), ip_address).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(ResetPasswordResponseView::from(refresh_token)))
}
