use crate::database::roles::get_roles::{get_roles_query, GetRolesQueryView};
use crate::endpoints::v1::admin::roles::get::view::GetResponseView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetError {
    DatabaseError,
}

impl std::fmt::Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_roles(state: web::Data<AppState>) -> Result<GetResponseView, GetError> {
    let view = GetRolesQueryView {};
    let result = get_roles_query(view, state.db_pool.clone())
        .await
        .map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            GetError::DatabaseError
        })?;
    Ok(GetResponseView::from(result))
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Roles retrieved successfully", body = GetResponseView),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get(state: web::Data<AppState>) -> Result<impl Responder, GetError> {
    let roles = get_roles(state).await?;
    Ok(HttpResponse::Ok().json(roles))
}
