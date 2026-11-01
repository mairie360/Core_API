use crate::endpoints::validation::{check_opaque, Validate, ValidationError, MAX_TOKEN_LENGTH};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Session to end, designated by its refresh token alone (no JWT needed).
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutView {
    /// Refresh token of the session, as returned by the login, the reset or the last refresh.
    #[schema(
        max_length = 512,
        example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE"
    )]
    refresh_token: String,
}

impl LogoutView {
    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}

impl Display for LogoutView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LogoutView {{ refresh_token: [PROTECTED] }}")
    }
}

impl Validate for LogoutView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("refresh_token", &self.refresh_token, MAX_TOKEN_LENGTH)
    }
}
