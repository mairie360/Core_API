use crate::database::ressources::add_access_to_user::AddAccessToUserQueryView;
use crate::database::ressources::get_ressource_type_id::GetRessourceTypeIdQueryView;
use crate::database::rights::get_permission_id::{GetPermissionIdQueryView, PermissionAction};
use crate::endpoints::v1::ressources::add_access::view::{AccessType, AddAccessView};
use crate::endpoints::v1::ressources::authorization::{
    can_manage_accesses, fits_int_column, is_valid_ressource_type,
};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddAccessError {
    InvalidId,
    InvalidAccessType,
    UnknownRessourceType,
    UnknownPermission,
    UnknownUser,
    Forbidden,
    Internal,
}

impl std::fmt::Display for AddAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidId => write!(f, "user_id and resource_id must be at most 2147483647."),
            Self::InvalidAccessType => write!(f, "Invalid access type."),
            Self::UnknownRessourceType => write!(f, "Unknown resource type."),
            Self::UnknownPermission => {
                write!(f, "This access type does not exist for this resource type.")
            }
            Self::UnknownUser => write!(f, "Unknown user."),
            Self::Forbidden => write!(
                f,
                "Forbidden: only an administrator or the owner of the resource can manage its accesses."
            ),
            Self::Internal => write!(f, "Internal server error."),
        }
    }
}

impl ResponseError for AddAccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidId
            | Self::InvalidAccessType
            | Self::UnknownRessourceType
            | Self::UnknownPermission
            | Self::UnknownUser => StatusCode::BAD_REQUEST,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Maps a failed lookup: no row means the caller sent an unknown value (`not_found`), anything
/// else is a server error.
const fn lookup_error(err: &ApiLibError, not_found: AddAccessError) -> AddAccessError {
    match err {
        ApiLibError::Database(DbError::NotFound) => not_found,
        _ => AddAccessError::Internal,
    }
}

async fn add_access_to_ressource(
    smart_db: &SmartDatabase,
    caller_id: u64,
    view: &AddAccessView,
) -> Result<(), AddAccessError> {
    if !fits_int_column(view.user_id()) || !fits_int_column(view.resource_id()) {
        return Err(AddAccessError::InvalidId);
    }
    if view.access_type() == AccessType::Error {
        return Err(AddAccessError::InvalidAccessType);
    }
    if !is_valid_ressource_type(view.ressource_type()) {
        return Err(AddAccessError::UnknownRessourceType);
    }

    let ressource_type_id: i32 = smart_db
        .fetch_scalar(&GetRessourceTypeIdQueryView::new(view.ressource_type()))
        .await
        .map_err(|err| lookup_error(&err, AddAccessError::UnknownRessourceType))?;

    let allowed = can_manage_accesses(
        smart_db,
        caller_id,
        view.ressource_type(),
        view.resource_id(),
    )
    .await
    .map_err(|_| AddAccessError::Internal)?;
    if !allowed {
        return Err(AddAccessError::Forbidden);
    }

    let ressource_type_id =
        u64::try_from(ressource_type_id).map_err(|_| AddAccessError::Internal)?;
    let permission_id: i32 = smart_db
        .fetch_scalar(&GetPermissionIdQueryView::new(
            ressource_type_id,
            PermissionAction::from(view.access_type().as_str().to_string()),
        ))
        .await
        .map_err(|err| lookup_error(&err, AddAccessError::UnknownPermission))?;
    let permission_id = u64::try_from(permission_id).map_err(|_| AddAccessError::Internal)?;

    smart_db
        .execute(AddAccessToUserQueryView::new(
            view.user_id(),
            ressource_type_id,
            view.resource_id(),
            permission_id,
        ))
        .await
        .map_err(|err| match err {
            ApiLibError::Database(DbError::ForeignKeyViolation(_)) => AddAccessError::UnknownUser,
            _ => AddAccessError::Internal,
        })
}

#[utoipa::path(
    post,
    path = "/add_access",
    summary = "Grant a user an access to a resource instance",
    description = "Adds an ACL entry giving `user_id` the `access_type` permission on the instance \
                   `resource_id` of `ressource_type` (for example read access to group `3`).\n\n\
                   Only an administrator, or the owner of the instance (its `owner_id` is the \
                   caller), may grant an access. Only `groups` and `events` instances have an \
                   owner: the accesses of any other resource type are managed by administrators \
                   only. A non-administrator asking for an instance that does not exist gets \
                   `403`, like for an instance owned by someone else.\n\n\
                   Idempotent: granting an access the user already has answers `200` without \
                   creating a second entry. List the entries with \
                   `POST /api/v1/ressources/{id}/access` and remove one with \
                   `POST /api/v1/ressources/remove_access`.",
    request_body(
        content = AddAccessView,
        description = "Access to grant.",
        example = json!({
            "user_id": 42,
            "resource_id": 3,
            "ressource_type": "groups",
            "access_type": "Read"
        })
    ),
    responses(
        (
            status = 200,
            description = "Access granted.",
            body = String,
            content_type = "text/plain",
            example = json!("Access added successfully")
        ),
        (
            status = 400,
            description = "Malformed JSON body, `user_id` or `resource_id` above 2147483647, \
                           `access_type` is `Error`, unknown `ressource_type`, access type that does not exist for this resource \
                           type, or `user_id` matching no account.",
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
#[post("/add_access")]
pub async fn add_access(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    view: web::Json<AddAccessView>,
) -> Result<impl Responder, AddAccessError> {
    add_access_to_ressource(state.get_smart_db(), user.id, &view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("Access added successfully"))
}
