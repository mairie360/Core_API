use crate::database::ressources::get_access_by_ressource::GetAccessByRessourceQueryView;
use crate::database::ressources::get_ressource_type_id::GetRessourceTypeIdQueryView;
use crate::endpoints::v1::ressources::authorization::{
    can_manage_accesses, is_valid_ressource_type,
};
use crate::endpoints::v1::ressources::get_access::view::GetAccessQuery;
use crate::endpoints::v1::ressources::GetAccessResultView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetError {
    UnknownRessourceType,
    Forbidden,
    Internal,
}

impl std::fmt::Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRessourceType => write!(f, "Unknown resource type."),
            Self::Forbidden => write!(
                f,
                "Forbidden: only an administrator or the owner of the resource can manage its accesses."
            ),
            Self::Internal => write!(f, "Internal server error."),
        }
    }
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownRessourceType => StatusCode::BAD_REQUEST,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_access_from_ressource(
    smart_db: &SmartDatabase,
    caller_id: u64,
    ressource_id: u64,
    ressource_type: &str,
) -> Result<GetAccessResultView, GetError> {
    if !is_valid_ressource_type(ressource_type) {
        return Err(GetError::UnknownRessourceType);
    }
    smart_db
        .fetch_scalar::<i32, _>(&GetRessourceTypeIdQueryView::new(ressource_type))
        .await
        .map_err(|err| match err {
            ApiLibError::Database(DbError::NotFound) => GetError::UnknownRessourceType,
            _ => GetError::Internal,
        })?;

    let allowed = can_manage_accesses(smart_db, caller_id, ressource_type, ressource_id)
        .await
        .map_err(|_| GetError::Internal)?;
    if !allowed {
        return Err(GetError::Forbidden);
    }

    let result = smart_db
        .fetch_all(&GetAccessByRessourceQueryView::new(
            ressource_id,
            ressource_type,
        ))
        .await
        .map_err(|_| GetError::Internal)?;

    Ok(GetAccessResultView::new(result))
}

#[utoipa::path(
    post,
    path = "/{id}/access",
    summary = "List the accesses of a resource instance",
    description = "Returns the ACL entries (user or group, permission) granted on the instance \
                   `id` of `ressource_type`. Global role rights are not listed, only per-instance \
                   entries created by `POST /api/v1/ressources/add_access` (or by the database, \
                   such as the group-owner entries).\n\n\
                   Only an administrator, or the owner of the instance, may list its accesses. \
                   Only `groups` and `events` instances have an owner: the accesses of any other \
                   resource type are listed by administrators only. A non-administrator asking \
                   for an instance that does not exist gets `403`.",
    params(
        ("id" = u64, Path, description = "Id of the resource instance.", example = 3),
        GetAccessQuery
    ),
    responses(
        (
            status = 200,
            description = "Entries of the instance, possibly empty.",
            body = GetAccessResultView,
            example = json!({
                "accesses": [
                    {
                        "id": 12,
                        "user_id": 42,
                        "group_id": null,
                        "resource_id": 5,
                        "resource_instance_id": 3,
                        "permission_id": 21
                    }
                ]
            })
        ),
        (
            status = 400,
            description = "Missing `ressource_type` query parameter, or unknown resource type.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown resource type.")
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
            description = "The caller is neither an administrator nor the owner of the instance \
                           (or the instance does not exist).",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: only an administrator or the owner of the resource can manage its accesses.")
        ),
        (
            status = 500,
            description = "Database failure.",
            body = String,
            content_type = "text/plain",
            example = json!("Internal server error.")
        )
    ),
    tag = "Ressources",
    security(
        ("jwt" = [])
    )
)]
#[post("/{id}/access")]
pub async fn get_access(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<u64>,
    query: web::Query<GetAccessQuery>,
) -> Result<impl Responder, GetError> {
    let response = get_access_from_ressource(
        state.get_smart_db(),
        user.id,
        id.into_inner(),
        query.ressource_type(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(response))
}
