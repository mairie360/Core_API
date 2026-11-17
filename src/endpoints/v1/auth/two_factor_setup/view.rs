use crate::endpoints::validation::{check_opaque, Validate, ValidationError, MAX_TOKEN_LENGTH};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Enrolment token of an agent who must set up a second factor.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SetupTokenView {
    /// Enrolment token returned with `2fa_setup_required` by `POST /api/v1/auth/login` (or
    /// `POST /api/v1/auth/reset_password`).
    #[schema(
        max_length = 512,
        example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE"
    )]
    token: String,
}

impl SetupTokenView {
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl std::fmt::Display for SetupTokenView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SetupTokenView {{ token: [PROTECTED] }}")
    }
}

impl Validate for SetupTokenView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("token", &self.token, MAX_TOKEN_LENGTH)
    }
}

/// First code of the authenticator application, with the enrolment token.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SetupConfirmView {
    /// Enrolment token returned with `2fa_setup_required`.
    #[schema(
        max_length = 512,
        example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE"
    )]
    token: String,
    /// 6-digit code shown by the authenticator application once the secret is added.
    #[schema(max_length = 32, example = "492039")]
    code: String,
}

impl SetupConfirmView {
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }
}

impl std::fmt::Display for SetupConfirmView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SetupConfirmView {{ token: [PROTECTED], code: [PROTECTED] }}"
        )
    }
}

impl Validate for SetupConfirmView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("token", &self.token, MAX_TOKEN_LENGTH)?;
        check_opaque("code", &self.code, 32)
    }
}

/// Answer of the confirmation: the session is open.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SetupCompletedView {
    /// Refresh token of the session just opened (the JWT is in the `Authorization` header).
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    pub refresh_token: String,
    /// The ten single-use backup codes (`xxxx-xxxx`). **This is the only time they are visible.**
    #[schema(example = json!(["a1b2-c3d4", "e5f6-0718"]))]
    pub backup_codes: Vec<String>,
}

impl std::fmt::Display for SetupCompletedView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SetupCompletedView {{ refresh_token: [PROTECTED], backup_codes: [PROTECTED] }}"
        )
    }
}
