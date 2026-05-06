use crate::endpoints::v1::ressources::add_access::view::AddAccessView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetError {
    BadRequest,
    DatabaseError,
    Unauthorized,
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
            GetError::Unauthorized => {
                write!(f, "Unauthorized.")
            }
        }
    }
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetError::BadRequest => StatusCode::BAD_REQUEST,
            GetError::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn add_access_to_ressource(
    state: web::Data<AppState>,
    view: AddAccessView,
) -> Result<(), GetError> {
    Ok(())
}

#[utoipa::path(
    post,
    path = "/add_access",
    responses(
        (status = 200, description = "Access added successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Ressources",
    security(
        ("jwt" = [])
    )
)]
#[post("/add_access")]
pub async fn add_access(
    state: web::Data<AppState>,
    view: web::Json<AddAccessView>,
) -> Result<impl Responder, GetError> {
    match add_access_to_ressource(state, view.into_inner()).await {
        Ok(_) => Ok(HttpResponse::Ok()),
        Err(e) => Err(e),
    }
}
