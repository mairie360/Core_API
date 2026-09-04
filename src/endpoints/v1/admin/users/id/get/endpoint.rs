use crate::{
    database::{
        admin::get_user::view::{
            AdminGetUserQueryResultView, AdminGetUserQueryView, RoleQueryResult, User,
        },
        groups::get_user_groups::GetUserGroupsQuerView,
        roles::get_roles_by_id::{GetRolesByIdQueryView, Role},
        sessions::{get_sessions_by_user::GetSessionsByUserQueryView, Session},
        users::get_roles::GetUserRolesQueryView,
    },
    endpoints::v1::admin::users::id::get::view::GetUserResultView,
};
use actix_web::{error::ResponseError, get, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetUserError {
    UnknownUser,
}

impl std::fmt::Display for GetUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUserError::UnknownUser => write!(f, "Unknown user"),
        }
    }
}

impl ResponseError for GetUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetUserError::UnknownUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user(
    state: web::Data<AppState>,
    user_id: u64,
) -> Result<GetUserResultView, GetUserError> {
    let smart_db = state.get_smart_db();

    let user: User = smart_db
        .fetch_one(&AdminGetUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;

    let roles_id: Vec<i32> = smart_db
        .fetch_all(&GetUserRolesQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;
    let roles_result: Vec<Role> = smart_db
        .fetch_all(&GetRolesByIdQueryView::new(roles_id.clone()))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;
    let mut roles: Vec<RoleQueryResult> = Vec::new();
    for i in 0..roles_result.len() {
        roles.push(RoleQueryResult::new(
            roles_id[i],
            roles_result[i].name(),
            roles_result[i].description(),
        ));
    }

    let sessions: Vec<Session> = smart_db
        .fetch_all(&GetSessionsByUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;

    let groups = smart_db
        .fetch_all(&GetUserGroupsQuerView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;

    let result = AdminGetUserQueryResultView::new(user, roles, groups, sessions);

    Ok(result.into())
}

#[utoipa::path(
    get,
    path = "",
    params(
        ("userId" = u64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "User retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Unknown user"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[get("/")]
pub async fn admin_get_user(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, GetUserError> {
    let result = get_user(state, path.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
