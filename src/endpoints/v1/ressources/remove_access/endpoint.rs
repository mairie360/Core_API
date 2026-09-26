use crate::database::ressources::get_access_entry::{AccessEntry, GetAccessEntryQueryView};
use crate::database::ressources::remove_access::RemoveAccessQueryView;
use crate::endpoints::v1::ressources::authorization::{is_admin, is_owner};
use crate::endpoints::v1::ressources::remove_access::view::RemoveAccessView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveAccessError {
    NotFound,
    Forbidden,
    Internal,
}

impl std::fmt::Display for RemoveAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Access not found."),
            Self::Forbidden => write!(
                f,
                "Forbidden: only an administrator or the owner of the resource can manage its accesses."
            ),
            Self::Internal => write!(f, "Internal server error."),
        }
    }
}

impl ResponseError for RemoveAccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn remove_access_to_ressource(
    smart_db: &SmartDatabase,
    caller_id: u64,
    view: &RemoveAccessView,
) -> Result<(), RemoveAccessError> {
    let caller_is_admin = is_admin(smart_db, caller_id)
        .await
        .map_err(|_| RemoveAccessError::Internal)?;

    let entry: AccessEntry = match smart_db
        .fetch_one(&GetAccessEntryQueryView::new(view.access_id()))
        .await
    {
        Ok(entry) => entry,
        // Only an administrator learns that an entry does not exist: anybody else cannot tell it
        // apart from an entry of a resource they do not own.
        Err(ApiLibError::Database(DbError::NotFound)) if caller_is_admin => {
            return Err(RemoveAccessError::NotFound)
        }
        Err(ApiLibError::Database(DbError::NotFound)) => return Err(RemoveAccessError::Forbidden),
        Err(_) => return Err(RemoveAccessError::Internal),
    };

    if !caller_is_admin {
        let instance_id = u64::try_from(entry.resource_instance_id())
            .map_err(|_| RemoveAccessError::Forbidden)?;
        let owner = is_owner(smart_db, caller_id, entry.ressource_type(), instance_id)
            .await
            .map_err(|_| RemoveAccessError::Internal)?;
        if !owner {
            return Err(RemoveAccessError::Forbidden);
        }
    }

    smart_db
        .execute(RemoveAccessQueryView::new(view.access_id()))
        .await
        .map_err(|_| RemoveAccessError::Internal)
}

#[utoipa::path(
    post,
    path = "/remove_access",
    summary = "Remove an access from a resource instance",
    description = "Deletes one ACL entry, identified by its `id` as listed by \
                   `POST /api/v1/ressources/{id}/access`.\n\n\
                   Only an administrator, or the owner of the instance the entry applies to, may \
                   remove it. Only `groups` and `events` instances have an owner: the entries of \
                   any other resource type are removed by administrators only. A \
                   non-administrator gets `403` both for an entry of an instance they do not own \
                   and for an entry that does not exist.",
    request_body(
        content = RemoveAccessView,
        description = "Entry to remove.",
        example = json!({ "access_id": 12 })
    ),
    responses(
        (
            status = 200,
            description = "Access removed.",
            body = String,
            content_type = "text/plain",
            example = json!("Access removed successfully")
        ),
        (
            status = 400,
            description = "Malformed JSON body or missing `access_id`.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `access_id`")
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
                           the entry applies to (or, for a non-administrator, the entry does not \
                           exist).",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: only an administrator or the owner of the resource can manage its accesses.")
        ),
        (
            status = 404,
            description = "Administrators only: no entry has this `access_id`.",
            body = String,
            content_type = "text/plain",
            example = json!("Access not found.")
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
#[post("/remove_access")]
pub async fn remove_access(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    view: web::Json<RemoveAccessView>,
) -> Result<impl Responder, RemoveAccessError> {
    remove_access_to_ressource(state.get_smart_db(), user.id, &view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("Access removed successfully"))
}
