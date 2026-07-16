use crate::database::groups::get_user_groups::{get_user_groups, GetUserGroupsQuerView};
use crate::database::roles::get_roles_by_id::{get_roles_by_id_query, GetRolesByIdQueryView};
use crate::database::users::get_roles::{get_user_roles_query, GetUserRolesQueryView};
use crate::database::users::get_user_by_id::{get_user_by_id_query, GetUserByIdQueryView};
use crate::endpoints::v1::user::id::get::view::GetUserResponseView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetUserError {
    DatabaseError,
    UnknownUser,
}

impl std::fmt::Display for GetUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUserError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            GetUserError::UnknownUser => {
                write!(f, "User not found.")
            }
        }
    }
}

impl ResponseError for GetUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetUserError::UnknownUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_get_user(
    state: web::Data<AppState>,
    id: u64,
) -> Result<GetUserResponseView, GetUserError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetUserError::DatabaseError),
    };

    let view = GetUserByIdQueryView::new(id);
    let result = get_user_by_id_query(view, pool.clone())
        .await
        .map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            GetUserError::UnknownUser
        })?;
    let view = GetUserGroupsQuerView::new(id);
    let groups = get_user_groups(view, pool.clone()).await.map_err(|e| {
        eprintln!("Login DB Error: {}", e);
        GetUserError::DatabaseError
    })?;
    let role = GetUserRolesQueryView::new(id);
    let role_id = get_user_roles_query(role, pool.clone())
        .await
        .map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            GetUserError::DatabaseError
        })?;
    let view = GetRolesByIdQueryView::new(role_id);
    let role = get_roles_by_id_query(view, pool).await.map_err(|e| {
        eprintln!("Login DB Error: {}", e);
        GetUserError::DatabaseError
    })?;

    Ok(GetUserResponseView::new(
        result.first_name(),
        result.last_name(),
        result.email(),
        result.phone_number(),
        result.status(),
        result.is_archived(),
        role[0].name(),
        groups,
    ))
}

#[utoipa::path(
    get,
    path = "/",
    params(
        ("id" = u64, Path, description = "ID de l'utilisateur")
    ),
    responses(
        (status = 200, description = "User retrieved successfully", body = GetUserResponseView),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_user(
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<impl Responder, GetUserError> {
    let user = trigger_get_user(state, id.parse::<u64>().unwrap_or(0)).await?;
    Ok(HttpResponse::Ok().json(user))
}
