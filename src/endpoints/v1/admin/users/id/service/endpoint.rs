//! Service of an agent, set by an administrator (MAIR-923, Database `mair-923-01`).

use crate::database::services::SetUserServiceQueryView;
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::db_error::{self, DbFailure};
use crate::endpoints::export_data::sqlstate;
use crate::endpoints::v1::admin::users::id::service::view::UserServiceView;
use crate::endpoints::validation::ValidatedJson;
use actix_web::{error::ResponseError, http::StatusCode, put, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserServiceError {
    UnknownUser,
    UnknownService,
    DatabaseError,
}

impl std::fmt::Display for UserServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::UnknownService => write!(f, "No service has this id."),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for UserServiceError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser | Self::UnknownService => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

#[utoipa::path(
    put,
    path = "service",
    summary = "Set the service of a user (administration)",
    description = "Attaches the agent to a service of the mairie (MAIR-923), replacing the one they \
                   had, or detaches them with `service_id: null`. The service then shows in the \
                   administration list and record of the user, and the search of \
                   `GET /api/v1/admin/users/` finds the agent by the name of their service. \
                   Idempotent. Reserved to administrators.",
    params(
        ("userId" = u64, Path, description = "Id of the user.", example = 42)
    ),
    request_body(content = UserServiceView, example = json!({"service_id": 3})),
    responses(
        (status = 204, description = "Service set. Empty body."),
        (
            status = 400,
            description = "`userId` is not an integer, or the body is malformed (`service_id` must be an integer or `null`).",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid path: ids are integers.")
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
            status = 404,
            description = "No active user has this id (unknown or archived), or no service has `service_id`. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Unknown user" = (value = json!("Unknown user"))),
                ("Unknown service" = (value = json!("No service has this id.")))
            )
        ),
        (
            status = 500,
            description = "Database error. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Users",
    security(("jwt" = []))
)]
#[put("/service")]
pub async fn admin_set_user_service(
    _: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    payload: ValidatedJson<UserServiceView>,
) -> Result<impl Responder, UserServiceError> {
    let query = SetUserServiceQueryView::new(path.into_inner(), payload.into_inner().service_id());
    let updated: Vec<i32> = state.get_smart_db().fetch_all(&query).await.map_err(|e| {
        if sqlstate(&e).as_deref() == Some("23503") {
            return UserServiceError::UnknownService;
        }
        match db_error::log("set the service of a user", &e) {
            DbFailure::NotFound => UserServiceError::UnknownUser,
            _ => UserServiceError::DatabaseError,
        }
    })?;
    if updated.is_empty() {
        return Err(UserServiceError::UnknownUser);
    }
    Ok(HttpResponse::NoContent().finish())
}
