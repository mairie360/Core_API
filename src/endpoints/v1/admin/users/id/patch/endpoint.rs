use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

use crate::{
    database::users::patch_user::{patch_user_query, PatchUserQueryView},
    endpoints::v1::admin::users::id::patch::view::PatchUserView,
};

#[derive(Debug, Clone, PartialEq)]
enum PatchUserError {
    UnknownUser,
}

impl std::fmt::Display for PatchUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchUserError::UnknownUser => write!(f, "Unknown user"),
        }
    }
}

impl ResponseError for PatchUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchUserError::UnknownUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn patch_user(
    state: web::Data<AppState>,
    user_id: u64,
    view: PatchUserView,
) -> Result<(), PatchUserError> {
    let view = PatchUserQueryView::new(
        user_id,
        view.first_name(),
        view.last_name(),
        view.email(),
        view.phone_number(),
        view.password(),
    );
    patch_user_query(view, state.get_smart_db())
        .await
        .map_err(|_| PatchUserError::UnknownUser)?;

    Ok(())
}

#[utoipa::path(
    patch,
    path = "",
    params(
        ("userId" = u64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "User patched successfully"),
        (status = 400, description = "Invalid data provided"),
        (status = 404, description = "Unknown user"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[patch("/")]
pub async fn admin_patch_user(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: web::Json<PatchUserView>,
) -> Result<impl Responder, PatchUserError> {
    patch_user(state, path.into_inner(), view.into_inner()).await?;

    Ok(HttpResponse::Ok().body("User patched successfully!"))
}
