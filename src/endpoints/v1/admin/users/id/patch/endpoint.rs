use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::pool::AppState;

use crate::{
    database::users::patch_user::{patch_user_query, PatchUserQueryView},
    endpoints::v1::admin::users::id::patch::view::PatchUserView,
};

#[derive(Debug, Clone, PartialEq)]
enum PatchUserError {
    DatabaseError,
    UnknownUser,
}

impl std::fmt::Display for PatchUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchUserError::DatabaseError => write!(f, "Database error occurred"),
            PatchUserError::UnknownUser => write!(f, "Unknown user"),
        }
    }
}

impl ResponseError for PatchUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(PatchUserError::DatabaseError),
    };

    let view = PatchUserQueryView::new(
        user_id,
        view.first_name().as_deref(),
        view.last_name().as_deref(),
        view.email().as_deref(),
        view.phone_number().as_deref(),
    );
    patch_user_query(view, &pool)
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
pub async fn patch(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: web::Json<PatchUserView>,
) -> Result<impl Responder, PatchUserError> {
    patch_user(state, path.into_inner(), view.into_inner()).await?;

    Ok(HttpResponse::Ok().body("User patched successfully!"))
}
