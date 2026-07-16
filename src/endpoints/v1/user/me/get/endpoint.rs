use crate::database::groups::get_user_groups::{get_user_groups, GetUserGroupsQuerView};
use crate::database::roles::get_roles_by_id::{get_roles_by_id_query, GetRolesByIdQueryView};
use crate::database::users::get_roles::{get_user_roles_query, GetUserRolesQueryView};
use crate::database::users::get_user_by_id::{get_user_by_id_query, GetUserByIdQueryView};
use crate::endpoints::v1::user::me::get::view::GetMeResponseView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum GetMeError {
    DatabaseError,
}

impl std::fmt::Display for GetMeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetMeError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetMeError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetMeError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_get_me(
    state: web::Data<AppState>,
    user_id: u64,
) -> Result<GetMeResponseView, GetMeError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetMeError::DatabaseError),
    };
    let view = GetUserByIdQueryView::new(user_id);
    let result = get_user_by_id_query(view, pool.clone())
        .await
        .map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            GetMeError::DatabaseError
        })?;
    let view = GetUserGroupsQuerView::new(user_id);
    let groups = get_user_groups(view, pool.clone()).await.map_err(|e| {
        eprintln!("Login DB Error: {}", e);
        GetMeError::DatabaseError
    })?;
    let role = GetUserRolesQueryView::new(user_id);
    let role_id = get_user_roles_query(role, pool.clone())
        .await
        .map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            GetMeError::DatabaseError
        })?;
    let view = GetRolesByIdQueryView::new(role_id);
    let role = get_roles_by_id_query(view, pool).await.map_err(|e| {
        eprintln!("Login DB Error: {}", e);
        GetMeError::DatabaseError
    })?;

    Ok(GetMeResponseView::new(
        result.first_name(),
        result.last_name(),
        result.email(),
        result.phone_number(),
        result.status(),
        role[0].name(),
        groups,
    ))
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Me retrieved successfully", body = GetMeResponseView),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_me(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, GetMeError> {
    let me = trigger_get_me(state, auth_user.id).await?;
    Ok(HttpResponse::Ok().json(me))
}
