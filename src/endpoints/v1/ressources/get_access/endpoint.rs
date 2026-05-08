use crate::database::ressources::get_access_by_ressource::{
    get_access_by_ressource, GetAccessByRessourceQueryView,
};
use crate::endpoints::v1::ressources::GetAccessResultView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            GetError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_access_from_ressource(
    state: web::Data<AppState>,
    ressource_id: u64,
) -> Result<GetAccessResultView, GetError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetError::DatabaseError),
    };

    let view = GetAccessByRessourceQueryView::new(ressource_id);
    let result = get_access_by_ressource(view, pool)
        .await
        .map_err(|_| GetError::BadRequest)?;

    Ok(GetAccessResultView::new(result))
}

#[utoipa::path(
    post,
    path = "/{id}/access",
    responses(
        (status = 200, body = GetAccessResultView),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Ressources",
    security(
        ("jwt" = [])
    )
)]
#[post("/{id}/access")]
pub async fn get_access(
    state: web::Data<AppState>,
    id: web::Path<u64>,
) -> Result<impl Responder, GetError> {
    let response = get_access_from_ressource(state, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}
