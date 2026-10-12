use crate::endpoints::validation::{check_opaque, Validate, ValidationError, MAX_TOKEN_LENGTH};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use webauthn_rs::prelude::{PublicKeyCredential, RequestChallengeResponse};

/// Options of a passkey sign-in, to hand to `navigator.credentials.get()`.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeyLoginOptionsResponseView {
    /// Id of the pending ceremony, to send back with the assertion. Single use, expires after
    /// two minutes.
    #[schema(value_type = String, example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35")]
    challenge_id: Uuid,
    /// `PublicKeyCredentialRequestOptions` (`WebAuthn` Level 3 JSON form): `challenge` and the
    /// ids in it are base64url strings, `allowCredentials` is empty (discoverable credential:
    /// the browser lists the passkeys registered for this site), `userVerification` is
    /// `required`. Pass it through `PublicKeyCredential.parseRequestOptionsFromJSON()`.
    #[schema(value_type = Object)]
    public_key: RequestChallengeResponse,
}

impl PasskeyLoginOptionsResponseView {
    #[must_use]
    pub const fn new(challenge_id: Uuid, public_key: RequestChallengeResponse) -> Self {
        Self {
            challenge_id,
            public_key,
        }
    }
}

/// Assertion of a passkey, answering the options of `POST /api/v1/auth/passkey/options`.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeyLoginView {
    /// The `challenge_id` of the options this assertion answers.
    #[schema(value_type = String, example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35")]
    challenge_id: Uuid,
    /// The `PublicKeyCredential` returned by `navigator.credentials.get()`, as given by its
    /// `toJSON()`: `id`, `rawId`, `type`, `response` (`authenticatorData`, `clientDataJSON`,
    /// `signature`, `userHandle`) and `clientExtensionResults`, binary fields base64url.
    #[schema(value_type = Object)]
    credential: PublicKeyCredential,
    /// Free-form description of the device, stored on the session so the user recognises their
    /// connections in `GET /api/v1/sessions/`.
    #[schema(max_length = 512, example = "Safari 26 on iPhone")]
    device_info: String,
}

impl PasskeyLoginView {
    #[must_use]
    pub const fn challenge_id(&self) -> Uuid {
        self.challenge_id
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

impl Display for PasskeyLoginView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PasskeyLoginView {{ challenge_id: {}, credential: [PROTECTED], device_info: {} }}",
            self.challenge_id, self.device_info
        )
    }
}

impl Validate for PasskeyLoginView {
    fn validate(&self) -> Result<(), ValidationError> {
        // The assertion itself is checked by the WebAuthn verification (and a bad one answers
        // 401, not 400): only the stored text is validated here.
        check_opaque("device_info", &self.device_info, MAX_TOKEN_LENGTH)
    }
}
