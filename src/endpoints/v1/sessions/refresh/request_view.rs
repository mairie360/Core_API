use crate::endpoints::validation::{
    check_opaque, check_optional, Validate, ValidationError, MAX_TOKEN_LENGTH,
};

/// Longest device description, as at sign-in.
const MAX_DEVICE_INFO_LENGTH: usize = 512;
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
    /// User agent of the browser that renews the session, written to the sign-in journal and the
    /// alert e-mails when the token turns out to be reused. The request's `User-Agent` when absent.
    #[schema(
        max_length = 512,
        example = "Mozilla/5.0 (X11; Linux x86_64) Firefox/140.0"
    )]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device_info: Option<String>,
}

impl RefreshRequestView {
    pub fn refresh_token(&self) -> String {
        self.refresh_token.clone()
    }

    pub fn device_info(&self) -> Option<String> {
        self.device_info.clone().filter(|value| !value.is_empty())
    }
}

impl Display for RefreshRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RefreshRequestView {{ refresh_token: [PROTECTED] }}")
    }
}

impl Validate for RefreshRequestView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("refresh_token", &self.refresh_token, MAX_TOKEN_LENGTH)?;
        check_optional(self.device_info.as_deref(), |device_info| {
            check_opaque("device_info", device_info, MAX_DEVICE_INFO_LENGTH)
        })
    }
}
