use crate::database::sessions::get_sessions_by_user::GetSessionsByUserQueryView;
use crate::endpoints::v1::sessions::history::response_view::HistoryResponseView;
use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum HistoryError {
    DatabaseError,
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for HistoryError {
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
) -> Result<HistoryResponseView, HistoryError> {
    let user_id = user.id;

    // Le cache Redis est désormais géré par `SmartDatabase` (cache-aside), via
    // `GetSessionsByUserQueryView::cache_key`.
    let query_result: Vec<crate::database::sessions::Session> = state
        .get_smart_db()
        .fetch_all(&GetSessionsByUserQueryView::new(user_id))
        .await
        .map_err(|_| HistoryError::DatabaseError)?;

    Ok(HistoryResponseView::new(
        query_result
            .into_iter()
            .map(std::convert::Into::into)
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "history",
    summary = "Consulter l'historique de ses sessions",
    description = "Renvoie **toutes** les sessions de l'utilisateur porté par le JWT, y compris \
                   celles qui sont expirées ou révoquées, afin qu'il puisse repérer une connexion \
                   qu'il ne reconnaît pas. Pour ne voir que les sessions utilisables, voir \
                   `GET /api/v1/sessions/`.\n\n\
                   `revoked_at` est `null` tant que la session n'a pas été révoquée.",
    responses(
        (
            status = 200,
            description = "Historique complet des sessions de l'utilisateur connecté.",
            body = HistoryResponseView,
            example = json!({
                "sessions": [
                    {
                        "id": "1",
                        "device_info": "Chrome 140 sur Windows 11",
                        "ip_address": "192.168.1.24",
                        "created_at": "2026-09-16 08:42:11 UTC",
                        "expires_at": "2026-09-23 08:42:11 UTC",
                        "revoked_at": null
                    },
                    {
                        "id": "2",
                        "device_info": "Safari 18 sur iPhone",
                        "ip_address": "10.0.0.7",
                        "created_at": "2026-09-02 19:03:55 UTC",
                        "expires_at": "2026-09-09 19:03:55 UTC",
                        "revoked_at": "2026-09-04 07:15:02 UTC"
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
#[get("/history")]
pub async fn history(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, HistoryError> {
    let result = get_user_info(user, state).await?;
    Ok(HttpResponse::Ok().json(result))
}
