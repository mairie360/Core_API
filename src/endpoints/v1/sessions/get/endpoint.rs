use crate::database::sessions::get_active_sessions::GetActiveSessionsQueryView;
use crate::endpoints::v1::sessions::get::response_view::GetSessionsResultView;
use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetError {
    DatabaseError,
}

impl std::fmt::Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_info(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<GetSessionsResultView, GetError> {
    let user_id = user.id;

    // Le cache Redis est désormais géré par `SmartDatabase` (cache-aside), via
    // `GetActiveSessionsQueryView::cache_key`.
    let query_result: Vec<crate::database::sessions::Session> = state
        .get_smart_db()
        .fetch_all(&GetActiveSessionsQueryView::new(user_id))
        .await
        .map_err(|_| GetError::DatabaseError)?;

    Ok(GetSessionsResultView::new(
        query_result.into_iter().map(|s| s.into()).collect(),
    ))
}

#[utoipa::path(
    get,
    path = "",
    summary = "Lister ses sessions actives",
    description = "Renvoie les sessions encore valides de l'utilisateur porté par le JWT : celles \
                   qui ne sont ni expirées ni révoquées. Pour l'historique complet, révocations \
                   comprises, voir `GET /api/v1/sessions/history`.\n\n\
                   Le champ `revoked_at` est donc toujours `null` ici. La liste peut être vide si \
                   la seule session en cours vient d'être révoquée.",
    responses(
        (
            status = 200,
            description = "Sessions actives de l'utilisateur connecté.",
            body = GetSessionsResultView,
            example = json!({
                "sessions": [
                    {
                        "id": "1",
                        "device_info": "Chrome 140 sur Windows 11",
                        "ip_address": "192.168.1.24",
                        "created_at": "2026-09-16 08:42:11 UTC",
                        "expires_at": "2026-09-23 08:42:11 UTC",
                        "revoked_at": null
                    }
                ]
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
            description = "Erreur de base de données lors de la lecture des sessions.",
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
#[get("/")]
pub async fn get_active_sessions(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, GetError> {
    let response = get_user_info(user, state).await?;
    Ok(HttpResponse::Ok().json(response))
}
