use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Jeton de rafraîchissement à échanger contre un nouveau jeton d'accès.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshRequestView {
    /// Jeton de rafraîchissement obtenu au login, pour la session à renouveler.
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    refresh_token: String,
}

impl RefreshRequestView {
    pub fn refresh_token(&self) -> String {
        self.refresh_token.clone()
    }
}

impl Display for RefreshRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RefreshRequestView {{ refresh_token: {} }}",
            self.refresh_token
        )
    }
}
