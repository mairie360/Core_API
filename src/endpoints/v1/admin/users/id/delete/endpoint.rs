use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

use crate::database::sessions::get_active_session_ids::GetActiveSessionIdsQueryView;
use crate::database::users::delete_user::DeleteUserQueryView;
use crate::keycloak::sync::{
    disable_account, enable_account, export_user, find_account, SyncError,
};
use crate::keycloak::KeycloakAdminClient;
use crate::session_revocation::publish_revoked_sessions;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
enum DeleteUserError {
    AlreadyDeleted,
    Keycloak(SyncError),
}

impl std::fmt::Display for DeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyDeleted => write!(f, "User is already deleted"),
            Self::Keycloak(error) => write!(f, "{error}"),
        }
    }
}

impl ResponseError for DeleteUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::AlreadyDeleted => StatusCode::OK,
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

async fn archive_in_core(state: &AppState, user_id: u64) -> Result<(), DeleteUserError> {
    let smart_db = state.get_smart_db();
    // The database drops the user's sessions with the account: read them first, to publish them
    // to the revocation list once the deletion succeeded (MAIR-264).
    let sessions: Vec<Uuid> = smart_db
        .fetch_all(&GetActiveSessionIdsQueryView::new(user_id))
        .await
        .unwrap_or_default();

    let view = DeleteUserQueryView::new(user_id);
    smart_db.execute(view).await.map_err(|e| {
        eprintln!("Error: {e}");
        DeleteUserError::AlreadyDeleted
    })?;

    publish_revoked_sessions(state.get_redis(), &sessions).await;

    Ok(())
}

async fn delete_user(
    state: web::Data<AppState>,
    admin: Option<&KeycloakAdminClient>,
    user_id: u64,
) -> Result<(), DeleteUserError> {
    let smart_db = state.get_smart_db();
    // The soft-delete view matches no row for an unknown or archived user: tell it apart first,
    // as the historical contract answers `200` in that case.
    let user = export_user(smart_db, user_id as i32)
        .await
        .map_err(|error| {
            if admin.is_some() {
                DeleteUserError::Keycloak(error)
            } else {
                DeleteUserError::AlreadyDeleted
            }
        })?
        .filter(|user| user.enabled)
        .ok_or(DeleteUserError::AlreadyDeleted)?;
    // Without Keycloak, or for an account unknown to Keycloak: Core alone.
    let Some(admin) = admin else {
        return archive_in_core(&state, user_id).await;
    };
    let Some(keycloak_id) = find_account(smart_db, admin, &user)
        .await
        .map_err(DeleteUserError::Keycloak)?
    else {
        return archive_in_core(&state, user_id).await;
    };

    disable_account(admin, &keycloak_id)
        .await
        .map_err(DeleteUserError::Keycloak)?;
    if let Err(error) = archive_in_core(&state, user_id).await {
        if let Err(restore) = enable_account(admin, &keycloak_id).await {
            eprintln!(
                "Keycloak sync: Core refused to archive user {user_id} and the Keycloak account {keycloak_id} could not be re-enabled: {restore}"
            );
        }
        return Err(error);
    }
    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    summary = "Archive a user (administration)",
    description = "Archives a user account (soft delete: the row is kept, flagged archived, and \
                   the schema ends the user's Core sessions). Reserved to administrators.\n\n\
                   Mind the status: a successful archive answers `204` with an empty body, while \
                   a **failure** (account already archived, unknown id, account owning groups, \
                   events or projects, or database outage) answers `200` with a plain-text \
                   message. A client cannot just test `2xx`: it must tell `204` from `200`.\n\n\
                   **Keycloak (MAIR-142).** When Core runs with a confidential Keycloak client and \
                   the account is active and known to Keycloak (recorded link, then e-mail), its \
                   Keycloak account is **disabled** (never deleted) and all its Keycloak sessions \
                   are ended **before** Core archives it. If Core then refuses (`200`), the \
                   Keycloak account is re-enabled. An account already archived answers `200` \
                   without calling Keycloak; an account unknown to Keycloak only goes through \
                   Core. Without Keycloak, or with a public client, only Core is written.",
    params(
        ("userId" = u64, Path, description = "User id.", example = 42)
    ),
    responses(
        (
            status = 204,
            description = "Account archived (and its Keycloak account disabled and signed out when mirrored). Empty body.",
        ),
        (
            status = 200,
            description = "Nothing was archived: account already archived, unknown id, account owning groups/events/projects, or write failure. The Keycloak account, if it had been disabled, is re-enabled.",
            body = String,
            content_type = "text/plain",
            example = json!("User is already deleted")
        ),
        (
            status = 400,
            description = "`userId` in the path is not an integer.",
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
            status = 500,
            description = "The account could not be read, or the Keycloak account found by e-mail is already linked to another Core account. Nothing is changed.",
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
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn admin_delete_user(
    state: web::Data<AppState>,
    admin: Option<web::Data<KeycloakAdminClient>>,
    path: web::Path<u64>,
) -> Result<impl Responder, DeleteUserError> {
    delete_user(
        state,
        admin.as_ref().map(web::Data::get_ref),
        path.into_inner(),
    )
    .await?;

    Ok(HttpResponse::NoContent())
}
