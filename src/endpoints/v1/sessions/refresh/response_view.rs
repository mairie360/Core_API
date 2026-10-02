use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Refresh token that replaces the one just presented (rotation, MAIR-390).
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshResponseView {
    /// New refresh token of the same session. The token sent in the request no longer works: the
    /// next call to `POST /api/v1/sessions/refresh` (or `/sessions/revoke`) must use this one.
    #[schema(example = "q3Vt9ZcX1yLw0aB7nE5kR2mH8sJ4dF6gP0uT3oI9vYc")]
    refresh_token: String,
}

impl RefreshResponseView {
    #[must_use]
    pub const fn new(refresh_token: String) -> Self {
        Self { refresh_token }
    }
}

impl Display for RefreshResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RefreshResponseView {{ refresh_token: [PROTECTED] }}")
    }
}
