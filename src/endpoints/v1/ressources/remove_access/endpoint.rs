use crate::database::ressources::remove_access::{remove_access_query, RemoveAccessQueryView};
use crate::endpoints::v1::ressources::remove_access::view::RemoveAccessView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum RemoveAccessError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for RemoveAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoveAccessError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            RemoveAccessError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for RemoveAccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            RemoveAccessError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            RemoveAccessError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn remove_access_to_ressource(
    state: web::Data<AppState>,
    view: RemoveAccessView,
) -> Result<(), RemoveAccessError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(RemoveAccessError::DatabaseError),
    };
    let request_view = RemoveAccessQueryView::new(view.access_id());
    remove_access_query(request_view, pool)
        .await
        .map_err(|_| RemoveAccessError::BadRequest)?;
    Ok(())
}

#[utoipa::path(
    post,
    path = "/remove_access",
    responses(
        (status = 200, description = "Access removed successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Ressources",
    security(
        ("jwt" = [])
    )
)]
#[post("/remove_access")]
pub async fn remove_access(
    state: web::Data<AppState>,
    view: web::Json<RemoveAccessView>,
) -> Result<impl Responder, RemoveAccessError> {
    remove_access_to_ressource(state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("Access removed successfully"))
}
