use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::users::patch_user::PatchUserQueryView;
use crate::endpoints::v1::user::me::patch::view::PatchMeView;

#[derive(Debug, Clone, PartialEq)]
enum PatchMeError {
    DatabaseError,
}

impl std::fmt::Display for PatchMeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchMeError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for PatchMeError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchMeError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_patch_me(
    state: web::Data<AppState>,
    view: PatchMeView,
    user_id: u64,
) -> Result<(), PatchMeError> {
    let db_view = PatchUserQueryView::new(
        user_id,
        view.first_name(),
        view.last_name(),
        view.email(),
        view.phone(),
        None,
    );
    if !db_view.is_noop() {
        state.get_smart_db().execute(db_view).await.map_err(|e| {
            eprintln!("Error: {:?}", e);
            PatchMeError::DatabaseError
        })?;
    }
    Ok(())
}

#[utoipa::path(
    patch,
    path = "/",
    responses(
        (status = 200, description = "User updated successfully"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/")]
pub async fn patch_me(
    state: web::Data<AppState>,
    view: web::Json<PatchMeView>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PatchMeError> {
    trigger_patch_me(state, view.into_inner(), auth_user.id).await?;
    Ok(HttpResponse::Ok())
}
