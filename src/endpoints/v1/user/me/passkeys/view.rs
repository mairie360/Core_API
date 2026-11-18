use crate::endpoints::validation::{check_label, Validate, ValidationError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use webauthn_rs::prelude::RegisterPublicKeyCredential;

/// Longest label of a passkey (the column is `VARCHAR(100)`).
pub const MAX_LABEL_LENGTH: usize = 100;

/// A passkey of the caller. The key itself is never returned.
#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct PasskeyView {
    /// Identifier to give to `DELETE /api/v1/user/me/passkeys/{id}/`.
    #[schema(example = 12)]
    pub id: i32,
    /// Name the agent gave to the passkey.
    #[schema(example = "Town hall laptop")]
    pub label: String,
    /// Name of the passkey manager that holds it (iCloud Keychain, Google Password Manager, ...),
    /// deduced from the authenticator at registration; `null` when unknown. The front offers it as the
    /// name of the passkey.
    #[schema(example = "Google Password Manager")]
    pub provider: Option<String>,
    /// When the passkey was registered (ISO 8601).
    #[schema(example = "2026-11-17T09:30:00+00:00")]
    pub created_at: String,
    /// Last sign-in with this passkey (ISO 8601), `null` when it was never used.
    #[schema(example = "2026-11-18T08:02:11+00:00")]
    pub last_used_at: Option<String>,
}

/// Challenge to register a new passkey.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeyRegisterOptionsView {
    /// `WebAuthn` `PublicKeyCredentialCreationOptions`, as `navigator.credentials.create` expects
    /// them (user verification required, no attestation, passkeys already registered excluded).
    #[schema(value_type = Object, example = json!({
        "publicKey": {
            "rp": { "name": "Mairie 360", "id": "mairie360-eip.fr" },
            "challenge": "q1w2e3r4t5y6u7i8o9p0a1s2d3f4g5h6j7k8l9z0x1c",
            "pubKeyCredParams": [{ "type": "public-key", "alg": -7 }],
            "authenticatorSelection": { "userVerification": "required" },
            "attestation": "none"
        }
    }))]
    pub options: serde_json::Value,
}

/// The browser's answer to a registration challenge, with the label of the passkey.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeyRegisterView {
    /// Name of the passkey, to recognise it in the list (device, for example).
    #[schema(min_length = 1, max_length = 100, example = "Town hall laptop")]
    label: String,
    /// The `PublicKeyCredential` returned by `navigator.credentials.create`, serialized as the
    /// `WebAuthn` JSON form (base64url binary fields).
    #[schema(value_type = Object, example = json!({
        "id": "k3m9x7p2",
        "rawId": "k3m9x7p2",
        "type": "public-key",
        "response": {
            "attestationObject": "o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YQ",
            "clientDataJSON": "eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIn0"
        }
    }))]
    credential: RegisterPublicKeyCredential,
}

impl PasskeyRegisterView {
    #[must_use]
    pub fn label(&self) -> &str {
        self.label.trim()
    }

    #[must_use]
    pub const fn credential(&self) -> &RegisterPublicKeyCredential {
        &self.credential
    }
}

impl std::fmt::Display for PasskeyRegisterView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PasskeyRegisterView {{ label: {}, credential: [PROTECTED] }}",
            self.label
        )
    }
}

impl Validate for PasskeyRegisterView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_label("label", &self.label, MAX_LABEL_LENGTH)
    }
}
