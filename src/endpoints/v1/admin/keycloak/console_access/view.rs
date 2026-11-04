use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A temporary access to the Keycloak admin console of the instance realm (MAIR-609).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct KeycloakConsoleAccessView {
    /// Link to open in the browser: `https://auth.<domain>/admin-access?token=<link token>`. The
    /// token is a signed JWT, valid `expires_in` seconds and **single use**: following the link
    /// sets the console cookie and redirects to `/admin/<realm>/console/`. Never stored, never
    /// logged; ask for a new link when it expired.
    #[schema(
        example = "https://auth.dev.mairie360-eip.fr/admin-access?token=eyJhbGciOiJFZERTQSIsInR5cCI6ImtjLWNvbnNvbGUtbGluaytqd3QifQ.eyJzdWIiOiIxIn0.c2lnbmF0dXJl"
    )]
    pub url: String,
    /// Seconds the link stays valid from now (120). The console session it opens then lasts 30
    /// minutes.
    #[schema(example = 120, minimum = 1)]
    pub expires_in: u64,
}
