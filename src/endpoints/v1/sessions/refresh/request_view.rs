use crate::endpoints::validation::{check_opaque, Validate, ValidationError, MAX_TOKEN_LENGTH};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Refresh token to exchange for a new access token.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshRequestView {
    /// Refresh token obtained at sign-in, for the session to renew.
    #[schema(
        max_length = 512,
        example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE"
    )]
    refresh_token: String,
}

impl RefreshRequestView {
    pub fn refresh_token(&self) -> String {
        self.refresh_token.clone()
    }
}

impl Display for RefreshRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RefreshRequestView {{ refresh_token: [PROTECTED] }}")
    }
}

impl Validate for RefreshRequestView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("refresh_token", &self.refresh_token, MAX_TOKEN_LENGTH)
    }
}
