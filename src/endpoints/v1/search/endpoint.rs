use crate::database::search::view::{GlobalSearchItem, GlobalSearchQueryView};
use crate::endpoints::v1::search::view::{like_escape, GlobalSearchResultView, SearchQuery};
use crate::endpoints::v1::user::directory_access::may_read_directory;
use crate::endpoints::validation::ValidatedQuery;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    DatabaseError,
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred while accessing the database.")
    }
}

impl ResponseError for SearchError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_global_search(
    state: web::Data<AppState>,
    caller_id: u64,
    query: SearchQuery,
) -> Result<GlobalSearchResultView, SearchError> {
    let smart_db = state.get_smart_db();
    let database_error = |error| {
        tracing::error!("{error:?}");
        SearchError::DatabaseError
    };
    // The directory rules apply: a caller the directory is closed to finds no agent.
    if !may_read_directory(smart_db, caller_id)
        .await
        .map_err(database_error)?
    {
        return Ok(GlobalSearchResultView { items: Vec::new() });
    }
    let view = GlobalSearchQueryView::new(&like_escape(query.term()), query.limit());
    let items: Vec<GlobalSearchItem> = smart_db.fetch_all(&view).await.map_err(database_error)?;
    Ok(GlobalSearchResultView { items })
}

#[utoipa::path(
    get,
    path = "",
    summary = "Search the agents (global search)",
    description = "Part of the global search of the shared header: every business API answers \
                   `GET /api/v1/search` with the same shape, and each BFF merges them. Core \
                   returns the **agents** (`kind = agent`): the non-archived users whose first \
                   and last names (both orders) or e-mail contain `q`, case-insensitive. Names \
                   starting with `q` come first, then the most recently updated accounts.\n\n\
                   The directory rules apply (MAIR-288): when `DIRECTORY_GUEST_ACCESS=none` and \
                   the caller only holds the Guest role, the list is empty (never `403`, so the \
                   BFF does not report Core as unavailable).\n\n\
                   Not cached: the result depends on the term.",
    params(SearchQuery),
    responses(
        (
            status = 200,
            description = "At most `limit` agents, best match first. Empty when nothing matches.",
            body = GlobalSearchResultView,
            example = json!({
                "items": [
                    {
                        "kind": "agent",
                        "id": 12,
                        "title": "Jean Dupont",
                        "excerpt": "jean.dupont@mairie360.fr",
                        "parent_id": null,
                        "updated_at": "2026-11-20T09:30:00Z"
                    }
                ]
            })
        ),
        (
            status = 400,
            description = "`q` is missing, shorter than 2 or longer than 100 characters once trimmed, or holds a control character; or `limit` is outside 1–20.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `q`: must be at least 2 characters")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error while searching.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Search",
    security(
        ("jwt" = [])
    )
)]
#[get("")]
pub async fn global_search(
    caller: AuthenticatedUser,
    state: web::Data<AppState>,
    query: ValidatedQuery<SearchQuery>,
) -> Result<impl Responder, SearchError> {
    let result = trigger_global_search(state, caller.id, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
