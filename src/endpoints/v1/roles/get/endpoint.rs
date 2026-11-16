use crate::database::roles::get_roles::GetRolesQueryView;
use crate::endpoints::v1::roles::get::view::GetRolesResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetError {
    DatabaseError,
}

impl std::fmt::Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_get_roles(state: web::Data<AppState>) -> Result<GetRolesResultView, GetError> {
    let view = GetRolesQueryView::default();
    let result = state.get_smart_db().fetch_all(&view).await.map_err(|e| {
        tracing::error!("Login DB Error: {e}");
        GetError::DatabaseError
    })?;
    Ok(GetRolesResultView::from(result))
}

#[utoipa::path(
    get,
    path = "/",
    summary = "List the available roles",
    description = "Returns every role defined on the platform, to fill a picker \
                   on the client side. Read-only and open to any authenticated user: \
                   creating, updating and deleting roles goes through \
                   `/api/v1/admin/roles`, reserved to administrators.\n\n\
                   A role without description returns an empty string, never `null`.",
    responses(
        (
            status = 200,
            description = "List of the roles of the platform.",
            body = GetRolesResultView,
            example = json!({
                "roles": [
                    { "id": 1, "name": "admin", "description": "Administrateur de la plateforme" },
                    { "id": 2, "name": "agent", "description": "Agent municipal" },
                    { "id": 3, "name": "citoyen", "description": "" }
                ]
            })
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error while reading the roles.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Roles",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_roles(state: web::Data<AppState>) -> Result<impl Responder, GetError> {
    let roles = trigger_get_roles(state).await?;
    Ok(HttpResponse::Ok().json(roles))
}
