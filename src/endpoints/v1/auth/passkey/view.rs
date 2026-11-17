use crate::endpoints::validation::{check_opaque, Validate, ValidationError, MAX_TOKEN_LENGTH};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use webauthn_rs::prelude::PublicKeyCredential;

/// Challenge of a passkey sign-in.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeyLoginOptionsView {
    /// Identifier of the ceremony: send it back, with the browser's answer, to
    /// `POST /api/v1/auth/passkey`. Single use, valid two minutes.
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    pub id: String,
    /// `WebAuthn` `PublicKeyCredentialRequestOptions`, as `navigator.credentials.get` expects them
    /// (`mediation: "conditional"`, user verification required, no `allowCredentials`: the
    /// account is found by the passkey, nobody types an e-mail).
    #[schema(value_type = Object, example = json!({
        "publicKey": {
            "challenge": "q1w2e3r4t5y6u7i8o9p0a1s2d3f4g5h6j7k8l9z0x1c",
            "timeout": 300000,
            "rpId": "mairie360-eip.fr",
            "userVerification": "required"
        },
        "mediation": "conditional"
    }))]
    pub options: serde_json::Value,
}

/// The answer of the browser to a passkey sign-in challenge.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeyLoginView {
    /// The `id` returned by `POST /api/v1/auth/passkey/options`.
    #[schema(
        max_length = 512,
        example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE"
    )]
    id: String,
    /// The `PublicKeyCredential` returned by `navigator.credentials.get`, serialized as the
    /// `WebAuthn` JSON form (base64url binary fields).
    #[schema(value_type = Object, example = json!({
        "id": "k3m9x7p2",
        "rawId": "k3m9x7p2",
        "type": "public-key",
        "response": {
            "authenticatorData": "example-authenticator-data",
            "clientDataJSON": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0In0",
            "signature": "MEUCIQ",
            "userHandle": "bWFpcmllMzYw"
        }
    }))]
    credential: PublicKeyCredential,
    /// Free description of the device, kept on the session like the one of a password login.
    #[schema(max_length = 512, example = "Chrome 140 sur Windows 11")]
    device_info: String,
}

impl PasskeyLoginView {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub const fn credential(&self) -> &PublicKeyCredential {
        &self.credential
    }

    #[must_use]
    pub fn device_info(&self) -> &str {
        &self.device_info
    }
}

impl std::fmt::Display for PasskeyLoginView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PasskeyLoginView {{ id: [PROTECTED], credential: [PROTECTED] }}"
        )
    }
}

impl Validate for PasskeyLoginView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("id", &self.id, MAX_TOKEN_LENGTH)?;
        check_opaque("device_info", &self.device_info, 512)
    }
}
