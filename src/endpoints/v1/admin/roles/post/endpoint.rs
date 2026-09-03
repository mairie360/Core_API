use crate::database::roles::create_role::CreateRoleQueryView;
use crate::endpoints::v1::admin::roles::view::RoleWriteView;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PostError {
    Duplicate,
}

impl std::fmt::Display for PostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostError::Duplicate => {
                write!(f, "A role with this name already exists.")
            }
        }
    }
}

impl ResponseError for PostError {
    fn status_code(&self) -> StatusCode {
        match self {
            PostError::Duplicate => StatusCode::CONFLICT,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn create_role(payload: RoleWriteView, state: web::Data<AppState>) -> Result<(), PostError> {
    let view = CreateRoleQueryView::new(
        payload.name(),
        payload.description(),
        payload.can_be_deleted(),
    );

    state
        .get_smart_db()
        .execute(view)
        .await
        .map_err(|_| PostError::Duplicate)?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/",
    request_body = RoleWriteView,
    responses(
        (status = 200, description = "Role created successfully"),
        (status = 400, description = "Bad request"),
        (status = 409, description = "Duplicate role name"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Roles"
)]
#[post("/")]
pub async fn admin_post_role(
    payload: web::Json<RoleWriteView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, PostError> {
    create_role(payload.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
