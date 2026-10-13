use crate::database::users::list_directory::{DirectoryUser, ListDirectoryUsersQueryView};
use crate::endpoints::v1::user::get::view::{
    parse_id_list, DirectoryUsersQuery, DirectoryUsersResultView, MAX_DIRECTORY_LIMIT,
};
use crate::endpoints::validation::ValidatedQuery;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum DirectoryError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for DirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest => write!(f, "Bad request."),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for DirectoryError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_list_directory_users(
    state: web::Data<AppState>,
    query: DirectoryUsersQuery,
) -> Result<DirectoryUsersResultView, DirectoryError> {
    let ids = parse_id_list(query.ids()).ok_or(DirectoryError::BadRequest)?;
    let group_ids = parse_id_list(query.group_ids()).ok_or(DirectoryError::BadRequest)?;
    let limit = query.limit().unwrap_or(MAX_DIRECTORY_LIMIT);
    if limit == 0 || limit > MAX_DIRECTORY_LIMIT {
        return Err(DirectoryError::BadRequest);
    }

    let view = ListDirectoryUsersQueryView::new(query.search(), &ids, &group_ids, limit);
    let key = ListDirectoryUsersQueryView::cache_key(query.search(), &ids, &group_ids, limit);
    let smart_db = state.get_smart_db();
    let users: Vec<DirectoryUser> = smart_db
        .cached(&key, async || smart_db.fetch_all(&view).await)
        .await
        .map_err(|error| {
            tracing::error!("{error:?}");
            DirectoryError::DatabaseError
        })?;

    Ok(DirectoryUsersResultView { users })
}

#[utoipa::path(
    get,
    path = "",
    summary = "Search the user directory",
    description = "Returns the non-archived users, with their roles and the ids of the groups \
                   they belong to. Feeds the user pickers of the other modules (task \
                   assignment, event invitation, adding someone to a conversation).\n\n\
                   The filters add up: a user must match **every** filter given. Without any \
                   filter, the first 1000 non-archived users are returned.\n\n\
                   Deliberately restricted view: no phone, status nor date. For a user's full \
                   record, call `GET /api/v1/user/{id}/`.\n\n\
                   **Cache (MAIR-524):** the page does not depend on the caller, so it is cached \
                   in Redis for 60 seconds, one entry per set of filters shared by every caller \
                   (`search` trimmed, `ids` / `group_ids` compared as sets). Every write through \
                   Core_API that changes what the directory shows invalidates it at once: \
                   account creation, admin patch and archive, `PATCH /api/v1/user/me/`, role \
                   grant/revoke, role rename/replace/delete, group creation and deletion, group \
                   member add/remove. A change made outside Core_API (direct SQL) shows up at \
                   the latest after the 60 seconds. When Redis is unreachable the page is read \
                   from Postgres, never failed.",
    params(DirectoryUsersQuery),
    responses(
        (
            status = 200,
            description = "Users matching the filters, possibly served from the 60-second cache. The list is empty if none matches.",
            body = DirectoryUsersResultView,
            example = json!({
                "users": [
                    {
                        "id": 1,
                        "first_name": "Jean",
                        "last_name": "Dupont",
                        "email": "jean.dupont@mairie360.fr",
                        "roles": ["agent"],
                        "group_ids": [3, 7]
                    },
                    {
                        "id": 2,
                        "first_name": "Amina",
                        "last_name": "Bensaïd",
                        "email": "amina.bensaid@mairie360.fr",
                        "roles": ["admin", "agent"],
                        "group_ids": []
                    }
                ]
            })
        ),
        (
            status = 400,
            description = "`ids` or `group_ids` is not a comma-separated list of strictly positive integers, `limit` is outside 1–1000, or `search`, `ids` or `group_ids` is longer than 255 characters or contains a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `search`: must not contain control characters")
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
            description = "Database error while reading the directory (on a cache miss).",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn list_directory_users(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    query: ValidatedQuery<DirectoryUsersQuery>,
) -> Result<impl Responder, DirectoryError> {
    let result = trigger_list_directory_users(state, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
