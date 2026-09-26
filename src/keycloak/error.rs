use std::fmt::{Display, Formatter};

/// Failure while signing a user in through Keycloak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeycloakError {
    /// Keycloak refused the authorization code (unknown, expired, already used, wrong
    /// `redirect_uri` or PKCE verifier).
    InvalidGrant,
    /// The `id_token` is malformed, badly signed, expired, or issued for another realm, client
    /// or nonce.
    InvalidIdToken,
    /// The token has no e-mail, or Keycloak has not verified it: it cannot be matched safely
    /// against a Mairie 360 account.
    EmailNotVerified,
    /// Keycloak could not be reached, answered with an unexpected status or body, or rejected
    /// Core's own client credentials.
    Unavailable,
}

impl Display for KeycloakError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGrant => write!(f, "Keycloak rejected the authorization code."),
            Self::InvalidIdToken => write!(f, "Invalid Keycloak ID token."),
            Self::EmailNotVerified => {
                write!(f, "The Keycloak account has no verified e-mail address.")
            }
            Self::Unavailable => write!(f, "Keycloak is unavailable."),
        }
    }
}

impl std::error::Error for KeycloakError {}
