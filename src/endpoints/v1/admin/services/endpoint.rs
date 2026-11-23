use crate::database::services::{
    CreateServiceQueryView, DeleteServiceQueryView, ListServicesQueryView, Service,
    UpdateServiceQueryView,
};
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::db_error::{self, DbFailure};
use crate::endpoints::v1::admin::services::view::{ServiceView, ServiceWriteView, ServicesView};
use crate::endpoints::validation::ValidatedJson;
use actix_web::http::StatusCode;
use actix_web::{delete, get, patch, post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    NotFound,
    Duplicate,
    DatabaseError,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "No service has this id."),
            Self::Duplicate => write!(f, "A service with this name already exists."),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for ServiceError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Duplicate => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

fn database_error(
    context: &str,
) -> impl Fn(mairie360_api_lib::error::ApiLibError) -> ServiceError + '_ {
    move |e| match db_error::log(context, &e) {
        DbFailure::Conflict => ServiceError::Duplicate,
        DbFailure::NotFound => ServiceError::NotFound,
        _ => ServiceError::DatabaseError,
    }
}

#[utoipa::path(
    get,
    path = "/",
    summary = "List the services of the mairie",
    description = "Every service of the mairie, sorted by name, with the number of active agents attached \
                   to it (MAIR-923). Reserved to administrators.",
    responses(
        (status = 200, description = "The services.", body = ServicesView,
         example = json!({"services": [{"id": 3, "name": "Roads", "created_at": "2026-11-20T08:00:00Z", "agents": 12}]})),
        (status = 401, description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
         body = String, content_type = "text/plain", example = json!("Jeton expiré")),
        (status = 403, description = "The user is authenticated but is not an administrator.",
         body = String, content_type = "text/plain", example = json!("Forbidden: User is not an admin.")),
        (status = 500, description = "Database failure.",
         body = String, content_type = "text/plain", example = json!("An error occurred while accessing the database.")),
    ),
    security(("jwt" = [])),
    tag = "Admin - Services"
)]
#[get("/")]
pub async fn admin_list_services(
    _: AdminUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, ServiceError> {
    let services: Vec<Service> = state
        .get_smart_db()
        .fetch_all(&ListServicesQueryView::default())
        .await
        .map_err(database_error("list services"))?;
    Ok(HttpResponse::Ok().json(ServicesView::from(services)))
}

#[utoipa::path(
    post,
    path = "/",
    summary = "Create a service",
    description = "Creates a service of the mairie (MAIR-923). Its name is unique whatever its case. \
                   Reserved to administrators.",
    request_body(content = ServiceWriteView, example = json!({"name": "Roads"})),
    responses(
        (status = 201, description = "Service created.", body = ServiceView,
         example = json!({"id": 3, "name": "Roads", "created_at": "2026-11-20T08:00:00Z", "agents": 0})),
        (status = 400, description = "Malformed body, or `name` breaking its rules: 1 to 64 characters, not blank, no control character.",
         body = String, content_type = "text/plain", example = json!("Invalid `name`: must not be empty")),
        (status = 401, description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
         body = String, content_type = "text/plain", example = json!("Jeton expiré")),
        (status = 403, description = "The user is authenticated but is not an administrator.",
         body = String, content_type = "text/plain", example = json!("Forbidden: User is not an admin.")),
        (status = 409, description = "A service already has this name, whatever its case.",
         body = String, content_type = "text/plain", example = json!("A service with this name already exists.")),
        (status = 500, description = "Database failure other than a duplicate name.",
         body = String, content_type = "text/plain", example = json!("An error occurred while accessing the database.")),
    ),
    security(("jwt" = [])),
    tag = "Admin - Services"
)]
#[post("/")]
pub async fn admin_create_service(
    _: AdminUser,
    payload: ValidatedJson<ServiceWriteView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ServiceError> {
    let service: Service = state
        .get_smart_db()
        .fetch_one(&CreateServiceQueryView::new(payload.into_inner().name()))
        .await
        .map_err(database_error("create service"))?;
    Ok(HttpResponse::Created().json(ServiceView::from(service)))
}

#[utoipa::path(
    patch,
    path = "/{serviceId}",
    summary = "Rename a service",
    description = "Renames a service of the mairie (MAIR-923); its agents stay attached to it. Reserved to \
                   administrators.",
    request_body(content = ServiceWriteView, example = json!({"name": "Roads and networks"})),
    responses(
        (status = 200, description = "Service renamed.", body = ServiceView,
         example = json!({"id": 3, "name": "Roads and networks", "created_at": "2026-11-20T08:00:00Z", "agents": 12})),
        (status = 400, description = "The `serviceId` of the path is not an integer, the body is malformed, or `name` breaks its rules: 1 to 64 characters, not blank, no control character.",
         body = String, content_type = "text/plain", example = json!("Invalid path: ids are integers.")),
        (status = 401, description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
         body = String, content_type = "text/plain", example = json!("Jeton expiré")),
        (status = 403, description = "The user is authenticated but is not an administrator.",
         body = String, content_type = "text/plain", example = json!("Forbidden: User is not an admin.")),
        (status = 404, description = "No service has this id.",
         body = String, content_type = "text/plain", example = json!("No service has this id.")),
        (status = 409, description = "Another service already has this name, whatever its case.",
         body = String, content_type = "text/plain", example = json!("A service with this name already exists.")),
        (status = 500, description = "Database failure.",
         body = String, content_type = "text/plain", example = json!("An error occurred while accessing the database.")),
    ),
    params(("serviceId" = u64, Path, description = "Id of the service.", example = 3)),
    security(("jwt" = [])),
    tag = "Admin - Services"
)]
#[patch("/{serviceId}")]
pub async fn admin_update_service(
    _: AdminUser,
    id: web::Path<u64>,
    payload: ValidatedJson<ServiceWriteView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ServiceError> {
    let services: Vec<Service> = state
        .get_smart_db()
        .fetch_all(&UpdateServiceQueryView::new(
            id.into_inner(),
            payload.into_inner().name(),
        ))
        .await
        .map_err(database_error("rename service"))?;
    let service = services.into_iter().next().ok_or(ServiceError::NotFound)?;
    Ok(HttpResponse::Ok().json(ServiceView::from(service)))
}

#[utoipa::path(
    delete,
    path = "/{serviceId}",
    summary = "Delete a service",
    description = "Deletes a service of the mairie (MAIR-923); its agents are left without a service. \
                   Reserved to administrators.",
    responses(
        (status = 204, description = "Service deleted. Empty body."),
        (status = 400, description = "The `serviceId` of the path is not an integer.",
         body = String, content_type = "text/plain", example = json!("Invalid path: ids are integers.")),
        (status = 401, description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
         body = String, content_type = "text/plain", example = json!("Jeton expiré")),
        (status = 403, description = "The user is authenticated but is not an administrator.",
         body = String, content_type = "text/plain", example = json!("Forbidden: User is not an admin.")),
        (status = 404, description = "No service has this id.",
         body = String, content_type = "text/plain", example = json!("No service has this id.")),
        (status = 500, description = "Database failure.",
         body = String, content_type = "text/plain", example = json!("An error occurred while accessing the database.")),
    ),
    params(("serviceId" = u64, Path, description = "Id of the service.", example = 3)),
    security(("jwt" = [])),
    tag = "Admin - Services"
)]
#[delete("/{serviceId}")]
pub async fn admin_delete_service(
    _: AdminUser,
    id: web::Path<u64>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ServiceError> {
    let deleted: Vec<i32> = state
        .get_smart_db()
        .fetch_all(&DeleteServiceQueryView::new(id.into_inner()))
        .await
        .map_err(database_error("delete service"))?;
    if deleted.is_empty() {
        return Err(ServiceError::NotFound);
    }
    Ok(HttpResponse::NoContent().finish())
}
