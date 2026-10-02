use crate::database::sessions::get_sessions_by_user::GetSessionsByUserQueryView;
use crate::endpoints::db_error;
use crate::endpoints::pagination::PageQuery;
use crate::endpoints::v1::sessions::history::response_view::HistoryResponseView;
use crate::endpoints::validation::ValidatedQuery;
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
    page: PageQuery,
) -> Result<HistoryResponseView, HistoryError> {
    let user_id = user.id;

    // Not cached (MAIR-267): see the query view.
    let query_result: Vec<crate::database::sessions::Session> = state
        .get_smart_db()
        .fetch_all(&GetSessionsByUserQueryView::page(user_id, page))
        .await
        .map_err(|e| {
            db_error::log("session history", &e);
            HistoryError::DatabaseError
        })?;

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
    description = "Returns the sessions of the JWT's user, expired and revoked ones included, \
                   newest first, so that they can spot a login they do not recognise. To see the \
                   usable sessions only, use `GET /api/v1/sessions/`.\n\n\
                   The history is paginated (MAIR-425): `limit` sessions (100 by default, 500 at \
                   most) from `offset`. A page shorter than `limit` is the last one.\n\n\
                   `revoked_at` is `null` until the session is revoked.",
    params(PageQuery),
    responses(
        (
            status = 200,
            description = "One page of the connected user's sessions, newest first.",
            body = HistoryResponseView,
            example = json!({
                "sessions": [
                    {
                        "id": "1",
                        "device_info": "Chrome 140 sur Windows 11",
                        "ip_address": "203.0.113.24",
                        "created_at": "2026-09-16 08:42:11 UTC",
                        "expires_at": "2026-09-23 08:42:11 UTC",
                        "revoked_at": null
                    },
                    {
                        "id": "2",
                        "device_info": "Safari 18 sur iPhone",
                        "ip_address": "198.51.100.7",
                        "created_at": "2026-09-02 19:03:55 UTC",
                        "expires_at": "2026-09-09 19:03:55 UTC",
                        "revoked_at": "2026-09-04 07:15:02 UTC"
                    }
                ]
            })
        ),
        (
            status = 400,
            description = "`limit` outside 1 to 500, or `offset` outside 0 to 1000000.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `limit`: must be between 1 and 500")
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
    page: ValidatedQuery<PageQuery>,
) -> Result<impl Responder, HistoryError> {
    let result = get_user_info(user, state, page.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
