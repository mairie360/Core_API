use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::query_views::IsSessionTokenValidQueryView;
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::state::AppState;

use crate::endpoints::v1::sessions::refresh::request_view::RefreshRequestView;
use mairie360_api_lib::security::AuthenticatedUser;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq)]
enum RefreshError {
    DatabaseError,
    InvalidToken,
}

impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefreshError::InvalidToken => write!(f, "Session not found"),
            RefreshError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for RefreshError {
    fn status_code(&self) -> StatusCode {
        match self {
            RefreshError::InvalidToken => StatusCode::UNAUTHORIZED,
            RefreshError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn refresh_request(
    user: AuthenticatedUser,
    ip_adress: IpAddr,
    view: RefreshRequestView,
    state: web::Data<AppState>,
) -> Result<String, RefreshError> {
    let user_id = user.id;

    let db_view = IsSessionTokenValidQueryView::new(user_id, view.refresh_token(), ip_adress);

    let is_valid: Result<bool, _> = state.get_smart_db().fetch_scalar(&db_view).await;

    match is_valid {
        // TODO: cf. login/endpoint.rs::generate_session — rôle non encore exploité par la lib.
        Ok(true) => generate_jwt(&user_id.to_string(), "").map_err(|_| RefreshError::DatabaseError),
        Ok(false) => Err(RefreshError::InvalidToken),
        Err(_) => Err(RefreshError::DatabaseError),
    }
}

#[utoipa::path(
    post,
    path = "refresh",
    summary = "Renouveler son JWT",
    description = "Échange un jeton de rafraîchissement encore valide contre un nouveau JWT, sans \
                   redemander le mot de passe. Le nouveau JWT est renvoyé dans l'en-tête \
                   `Authorization` ; le corps n'est qu'un message de confirmation en texte brut.\n\n\
                   Le JWT courant reste exigé dans l'en-tête `Authorization` de la requête : \
                   appeler cet endpoint avec un JWT déjà expiré échoue en `401` au niveau de \
                   l'intergiciel, avant d'atteindre le handler. Il faut donc rafraîchir **avant** \
                   l'expiration, sinon une reconnexion complète est nécessaire.\n\n\
                   Le jeton de rafraîchissement est validé pour le couple (utilisateur, adresse IP \
                   d'origine) : un changement de réseau invalide la session.\n\n\
                   Le jeton de rafraîchissement n'est pas tourné : le même reste utilisable \
                   jusqu'à sa révocation ou son expiration.",
    request_body(
        content = RefreshRequestView,
        description = "Jeton de rafraîchissement obtenu au login.",
        example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
    ),
    responses(
        (
            status = 200,
            description = "Nouveau JWT émis, renvoyé dans l'en-tête `Authorization`.",
            body = String,
            content_type = "text/plain",
            headers(
                ("Authorization" = String, description = "Nouveau JWT d'accès, préfixé par `Bearer `.")
            ),
            example = json!("JWT refreshed successfully")
        ),
        (
            status = 400,
            description = "Corps JSON malformé ou champ `refresh_token` absent.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `refresh_token`")
        ),
        (
            status = 401,
            description = "JWT de la requête absent, invalide ou expiré, ou jeton de rafraîchissement inconnu, révoqué, expiré, ou présenté depuis une autre adresse IP.",
            body = String,
            content_type = "text/plain",
            example = json!("Session not found")
        ),
        (
            status = 500,
            description = "Erreur de base de données, ou échec de génération du nouveau JWT.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Sessions",
    security(
        ("jwt" = [])
    )
)]
#[post("/refresh")]
pub async fn refresh(
    user: AuthenticatedUser,
    body: web::Json<RefreshRequestView>,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RefreshError> {
    let view = body.into_inner();

    let ip_adress = request
        .connection_info()
        .realip_remote_addr()
        .unwrap()
        .parse()
        .unwrap();

    let new_jwt = refresh_request(user, ip_adress, view, state).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {}", new_jwt)))
        .body("JWT refreshed successfully"))
}
