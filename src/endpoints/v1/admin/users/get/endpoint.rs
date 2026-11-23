use crate::database::admin::list_users::{
    AdminCountUsersQueryView, AdminListUsersQueryView, AdminUserRow, AdminUsersFilter,
};
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::v1::admin::users::get::view::{
    AdminListUsersQuery, AdminListUsersResultView, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
};
use crate::endpoints::validation::ValidatedQuery;
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
            Self::InvalidPagination => write!(f, "Invalid pagination"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for ListUsersError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidPagination => StatusCode::BAD_REQUEST,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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
    let filter = AdminUsersFilter {
        search: query.search().map(str::to_string),
        group_id: query.group_id(),
        account_status: query.account_status().map(str::to_string),
        role_id: query.role_id(),
    };
    let users_view =
        AdminListUsersQueryView::filtered(&filter, query.descending(), page, page_size);
    let count_view = AdminCountUsersQueryView::filtered(&filter);
    let (users, total) = futures_util::try_join!(
        smart_db.fetch_all::<AdminUserRow, _>(&users_view),
        smart_db.fetch_scalar::<i64, _>(&count_view),
    )
    .map_err(|error| {
        tracing::error!("{error:?}");
        ListUsersError::DatabaseError
    })?;

    let total = u64::try_from(total).unwrap_or_default();
    Ok(AdminListUsersResultView {
        users: users
            .into_iter()
            .map(AdminUserRow::with_e164_phone)
            .collect(),
        page,
        page_size,
        total,
        total_pages: total.div_ceil(page_size),
    })
}

#[utoipa::path(
    get,
    path = "",
    summary = "List the users (administration)",
    description = "Paginated administration view of the user accounts, archived ones included, \
                   with phone, status and roles, the number of active sessions, the last sign-in and the \
                   second factor (methods and last verification, never a secret). Reserved to \
                   administrators. `status` is `suspended` for an account an administrator \
                   suspended (MAIR-904).\n\n\
                   Not to be confused with `GET /api/v1/user/`, the directory open to everyone: that one \
                   hides the archived accounts, is not paginated and returns neither phone nor status.\n\n\
                   Filters (MAIR-904): `search`, `group_id`, `account_status` (the state the \
                   administration shows: active, invited, inactive, suspended) and `role_id` \
                   combine; `sort=-name` reverses the order by name.\n\n\
                   `total` counts the users matching the filters, across all \
                   pages; `total_pages` follows from it. A `page` past `total_pages` \
                   returns an empty list, not an error.",
    params(AdminListUsersQuery),
    responses(
        (
            status = 200,
            description = "Page of users matching the filters.",
            body = AdminListUsersResultView,
            example = json!({
                "users": [
                    {
                        "id": 42,
                        "first_name": "Jean",
                        "last_name": "Dupont",
                        "email": "jean.dupont@mairie360.fr",
                        "phone_number": "+33612345678",
                        "phone_country": "FR",
                        "status": "active",
                        "account_status": "active",
                        "is_archived": false,
                        "role": { "id": 2, "name": "agent" },
                        "roles": [{ "id": 2, "name": "agent" }],
                        "active_sessions": 2,
                        "last_login_at": "2026-09-16T08:42:11Z",
                        "two_factor": { "methods": ["totp", "backup_codes"], "last_verified_at": "2026-09-16T08:42:30Z" }
                    }
                ],
                "page": 1,
                "page_size": 20,
                "total": 137,
                "total_pages": 7
            })
        ),
        (
            status = 400,
            description = "`page` is 0, `page_size` is 0 or above 500, `search` is longer than 255 characters or contains a control character, `account_status` is not `active`, `invited`, `inactive` or `suspended`, or `sort` is not `name` or `-name`.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid pagination")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 500,
            description = "Database error while reading or counting.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn admin_list_users(
    _: AdminUser,
    state: web::Data<AppState>,
    query: ValidatedQuery<AdminListUsersQuery>,
) -> Result<impl Responder, ListUsersError> {
    let result = list_users(state, query.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
