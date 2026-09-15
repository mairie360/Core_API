use crate::database::admin::list_users::{
    AdminCountUsersQueryView, AdminListUsersQueryView, AdminUserRow,
};
use crate::endpoints::v1::admin::users::get::view::{
    AdminListUsersQuery, AdminListUsersResultView, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
};
use actix_web::{error::ResponseError, get, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ListUsersError {
    InvalidPagination,
    DatabaseError,
}

impl std::fmt::Display for ListUsersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListUsersError::InvalidPagination => write!(f, "Invalid pagination"),
            ListUsersError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for ListUsersError {
    fn status_code(&self) -> StatusCode {
        match self {
            ListUsersError::InvalidPagination => StatusCode::BAD_REQUEST,
            ListUsersError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn list_users(
    state: web::Data<AppState>,
    query: AdminListUsersQuery,
) -> Result<AdminListUsersResultView, ListUsersError> {
    let page = query.page().unwrap_or(1);
    let page_size = query.page_size().unwrap_or(DEFAULT_PAGE_SIZE);
    if page == 0 || page_size == 0 || page_size > MAX_PAGE_SIZE {
        return Err(ListUsersError::InvalidPagination);
    }

    let smart_db = state.get_smart_db();
    let users_view =
        AdminListUsersQueryView::new(query.search(), query.group_id(), page, page_size);
    let count_view = AdminCountUsersQueryView::new(query.search(), query.group_id());
    let (users, total) = futures_util::try_join!(
        smart_db.fetch_all::<AdminUserRow, _>(&users_view),
        smart_db.fetch_scalar::<i64, _>(&count_view),
    )
    .map_err(|error| {
        eprintln!("{:?}", error);
        ListUsersError::DatabaseError
    })?;

    let total = total.max(0) as u64;
    Ok(AdminListUsersResultView {
        users,
        page,
        page_size,
        total,
        total_pages: total.div_ceil(page_size),
    })
}

#[utoipa::path(
    get,
    path = "",
    params(AdminListUsersQuery),
    responses(
        (status = 200, description = "Users retrieved successfully", body = AdminListUsersResultView),
        (status = 400, description = "Invalid pagination"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn admin_list_users(
    state: web::Data<AppState>,
    query: web::Query<AdminListUsersQuery>,
) -> Result<impl Responder, ListUsersError> {
    let result = list_users(state, query.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
