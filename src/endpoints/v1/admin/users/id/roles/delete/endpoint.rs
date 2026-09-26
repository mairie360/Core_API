use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

use crate::database::users::remove_role::RemoveRolesQueryView;
use crate::endpoints::v1::admin::users::id::roles::post::endpoint::role_name;
use crate::keycloak::sync::{export_user, find_account, map_role, unmap_role, SyncError};
use crate::keycloak::KeycloakAdminClient;

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemoveUserRoleError {
    NotFound,
    Keycloak(SyncError),
}

impl std::fmt::Display for RemoveUserRoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => {
                write!(f, "The requested resource was not found.")
            }
            Self::Keycloak(error) => write!(f, "{error}"),
        }
    }
}

impl ResponseError for RemoveUserRoleError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Keycloak(SyncError::EmailTaken) => StatusCode::CONFLICT,
            Self::Keycloak(SyncError::Database | SyncError::LinkedToAnotherUser) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Keycloak(SyncError::NotConfigured) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Keycloak(SyncError::KeycloakForbidden | SyncError::KeycloakUnavailable) => {
                StatusCode::BAD_GATEWAY
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn revoke_in_core(
    smart_db: &SmartDatabase,
    user_id: u64,
    role_id: u64,
) -> Result<(), RemoveUserRoleError> {
    let view = RemoveRolesQueryView::new(role_id, user_id);
    smart_db
        .execute(view)
        .await
        .map_err(|_| RemoveUserRoleError::NotFound)?;

    Ok(())
}

async fn remove_role_from_user(
    state: web::Data<AppState>,
    admin: Option<&KeycloakAdminClient>,
    user_id: u64,
    role_id: u64,
) -> Result<(), RemoveUserRoleError> {
    let smart_db = state.get_smart_db();
    let Some(admin) = admin else {
        return revoke_in_core(smart_db, user_id, role_id).await;
    };
    // Unknown role or user (Core answers 404), or account unknown to Keycloak: Core alone.
    let Some(name) = role_name(smart_db, role_id)
        .await
        .map_err(RemoveUserRoleError::Keycloak)?
    else {
        return revoke_in_core(smart_db, user_id, role_id).await;
    };
    let Some(user) = export_user(smart_db, user_id as i32)
        .await
        .map_err(RemoveUserRoleError::Keycloak)?
    else {
        return revoke_in_core(smart_db, user_id, role_id).await;
    };
    let Some(keycloak_id) = find_account(smart_db, admin, &user)
        .await
        .map_err(RemoveUserRoleError::Keycloak)?
    else {
        return revoke_in_core(smart_db, user_id, role_id).await;
    };

    let unmapped = unmap_role(admin, &keycloak_id, &name)
        .await
        .map_err(RemoveUserRoleError::Keycloak)?;
    if let Err(error) = revoke_in_core(smart_db, user_id, role_id).await {
        if unmapped {
            if let Err(restore) = map_role(admin, &keycloak_id, &name).await {
                eprintln!(
                    "Keycloak sync: Core refused to revoke role {name} from user {user_id} and it could not be re-mapped to {keycloak_id}: {restore}"
                );
            }
        }
        return Err(error);
    }
    Ok(())
}

#[utoipa::path(
    delete,
    summary = "Revoke a role from a user",
    description = "Detaches a role from a user account. The role itself and the account are \
                   kept; only the assignment disappears. Reserved to administrators.\n\n\
                   Unlike the grant, both ids are read from the path. Revoking a role the user \
                   does not hold, or naming an unknown user or role, matches no row and still \
                   answers `204`; `404` is reserved to a failed write.\n\n\
                   **Keycloak (MAIR-142).** When Core runs with a confidential Keycloak client \
                   and the account is known to Keycloak (recorded link, then e-mail), the realm \
                   role of the same name is unmapped from the Keycloak account **before** Core \
                   writes; if Core then refuses (`404`), the mapping is restored. Roles the user \
                   holds only in Keycloak are never touched. An unknown role or user, or an \
                   account unknown to Keycloak, only goes through Core. Without Keycloak, or with \
                   a public client, only Core is written.",
    params(
        ("userId" = u64, Path, description = "User id.", example = 42),
        ("roleId" = u64, Path, description = "Id of the role to revoke.", example = 2)
    ),
    path = "/{roleId}",
    responses(
        (
            status = 204,
            description = "Role revoked (and unmapped in Keycloak when mirrored), or nothing to revoke. Empty body.",
        ),
        (
            status = 400,
            description = "`userId` or `roleId` is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("can not parse \"abc\" to a u64")
        ),
        (
            status = 401,
            description = "`Authorization` header missing, JWT invalid or expired, or session revoked.",
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
            description = "The write failed (database outage). The Keycloak mapping, if it had been removed, is restored.",
            body = String,
            content_type = "text/plain",
            example = json!("The requested resource was not found.")
        ),
        (
            status = 500,
            description = "The role or the account could not be read, or the Keycloak account found by e-mail is already linked to another Core account. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 502,
            description = "Keycloak could not be reached or refused Core's service account. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Unavailable" = (value = json!("Keycloak is unavailable."))),
                ("Service account refused" = (value = json!("Keycloak refused Core's service account.")))
            )
        ),
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Users"
)]
#[delete("/{roleId}")]
pub async fn admin_delete_user_role(
    state: web::Data<AppState>,
    admin: Option<web::Data<KeycloakAdminClient>>,
    params: web::Path<(u64, u64)>,
) -> Result<impl Responder, RemoveUserRoleError> {
    let (user_id, role_id) = params.into_inner();
    remove_role_from_user(
        state,
        admin.as_ref().map(web::Data::get_ref),
        user_id,
        role_id,
    )
    .await?;
    Ok(HttpResponse::NoContent())
}
