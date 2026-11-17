use crate::endpoints::validation::{check_opaque, Validate, ValidationError, MAX_TOKEN_LENGTH};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Second step of a login.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginTwoFactorView {
    /// Login token returned by `POST /api/v1/auth/login` (or `POST /api/v1/auth/reset_password`).
    #[schema(
        max_length = 512,
        example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE"
    )]
    token: String,
    /// 6-digit code of the authenticator application, or an unspent backup code (`xxxx-xxxx`).
    #[schema(max_length = 32, example = "492039")]
    code: String,
}

impl LoginTwoFactorView {
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }
}

impl std::fmt::Display for LoginTwoFactorView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginTwoFactorView {{ token: [PROTECTED], code: [PROTECTED] }}"
        )
    }
}

impl Validate for LoginTwoFactorView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("token", &self.token, MAX_TOKEN_LENGTH)?;
        check_opaque("code", &self.code, 32)
    }
}
