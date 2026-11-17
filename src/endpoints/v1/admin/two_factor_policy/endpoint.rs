use super::view::{TwoFactorPolicyView, UpdateTwoFactorPolicyView};
use crate::database::two_factor_policy::{
    GetTwoFactorPolicyQueryView, TwoFactorPolicyRecord, UpdateTwoFactorPolicyQueryView,
};
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::validation::ValidatedJson;
use actix_web::{get, http::StatusCode, put, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TwoFactorPolicyError {
    DatabaseError,
    /// Passkeys were allowed on an instance without `WebAuthn` configuration (MAIR-810): allowing
    /// them would let an administrator remove the authenticator application while nothing can
    /// replace it, locking every agent out.
    PasskeysUnavailable,
}

impl std::fmt::Display for TwoFactorPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => write!(f, "Database error occurred"),
            Self::PasskeysUnavailable => write!(
                f,
                "Invalid `passkey_allowed`: passkeys are not configured on this instance"
            ),
        }
    }
}

impl ResponseError for TwoFactorPolicyError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::PasskeysUnavailable => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    get,
    path = "",
    summary = "Read the two-factor policy of the instance",
    description = "Returns whether the second factor is mandatory, the accepted modes, who changed \
                   the policy last and when. A new instance starts with the second factor optional, \
                   the authenticator application and the backup codes allowed. Reserved to \
                   administrators.",
    responses(
        (
            status = 200,
            description = "The policy.",
            body = TwoFactorPolicyView,
            example = json!({
                "required": true,
                "totp_allowed": true,
                "passkey_allowed": false,
                "backup_codes_allowed": true,
                "updated_by": 1,
                "updated_at": "2026-11-17T09:30:00Z"
            })
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
            status = 500,
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Two-factor policy",
    security(("jwt" = []))
)]
#[get("")]
pub async fn get_two_factor_policy(
    _admin: AdminUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, TwoFactorPolicyError> {
    let record = state
        .get_smart_db()
        .fetch_one::<TwoFactorPolicyRecord, _>(&GetTwoFactorPolicyQueryView::new())
        .await
        .map_err(|e| {
            tracing::error!("Read the two-factor policy: {e}");
            TwoFactorPolicyError::DatabaseError
        })?;
    Ok(HttpResponse::Ok().json(TwoFactorPolicyView::from(record)))
}

#[utoipa::path(
    put,
    path = "",
    summary = "Replace the two-factor policy of the instance",
    description = "Replaces the whole policy. With `required: true`, an agent without a second \
                   factor who gives the right password gets `202` `2fa_setup_required` instead of \
                   a session, and can only enrol (`POST /api/v1/auth/2fa/setup` and `confirm`); \
                   agents who already have one are not affected. An agent can no longer turn the \
                   second factor off (`POST /api/v1/user/me/2fa/disable/` answers `403`). A mode \
                   that is not allowed any more is refused at the next login: the agent who used \
                   it enrols another one. Backup codes not allowed: they are refused at login.\n\n\
                   The change is recorded with the administrator and the date, and every change \
                   is kept in an append-only audit log of the database; an `AUDIT` line is also \
                   logged. A mode that does not exist (SMS, e-mail) is an unknown field. At least \
                   one of `totp_allowed` / `passkey_allowed` must be `true`, and `passkey_allowed` \
                   is refused while the instance has no `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN`. Reserved to administrators.",
    request_body(
        content = UpdateTwoFactorPolicyView,
        description = "The new policy.",
        example = json!({
            "required": true,
            "totp_allowed": true,
            "passkey_allowed": false,
            "backup_codes_allowed": true
        })
    ),
    responses(
        (
            status = 200,
            description = "The policy now in force.",
            body = TwoFactorPolicyView,
            example = json!({
                "required": true,
                "totp_allowed": true,
                "passkey_allowed": false,
                "backup_codes_allowed": true,
                "updated_by": 1,
                "updated_at": "2026-11-17T09:30:00Z"
            })
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing or unknown field (`sms`, `email`…), no mode left allowed, or `passkey_allowed: true` while the instance has no WebAuthn configuration.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `totp_allowed`: at least one mode (totp_allowed or passkey_allowed) must stay allowed")
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
            status = 500,
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Two-factor policy",
    security(("jwt" = []))
)]
#[put("")]
pub async fn update_two_factor_policy(
    admin: AdminUser,
    state: web::Data<AppState>,
    payload: ValidatedJson<UpdateTwoFactorPolicyView>,
    passkeys: Option<web::Data<crate::passkeys::Passkeys>>,
) -> Result<impl Responder, TwoFactorPolicyError> {
    let policy = payload.into_inner();
    if policy.passkey_allowed && passkeys.is_none() {
        return Err(TwoFactorPolicyError::PasskeysUnavailable);
    }
    let record = state
        .get_smart_db()
        .fetch_one::<TwoFactorPolicyRecord, _>(&UpdateTwoFactorPolicyQueryView::new(
            admin.id,
            policy.required,
            policy.totp_allowed,
            policy.passkey_allowed,
            policy.backup_codes_allowed,
        ))
        .await
        .map_err(|e| {
            tracing::error!("Update the two-factor policy: {e}");
            TwoFactorPolicyError::DatabaseError
        })?;
    tracing::warn!(
        admin_id = admin.id,
        required = policy.required,
        totp_allowed = policy.totp_allowed,
        passkey_allowed = policy.passkey_allowed,
        backup_codes_allowed = policy.backup_codes_allowed,
        "AUDIT: two-factor policy changed by an administrator"
    );
    Ok(HttpResponse::Ok().json(TwoFactorPolicyView::from(record)))
}
