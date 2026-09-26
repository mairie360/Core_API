use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::password::hash_password;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

use crate::keycloak::sync::{export_user, find_account, profile_of, write_profile, SyncError};
use crate::keycloak::{KeycloakAdminClient, KeycloakUserProfile};
use crate::{
    database::users::patch_user::PatchUserQueryView,
    endpoints::v1::admin::users::id::patch::view::PatchUserView,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum PatchUserError {
    UnknownUser,
    DatabaseError,
    Keycloak(SyncError),
}

impl std::fmt::Display for PatchUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::DatabaseError => write!(f, "Database error occurred"),
            Self::Keycloak(error) => write!(f, "{error}"),
        }
    }
}

impl ResponseError for PatchUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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

async fn patch_in_core(
    smart_db: &SmartDatabase,
    user_id: u64,
    view: &PatchUserView,
) -> Result<(), PatchUserError> {
    let hashed_password = view
        .password()
        .map(hash_password)
        .transpose()
        .map_err(|e| {
            eprintln!("Password hashing error: {e}");
            PatchUserError::DatabaseError
        })?;

    let view = PatchUserQueryView::new(
        user_id,
        view.first_name(),
        view.last_name(),
        view.email(),
        view.phone_number(),
        hashed_password.as_deref(),
    );
    if !view.is_noop() {
        smart_db
            .execute(view)
            .await
            .map_err(|_| PatchUserError::UnknownUser)?;
    }

    Ok(())
}

/// The Keycloak profile of `current` once `view` is applied, `None` when the patch changes
/// nothing Keycloak holds (phone number, password).
fn patched_profile(
    current: &KeycloakUserProfile,
    view: &PatchUserView,
) -> Option<KeycloakUserProfile> {
    if view.first_name().is_none() && view.last_name().is_none() && view.email().is_none() {
        return None;
    }
    Some(KeycloakUserProfile {
        email: view.email().unwrap_or(&current.email).to_string(),
        first_name: view.first_name().unwrap_or(&current.first_name).to_string(),
        last_name: view.last_name().unwrap_or(&current.last_name).to_string(),
        enabled: current.enabled,
    })
}

async fn patch_user(
    state: web::Data<AppState>,
    admin: Option<&KeycloakAdminClient>,
    user_id: u64,
    view: PatchUserView,
) -> Result<(), PatchUserError> {
    let smart_db = state.get_smart_db();
    let Some(admin) = admin else {
        return patch_in_core(smart_db, user_id, &view).await;
    };
    // Unknown user, patch outside the Keycloak profile, or account unknown to Keycloak: Core
    // alone.
    let Some(user) = export_user(smart_db, user_id as i32)
        .await
        .map_err(PatchUserError::Keycloak)?
    else {
        return patch_in_core(smart_db, user_id, &view).await;
    };
    let current = profile_of(&user);
    let Some(patched) = patched_profile(&current, &view) else {
        return patch_in_core(smart_db, user_id, &view).await;
    };
    let Some(keycloak_id) = find_account(smart_db, admin, &user)
        .await
        .map_err(PatchUserError::Keycloak)?
    else {
        return patch_in_core(smart_db, user_id, &view).await;
    };

    write_profile(admin, &keycloak_id, &patched)
        .await
        .map_err(PatchUserError::Keycloak)?;
    if let Err(error) = patch_in_core(smart_db, user_id, &view).await {
        if let Err(restore) = write_profile(admin, &keycloak_id, &current).await {
            eprintln!(
                "Keycloak sync: Core refused the patch of user {user_id} and the former Keycloak profile of {keycloak_id} could not be restored: {restore}"
            );
        }
        return Err(error);
    }
    Ok(())
}

#[utoipa::path(
    patch,
    path = "",
    summary = "Edit a user (administration)",
    description = "Partially updates an account: only the fields present in the body are \
                   written. Reserved to administrators.\n\n\
                   Unlike `PATCH /api/v1/user/me/`, an administrator can also change the \
                   password here. For a reset that enforces the length rules, prefer \
                   `PATCH /api/v1/admin/users/{userId}/password`, which checks them: the \
                   `password` field of this endpoint is written without validation.\n\n\
                   An empty body is accepted and writes nothing. As the account's existence is \
                   not checked beforehand, an unknown `userId` then answers `200`.\n\n\
                   **Keycloak (MAIR-142).** When Core runs with a confidential Keycloak client \
                   and the body changes `first_name`, `last_name` or `email`, the new profile is \
                   written to the user's Keycloak account **before** Core (the account is found \
                   by the recorded link, then by e-mail; an account unknown to Keycloak is left \
                   to the migration and only Core is written). A new e-mail is written verified \
                   and does not change the Keycloak username. If Core then refuses the patch \
                   (`404`), the former Keycloak profile is restored. `phone_number` and \
                   `password` are Core-only. Without Keycloak, or with a public client, only Core \
                   is written.",
    params(
        ("userId" = u64, Path, description = "User id.", example = 42)
    ),
    request_body(
        content = PatchUserView,
        description = "Fields to change. All optional; an absent or `null` field is ignored.",
        example = json!({
            "email": "j.dupont@mairie360.fr",
            "phone_number": "0798765432"
        })
    ),
    responses(
        (
            status = 200,
            description = "Account updated (in Keycloak too when mirrored), or nothing to update.",
            body = String,
            content_type = "text/plain",
            example = json!("User patched successfully!")
        ),
        (
            status = 400,
            description = "Malformed JSON body, or `userId` in the path is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: invalid type: integer `42`, expected a string")
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
            description = "The write failed: unknown `userId`, or e-mail already used by another account. The Keycloak profile, if it had been updated, is restored.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 409,
            description = "Another Keycloak account already uses the new e-mail. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Another Keycloak account already uses this e-mail address.")
        ),
        (
            status = 500,
            description = "The account could not be read, its new password could not be hashed (internal argon2 error), or the Keycloak account found by e-mail is already linked to another Core account. Nothing is changed.",
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
#[patch("/")]
pub async fn admin_patch_user(
    state: web::Data<AppState>,
    admin: Option<web::Data<KeycloakAdminClient>>,
    path: web::Path<u64>,
    view: web::Json<PatchUserView>,
) -> Result<impl Responder, PatchUserError> {
    patch_user(
        state,
        admin.as_ref().map(web::Data::get_ref),
        path.into_inner(),
        view.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().body("User patched successfully!"))
}
