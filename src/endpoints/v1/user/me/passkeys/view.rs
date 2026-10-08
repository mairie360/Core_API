use crate::database::passkeys::PasskeySummary;
use crate::endpoints::validation::{check_label, Validate, ValidationError};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use webauthn_rs::prelude::{CreationChallengeResponse, RegisterPublicKeyCredential};

/// `user_passkeys.label` is `VARCHAR(100)`.
pub const MAX_PASSKEY_LABEL_LENGTH: usize = 100;

/// Options of a passkey registration, to hand to `navigator.credentials.create()`.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeyRegistrationOptionsResponseView {
    /// Id of the pending ceremony, to send back with the attestation. Single use, expires after
    /// two minutes, bound to the account that asked for it.
    #[schema(value_type = String, example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35")]
    challenge_id: Uuid,
    /// `PublicKeyCredentialCreationOptions` (`WebAuthn` Level 3 JSON form): `challenge`,
    /// `user.id` and `excludeCredentials[].id` are base64url strings, `user.name` is the e-mail,
    /// `user.displayName` the full name, `userVerification` is `required` and the passkeys
    /// already registered are excluded. Pass it through
    /// `PublicKeyCredential.parseCreationOptionsFromJSON()`.
    #[schema(value_type = Object)]
    public_key: CreationChallengeResponse,
}

impl PasskeyRegistrationOptionsResponseView {
    #[must_use]
    pub const fn new(challenge_id: Uuid, public_key: CreationChallengeResponse) -> Self {
        Self {
            challenge_id,
            public_key,
        }
    }
}

/// Attestation of a new passkey, answering the options of
/// `POST /api/v1/user/me/passkeys/options`.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegisterPasskeyView {
    /// The `challenge_id` of the options this attestation answers.
    #[schema(value_type = String, example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35")]
    challenge_id: Uuid,
    /// Label the user gives the passkey (1 to 100 characters, no control character), shown in
    /// the list so they recognise the device.
    #[schema(min_length = 1, max_length = 100, example = "iPhone de Jean")]
    label: String,
    /// The `PublicKeyCredential` returned by `navigator.credentials.create()`, as given by its
    /// `toJSON()`: `id`, `rawId`, `type`, `response` (`attestationObject`, `clientDataJSON`,
    /// `transports`) and `clientExtensionResults`, binary fields base64url.
    #[schema(value_type = Object)]
    credential: RegisterPublicKeyCredential,
}

impl RegisterPasskeyView {
    #[must_use]
    pub const fn challenge_id(&self) -> Uuid {
        self.challenge_id
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub const fn credential(&self) -> &RegisterPublicKeyCredential {
        &self.credential
    }
}

impl Display for RegisterPasskeyView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegisterPasskeyView {{ challenge_id: {}, label: {}, credential: [PROTECTED] }}",
            self.challenge_id, self.label
        )
    }
}

impl Validate for RegisterPasskeyView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_label("label", &self.label, MAX_PASSKEY_LABEL_LENGTH)
    }
}

/// The passkeys of the signed-in user.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeyListResponseView {
    /// Oldest first. Never the public keys.
    passkeys: Vec<PasskeySummary>,
}

impl PasskeyListResponseView {
    #[must_use]
    pub const fn new(passkeys: Vec<PasskeySummary>) -> Self {
        Self { passkeys }
    }

    #[must_use]
    pub fn passkeys(&self) -> &[PasskeySummary] {
        &self.passkeys
    }
}
