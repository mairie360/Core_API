use crate::database::roles::get_roles::GetRolesQueryView;
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::v1::admin::roles::get::view::AdminGetRolesResultView;
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
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

async fn get_roles(state: web::Data<AppState>) -> Result<AdminGetRolesResultView, GetError> {
    let view = GetRolesQueryView::default();
    let result = state.get_smart_db().fetch_all(&view).await.map_err(|e| {
        tracing::error!("Login DB Error: {e}");
        GetError::DatabaseError
    })?;
    Ok(AdminGetRolesResultView::from(result))
}

#[utoipa::path(
    get,
    path = "/",
    summary = "List the roles (administration)",
    description = "Returns every role of the platform. Same content as \
                   `GET /api/v1/roles/`, but under the administration prefix: it is the entry \
                   point of the admin screens, next to the write operations.\n\n\
                   Reserved to administrators. A role without description returns an empty string, \
                   never `null`.",
    responses(
        (
            status = 200,
            description = "List of the roles of the platform.",
            body = AdminGetRolesResultView,
            example = json!({
                "roles": [
                    { "id": 1, "name": "admin", "description": "Platform administrator" },
                    { "id": 2, "name": "agent", "description": "Agent municipal" }
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
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 500,
            description = "Database error while reading the roles.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Roles"
)]
#[get("/")]
pub async fn admin_get_role(
    _: AdminUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, GetError> {
    let roles = get_roles(state).await?;
    Ok(HttpResponse::Ok().json(roles))
}
