use crate::database::users::add_role::{add_role_query, AddRolesQueryView};
use crate::endpoints::v1::admin::users::id::roles::post::view::AddRoleToUserView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum AddRoleToUserError {
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for AddRoleToUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddRoleToUserError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            AddRoleToUserError::NotFound => {
                write!(f, "User or role not found.")
            }
        }
    }
}

impl ResponseError for AddRoleToUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            AddRoleToUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            AddRoleToUserError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn add_role_to_user(
    state: web::Data<AppState>,
    view: AddRoleToUserView,
) -> Result<(), AddRoleToUserError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(AddRoleToUserError::DatabaseError),
    };

    let view = AddRolesQueryView::new(view.role_id(), view.user_id());
    add_role_query(view, pool)
        .await
        .map_err(|_| AddRoleToUserError::NotFound)?;

    Ok(())
}

#[utoipa::path(
    post,
    request_body = AddRoleToUserView,
    path = "/",
    params(
        ("userId" = u64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "User role updated successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "User or role not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn admin_add_role_to_user(
    state: web::Data<AppState>,
    view: web::Json<AddRoleToUserView>,
) -> Result<impl Responder, AddRoleToUserError> {
    add_role_to_user(state, view.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
