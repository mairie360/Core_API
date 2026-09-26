use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Keycloak authorization code to redeem for a Mairie 360 session.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct KeycloakLoginView {
    /// Authorization code Keycloak appended to the redirect URI after the user signed in.
    /// Single use and short-lived (one minute by default in Keycloak).
    #[schema(
        example = "7c1e0f5a-2b8d-4f3e-9a61-d4c2b7e8f901.3b5d9e2a-6f14-4c8b-a7d0-1e9f2c4b6a83"
    )]
    code: String,
    /// Redirect URI sent in the authorization request, byte for byte: Keycloak refuses the code
    /// otherwise.
    #[schema(format = "uri", example = "https://login.mairie360.fr/auth/callback")]
    redirect_uri: String,
    /// PKCE verifier matching the `code_challenge` of the authorization request. Omit it (or
    /// send `null`) only if the request carried no challenge.
    #[schema(nullable, example = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")]
    #[serde(default)]
    code_verifier: Option<String>,
    /// Nonce sent in the authorization request. When present, the ID token must carry the same
    /// value; omit it (or send `null`) only if the request carried no nonce.
    #[schema(nullable, example = "n-0S6_WzA2Mj")]
    #[serde(default)]
    nonce: Option<String>,
    /// Free-form description of the device, stored on the session so the user recognises their
    /// connections in `GET /api/v1/sessions/`.
    #[schema(example = "Firefox 142 on Ubuntu 24.04")]
    device_info: String,
}

impl KeycloakLoginView {
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    #[must_use]
    pub fn code_verifier(&self) -> Option<&str> {
        self.code_verifier.as_deref()
    }

    #[must_use]
    pub fn nonce(&self) -> Option<&str> {
        self.nonce.as_deref()
    }

    #[must_use]
    pub fn device_info(&self) -> &str {
        &self.device_info
    }
}
