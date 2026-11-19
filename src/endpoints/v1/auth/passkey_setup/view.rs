use crate::endpoints::v1::user::me::passkeys::view::MAX_LABEL_LENGTH;
use crate::endpoints::validation::{
    check_label, check_opaque, Validate, ValidationError, MAX_TOKEN_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use webauthn_rs::prelude::RegisterPublicKeyCredential;

/// The browser's answer to the registration challenge, with the enrolment token and the label of
/// the passkey.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeySetupView {
    /// Enrolment token returned with `2fa_setup_required`.
    #[schema(
        max_length = 512,
        example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE"
    )]
    token: String,
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

impl PasskeySetupView {
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub fn label(&self) -> &str {
        self.label.trim()
    }

    #[must_use]
    pub const fn credential(&self) -> &RegisterPublicKeyCredential {
        &self.credential
    }
}

impl std::fmt::Display for PasskeySetupView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PasskeySetupView {{ token: [PROTECTED], label: {}, credential: [PROTECTED] }}",
            self.label
        )
    }
}

impl Validate for PasskeySetupView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("token", &self.token, MAX_TOKEN_LENGTH)?;
        check_label("label", &self.label, MAX_LABEL_LENGTH)
    }
}

/// Answer of the enrolment: the session is open.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PasskeySetupCompletedView {
    /// Refresh token of the session just opened (the JWT is in the `Authorization` header).
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    pub refresh_token: String,
}

impl std::fmt::Display for PasskeySetupCompletedView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PasskeySetupCompletedView {{ refresh_token: [PROTECTED] }}"
        )
    }
}
