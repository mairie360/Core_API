use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::validation::ValidatedJson;
use crate::passwords;
use crate::session_revocation::{publish_revoked_sessions, revoke_all_user_sessions_in};
use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

use crate::{
    database::users::patch_user::PatchUserQueryView,
    endpoints::v1::admin::users::id::patch::view::PatchUserView,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum PatchUserError {
    BadRequest(String),
    EmailAlreadyUsed,
    DatabaseError,
}

impl std::fmt::Display for PatchUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(message) => write!(f, "{message}"),
            Self::EmailAlreadyUsed => {
                write!(f, "Another account already uses this e-mail address.")
            }
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for PatchUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::EmailAlreadyUsed => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Writes the patch to Core. A new password also revokes every session of the user, in the same
/// transaction (MAIR-420): the sessions to publish to the revocation list are returned.
async fn patch_in_core(
    smart_db: &SmartDatabase,
    user_id: u64,
    view: &PatchUserView,
) -> Result<Vec<Uuid>, PatchUserError> {
    let hashed_password = match view.password() {
        Some(password) => Some(passwords::hash(password).await.map_err(|e| {
            tracing::error!("Password hashing error: {e}");
            PatchUserError::DatabaseError
        })?),
        None => None,
    };

    let phone = view
        .phone_change()
        .map_err(|e| PatchUserError::BadRequest(e.to_string()))?;
    let query = PatchUserQueryView::new(
        user_id,
        view.first_name(),
        view.last_name(),
        view.email(),
        &phone,
        hashed_password.as_deref(),
    );
    if query.is_noop() {
        return Ok(Vec::new());
    }
    let database_error = |e: ApiLibError| match e {
        ApiLibError::Database(DbError::UniqueViolation(_)) => PatchUserError::EmailAlreadyUsed,
        e => {
            tracing::error!("Admin patch user DB Error: {e}");
            PatchUserError::DatabaseError
        }
    };
    let mut tx = smart_db.begin().await.map_err(database_error)?;
    tx.execute(&query).await.map_err(database_error)?;
    // A new password ends the sessions opened with the previous one, in every API, like
    // `PATCH /admin/users/{userId}/password` (MAIR-390).
    let revoked = if hashed_password.is_some() {
        revoke_all_user_sessions_in(&mut tx, user_id)
            .await
            .map_err(database_error)?
    } else {
        Vec::new()
    };
    tx.commit().await.map_err(database_error)?;
    Ok(revoked)
}

async fn patch_user(
    state: web::Data<AppState>,
    user_id: u64,
    view: PatchUserView,
) -> Result<Vec<Uuid>, PatchUserError> {
    patch_in_core(state.get_smart_db(), user_id, &view).await
}

#[utoipa::path(
    patch,
    path = "",
    summary = "Edit a user (administration)",
    description = "Partially updates an account: only the fields present in the body are \
                   written. Reserved to administrators.\n\n\
                   Unlike `PATCH /api/v1/user/me/`, an administrator can also change the \
                   password here (same rules as `PATCH /api/v1/admin/users/{userId}/password`); \
                   every session of the user is then revoked.\n\n\
                   An empty body is accepted and writes nothing. As the account's existence is \
                   not checked beforehand, an unknown `userId` then answers `200`.",
    params(
        ("userId" = u64, Path, description = "User id.", example = 42)
    ),
    request_body(
        content = PatchUserView,
        description = "Fields to change. All optional; an absent or `null` field is ignored, except `phone_number` where `null` or `\"\"` removes the phone (MAIR-480).",
        examples(
            ("Change e-mail and phone" = (value = json!({ "email": "j.dupont@mairie360.fr", "phone_number": "07 98 76 54 32", "phone_country": "FR" }))),
            ("Remove the phone" = (value = json!({ "phone_number": null })))
        )
    ),
    responses(
        (
            status = 200,
            description = "Account updated, or nothing to update.",
            body = String,
            content_type = "text/plain",
            example = json!("User patched successfully!")
        ),
        (
            status = 400,
            description = "Malformed JSON body, `userId` in the path not an integer, or a field breaking its rules: `first_name` / `last_name` 1 to 64 characters, no control character; `email` a valid address of at most 320 characters; `phone_number` not a valid number of `phone_country` (or a national number without `phone_country`, 32 characters at most); `phone_country` not an ISO 3166-1 alpha-2 code, or sent without `phone_number`; `password` 8 to 255 characters without control character. The body names the first invalid field.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `password`: must be at least 8 characters")
        ),
        (
            status = 401,
            description = "`Authorization` header missing, JWT invalid or expired, or session revoked.",
            body = String,
            content_type = "text/plain",
            example = json!("Unauthorized")
        ),
        (
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 409,
            description = "Another account already uses the new e-mail. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Another account already uses this e-mail address.")
        ),
        (
            status = 500,
            description = "The account could not be read or written, its new password could not be hashed (internal argon2 error), or its sessions could not be revoked after a password change.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/")]
pub async fn admin_patch_user(
    _: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: ValidatedJson<PatchUserView>,
) -> Result<impl Responder, PatchUserError> {
    let user_id = path.into_inner();
    let view = view.into_inner();
    let revoked = patch_user(state.clone(), user_id, view).await?;
    publish_revoked_sessions(state.get_redis(), &revoked).await;

    Ok(HttpResponse::Ok().body("User patched successfully!"))
}
