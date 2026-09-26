use crate::database::auth::is_first_time::IsFirstTimeQueryView;
use crate::database::auth::unset_first_connection::UnsetFirstConnectionQueryView;
use crate::endpoints::v1::auth::force_change_password::view::ForceChangePasswordView;
use crate::session_revocation::revoke_all_user_sessions;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::password::hash_password;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ForceChanhePasswordError {
    DatabaseError,
    Forbidden,
    Unauthorized,
}

impl std::fmt::Display for ForceChanhePasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::Forbidden => {
                write!(f, "Unknown user token")
            }
            Self::Unauthorized => {
                write!(f, "Unauthorized")
            }
        }
    }
}

impl ResponseError for ForceChanhePasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_id(state: &AppState, token: &str) -> Option<u64> {
    match state
        .get_redis()
        .secure_get::<String>(&format!("{token}/first_connection_id"))
        .await
    {
        Ok(Some(id)) => id.parse().ok(),
        _ => None,
    }
}

async fn is_first_time(smart_db: &SmartDatabase, user_id: u64) -> bool {
    smart_db
        .fetch_scalar(&IsFirstTimeQueryView::new(user_id))
        .await
        .unwrap_or(false)
}

async fn change_password(
    smart_db: &SmartDatabase,
    user_id: u64,
    new_password: &str,
) -> Result<(), ForceChanhePasswordError> {
    let hashed_password = hash_password(new_password).map_err(|e| {
        eprintln!("Password hashing error: {e}");
        ForceChanhePasswordError::DatabaseError
    })?;
    smart_db
        .execute(UnsetFirstConnectionQueryView::new(
            user_id,
            &hashed_password,
        ))
        .await
        .map_err(|_| ForceChanhePasswordError::DatabaseError)
}

async fn force_change_password_trigger(
    state: web::Data<AppState>,
    view: ForceChangePasswordView,
) -> Result<(), ForceChanhePasswordError> {
    let smart_db = state.get_smart_db();

    let Some(user_id) = get_user_id(&state, view.token()).await else {
        return Err(ForceChanhePasswordError::Forbidden);
    };

    if !is_first_time(smart_db, user_id).await {
        return Err(ForceChanhePasswordError::Unauthorized);
    }

    change_password(smart_db, user_id, view.new_password()).await?;
    // A password change ends the sessions opened with the previous password, in every API
    // (MAIR-264). Usually none: a first-connection login opens no session.
    revoke_all_user_sessions(&state, user_id)
        .await
        .map_err(|_| ForceChanhePasswordError::DatabaseError)?;
    consume_first_connection_token(&state, view.token(), user_id).await;

    Ok(())
}

/// Le jeton de première connexion est à usage unique : une fois le mot de passe enregistré, les deux
/// clés posées au login sont supprimées. Un échec Redis n'annule pas le changement déjà persisté.
async fn consume_first_connection_token(state: &AppState, token: &str, user_id: u64) {
    let redis = state.get_redis();
    for key in [
        format!("{}/first_connection_id", token),
        format!("{}/first_connection_token", user_id),
    ] {
        if let Err(error) = redis.secure_delete(&key).await {
            eprintln!(
                "Suppression du jeton de première connexion impossible : {:?}",
                error
            );
        }
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Changer le mot de passe imposé à la première connexion",
    description = "Termine le parcours de première connexion : `POST /api/v1/auth/login` répond \
                   `412` avec un jeton à usage unique tant que l'utilisateur n'a pas choisi son \
                   propre mot de passe. Cet endpoint consomme ce jeton et enregistre le nouveau \
                   mot de passe ; l'utilisateur peut ensuite se connecter normalement.\n\n\
                   Contrairement à `reset_password`, aucune session n'est ouverte ici : la réponse \
                   a un corps vide et il faut rappeler `POST /api/v1/auth/login`.\n\n\
                   Route publique : le `JwtMiddleware` laisse passer tout ce qui est sous `/auth`.",
    request_body(
        content = ForceChangePasswordView,
        description = "Jeton de première connexion renvoyé par le `412` du login, et nouveau mot de passe.",
        example = json!({
            "token": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35",
            "new_password": "NouveauMotDePasse!123"
        })
    ),
    responses(
        (
            status = 200,
            description = "Mot de passe enregistré et jeton de première connexion consommé. Corps vide.",
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
            description = "Le jeton est valide mais le compte n'est plus en première connexion : le mot de passe a déjà été changé.",
            body = String,
            content_type = "text/plain",
            example = json!("Unauthorized")
        ),
        (
            status = 403,
            description = "Jeton de première connexion inconnu, expiré ou déjà consommé.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user token")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de l'enregistrement du mot de passe.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Auth"
)]
#[post("/force_change_password")]
pub async fn force_change_password(
    state: web::Data<AppState>,
    body: web::Json<ForceChangePasswordView>,
) -> Result<impl Responder, ForceChanhePasswordError> {
    force_change_password_trigger(state, body.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
