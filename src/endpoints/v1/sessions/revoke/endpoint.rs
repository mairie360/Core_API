use std::net::IpAddr;

use crate::database::sessions::revoke_session_by_token::RevokeSessionByTokenQueryView;
use crate::endpoints::v1::sessions::revoke::request_view::RevokeRequestView;
use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};

use mairie360_api_lib::database::query_views::IsSessionTokenValidQueryView;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum RevokeError {
    InvalidToken,
    DatabaseError,
}

impl std::fmt::Display for RevokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RevokeError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            RevokeError::InvalidToken => {
                write!(f, "Session not found.")
            }
        }
    }
}

impl ResponseError for RevokeError {
    fn status_code(&self) -> StatusCode {
        match self {
            RevokeError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            RevokeError::InvalidToken => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn revoke_request(
    user: AuthenticatedUser,
    view: RevokeRequestView,
    state: web::Data<AppState>,
    ip_adress: IpAddr,
) -> Result<(), RevokeError> {
    let user_id = user.id;

    let db_view = IsSessionTokenValidQueryView::new(user_id, view.refresh_token(), ip_adress);

    let is_valid: Result<bool, _> = state.get_smart_db().fetch_scalar(&db_view).await;

    let db_view = match is_valid {
        Ok(true) => RevokeSessionByTokenQueryView::new(user_id, &view.refresh_token()),
        Ok(false) => return Err(RevokeError::InvalidToken),
        Err(_) => return Err(RevokeError::DatabaseError),
    };

    match state.get_smart_db().execute(db_view).await {
        Ok(_) => Ok(()),
        Err(_) => Err(RevokeError::DatabaseError),
    }
}

#[utoipa::path(
    post,
    path = "revoke",
    summary = "Révoquer une de ses sessions",
    description = "Révoque la session identifiée par son jeton de rafraîchissement : c'est la \
                   déconnexion. Le jeton ne peut plus servir à `POST /api/v1/sessions/refresh`, et \
                   la session bascule dans l'historique avec un `revoked_at` renseigné.\n\n\
                   Un utilisateur ne peut révoquer que ses propres sessions : le jeton est validé \
                   pour le couple (utilisateur du JWT, adresse IP d'origine) avant la révocation. \
                   Révoquer la session courante n'invalide pas immédiatement le JWT déjà émis, qui \
                   reste refusé à la prochaine vérification de session.",
    request_body(
        content = RevokeRequestView,
        description = "Jeton de rafraîchissement de la session à révoquer.",
        example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
    ),
    responses(
        (
            status = 200,
            description = "Session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Session revoked successfully")
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
            description = "JWT de la requête absent, invalide ou expiré, ou jeton de rafraîchissement inconnu, déjà révoqué, ou n'appartenant pas à l'utilisateur connecté.",
            body = String,
            content_type = "text/plain",
            example = json!("Session not found.")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la révocation.",
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
#[post("/revoke")]
pub async fn revoke(
    user: AuthenticatedUser,
    body: web::Json<RevokeRequestView>,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RevokeError> {
    let view = body.into_inner();

    let ip_adress = request
        .connection_info()
        .realip_remote_addr()
        .unwrap()
        .parse()
        .unwrap();

    revoke_request(user, view, state, ip_adress)
        .await
        .map(|_| HttpResponse::Ok().body("Session revoked successfully"))
}
