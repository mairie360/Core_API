use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Session de l'utilisateur connecté à révoquer, désignée par son jeton de rafraîchissement.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RevokeRequestView {
    /// Jeton de rafraîchissement de la session à révoquer.
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    pub refresh_token: String,
}

impl RevokeRequestView {
    pub fn refresh_token(&self) -> String {
        self.refresh_token.clone()
    }
}

impl Display for RevokeRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeRequestView {{ refresh_token: {} }}",
            self.refresh_token
        )
    }
}
