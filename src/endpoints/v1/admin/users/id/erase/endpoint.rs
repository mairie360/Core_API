//! Erasure of a user (GDPR art. 17, MAIR-289), distinct from the archiving of
//! `DELETE /api/v1/admin/users/{userId}/`: `anonymize_user()` of the database follows the
//! `erasure` of every column of `gdpr/inventory.yaml` (identity cleared, memberships deleted,
//! private events deleted, groups / projects / events handed over to an administrator, audit
//! identity hashed). Core publishes the sessions it closed to the revocation list.
use crate::database::users::erasure::{
    AnonymizeUserQueryView, AnonymizeUserResult, ERASURE_REFUSED, UNKNOWN_USER,
};
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::db_error;
use crate::endpoints::export_data::sqlstate;
use crate::endpoints::v1::admin::users::id::erase::view::ErasureView;
use crate::session_revocation::publish_revoked_sessions;
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum EraseUserError {
    UnknownUser,
    Refused,
    DatabaseError,
}

impl std::fmt::Display for EraseUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::Refused => write!(
                f,
                "This account cannot be erased: it is the seeded administrator, or no other active administrator can take over its groups, projects and events"
            ),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for EraseUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::Refused => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// `anonymize_user()`, then the closed sessions on the revocation list (MAIR-264). The boolean
/// is `false` when Redis did not accept every revoked session.
async fn anonymize_in_core(
    state: &AppState,
    user_id: u64,
) -> Result<(AnonymizeUserResult, bool), EraseUserError> {
    let result: AnonymizeUserResult = state
        .get_smart_db()
        .fetch_one(&AnonymizeUserQueryView::new(user_id))
        .await
        .map_err(|error| match sqlstate(&error).as_deref() {
            Some(UNKNOWN_USER) => EraseUserError::UnknownUser,
            Some(ERASURE_REFUSED) => EraseUserError::Refused,
            _ => {
                db_error::log("erase user", &error);
                EraseUserError::DatabaseError
            }
        })?;
    let published = publish_revoked_sessions(state.get_redis(), &result.revoked_sessions).await;
    Ok((result, published))
}

async fn erase_user(state: &AppState, user_id: u64) -> Result<ErasureView, EraseUserError> {
    let (result, published) = anonymize_in_core(state, user_id).await?;
    Ok(ErasureView::new(&result, published))
}

#[utoipa::path(
    post,
    path = "erase",
    summary = "Erase a user (administration, GDPR)",
    description = "Right to erasure (GDPR art. 17, MAIR-289). Unlike `DELETE` (archiving), the \
                   account loses its identity for good: `anonymize_user()` of the database applies \
                   the `erasure` the personal data inventory decides for every column. Names, \
                   e-mail, phone, photo and biography are cleared and the password removed; \
                   memberships, sessions, preferences and SSO links are deleted; private events \
                   are deleted; groups, projects and the other events are handed over to an \
                   active administrator; shared content (messages, comments) stays for the other \
                   members without its author's identity; the identity in the audit log is \
                   replaced by a hash. The row stays, archived, so that references keep working, \
                   and cannot be restored. Administrators only. Erasing an account twice changes \
                   nothing (`already_anonymized`).\n\n\
                   The sessions closed are published to the revocation list, so their JWTs are \
                   refused by every API at once; `revocation_published` is `false` when Redis did \
                   not accept every one of them (Core refuses them anyway, the other APIs only \
                   until they expire).",
    params(("userId" = u64, Path, description = "User id.", example = 42)),
    responses(
        (status = 200, description = "Account erased (or already anonymized).", body = ErasureView),
        (status = 400, description = "`userId` in the path is not an integer.", body = String, content_type = "text/plain", example = json!("can not parse \"abc\" to a u64")),
        (status = 401, description = "`Authorization` header missing, invalid or expired JWT, or revoked session.", body = String, content_type = "text/plain", example = json!("Jeton expiré")),
        (status = 403, description = "The user is authenticated but is not an administrator.", body = String, content_type = "text/plain", example = json!("Forbidden: User is not an admin.")),
        (status = 404, description = "No account has this id.", body = String, content_type = "text/plain", example = json!("Unknown user")),
        (status = 409, description = "The account is the seeded administrator, or no other active administrator can take over its groups, projects and events. Nothing is changed.", body = String, content_type = "text/plain", example = json!("This account cannot be erased: it is the seeded administrator, or no other active administrator can take over its groups, projects and events")),
        (status = 500, description = "Database error. Nothing is changed.", body = String, content_type = "text/plain", example = json!("Database error occurred")),
    ),
    tag = "Admin - Users",
    security(("jwt" = []))
)]
#[post("/erase")]
pub async fn admin_erase_user(
    _: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, EraseUserError> {
    let erased = erase_user(&state, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(erased))
}
