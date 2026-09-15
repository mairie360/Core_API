use crate::database::users::list_directory::{DirectoryUser, ListDirectoryUsersQueryView};
use crate::endpoints::v1::user::get::view::{
    parse_id_list, DirectoryUsersQuery, DirectoryUsersResultView, MAX_DIRECTORY_LIMIT,
};
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
            DirectoryError::BadRequest => write!(f, "Bad request."),
            DirectoryError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for DirectoryError {
    fn status_code(&self) -> StatusCode {
        match self {
            DirectoryError::BadRequest => StatusCode::BAD_REQUEST,
            DirectoryError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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

    let users: Vec<DirectoryUser> = state
        .get_smart_db()
        .fetch_all(&ListDirectoryUsersQueryView::new(
            query.search(),
            &ids,
            &group_ids,
            limit,
        ))
        .await
        .map_err(|error| {
            eprintln!("{:?}", error);
            DirectoryError::DatabaseError
        })?;

    Ok(DirectoryUsersResultView { users })
}

#[utoipa::path(
    get,
    path = "",
    params(DirectoryUsersQuery),
    responses(
        (status = 200, description = "Non-archived users", body = DirectoryUsersResultView),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
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
    query: web::Query<DirectoryUsersQuery>,
) -> Result<impl Responder, DirectoryError> {
    let result = trigger_list_directory_users(state, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
