use crate::database::two_factor_policy::TwoFactorPolicyRecord;
use crate::endpoints::validation::{
    check_opaque, Validate, ValidationError, MAX_EMAIL_LENGTH, MAX_PASSWORD_LENGTH,
    MAX_TOKEN_LENGTH,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Sign-in credentials.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginView {
    /// E-mail address of the account.
    #[schema(max_length = 320, format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
    /// Plain-text password, sent as is over the TLS channel.
    #[schema(max_length = 255, format = Password, example = "MotDePasse!123")]
    password: String,
    /// Free description of the device, kept on the session so that the user
    /// recognises their sign-ins in `GET /api/v1/sessions/`.
    #[schema(max_length = 512, example = "Chrome 140 sur Windows 11")]
    device_info: String,
}

impl LoginView {
    #[must_use]
    pub fn email(&self) -> String {
        self.email.clone()
    }

    #[must_use]
    pub fn password(&self) -> String {
        self.password.clone()
    }

    #[must_use]
    pub fn device_info(&self) -> String {
        self.device_info.clone()
    }
}

impl Display for LoginView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginView {{ email: [PROTECTED], password: [PROTECTED], device_info: {} }}",
            self.device_info
        )
    }
}

/// Successful sign-in: refresh token of the opened session.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginResponseView {
    /// Opaque token used to get a new JWT through `POST /api/v1/sessions/refresh`,
    /// without asking for the password again. Keep it on the client side, never in a URL.
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    refresh_token: String,
    /// Only after a password sign-in: what the login screen needs to propose a passkey. Absent after a
    /// sign-in by passkey, which needs no proposal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    passkey: Option<PasskeyOfferView>,
}

/// Whether to propose a passkey right after a password sign-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PasskeyOfferView {
    /// Passkeys are usable: the instance has a WebAuthn relying party and the administrator accepts them.
    #[schema(example = true)]
    pub allowed: bool,
    /// The account already has at least one passkey.
    #[schema(example = false)]
    pub registered: bool,
}

impl LoginResponseView {
    #[must_use]
    pub const fn new(refresh_token: String) -> Self {
        Self {
            refresh_token,
            passkey: None,
        }
    }

    /// Adds the passkey proposal data (password sign-ins only).
    #[must_use]
    pub const fn with_passkey(mut self, offer: PasskeyOfferView) -> Self {
        self.passkey = Some(offer);
        self
    }

    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}

impl Display for LoginResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LoginResponseView {{ refresh_token: [PROTECTED] }}")
    }
}

impl From<String> for LoginResponseView {
    fn from(token: String) -> Self {
        Self::new(token)
    }
}

/// First sign-in: token to present to choose a new password before a session is opened.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginFirstConnectionResponseView {
    /// Single-use first sign-in token, to present to
    /// `POST /api/v1/auth/force_change_password` to choose a password.
    #[schema(
        pattern = r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
        example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35"
    )]
    token: String,
}

impl LoginFirstConnectionResponseView {
    #[must_use]
    pub const fn new(token: String) -> Self {
        Self { token }
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl Display for LoginFirstConnectionResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ \"token\": [PROTECTED] }}")
    }
}

impl From<String> for LoginFirstConnectionResponseView {
    fn from(token: String) -> Self {
        Self { token }
    }
}

/// What the administrator allows, given to the agent who has to enrol so that the screen only offers
/// what will be accepted (the agent has no session yet, so it cannot read the policy itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TwoFactorSetupPolicyView {
    /// The authenticator application (TOTP) is an accepted mode.
    #[schema(example = true)]
    pub totp_allowed: bool,
    /// Passkeys are an accepted mode.
    #[schema(example = false)]
    pub passkey_allowed: bool,
    /// Backup codes are accepted to finish a login: when `false`, none must be shown.
    #[schema(example = true)]
    pub backup_codes_allowed: bool,
}

impl From<&TwoFactorPolicyRecord> for TwoFactorSetupPolicyView {
    fn from(record: &TwoFactorPolicyRecord) -> Self {
        Self {
            totp_allowed: record.totp_allowed,
            passkey_allowed: record.passkey_allowed,
            backup_codes_allowed: record.backup_codes_allowed,
        }
    }
}

/// Answer of `POST /auth/login` (and of `POST /auth/reset_password`) for an account protected by
/// the second factor: no session yet, only the token of the second step.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct TwoFactorRequiredView {
    /// `2fa_required`: give a code to `POST /api/v1/auth/login/2fa`. `2fa_setup_required` (the
    /// instance makes the second factor mandatory and the account has none): enrol with
    /// `POST /api/v1/auth/2fa/setup` then `POST /api/v1/auth/2fa/confirm`.
    #[schema(pattern = r"^2fa_(setup_)?required$", example = "2fa_required")]
    status: String,
    /// Login or enrolment token: valid five minutes, usable on the call `status` names and
    /// nowhere else (it opens no other route). Keep it on the client side, never in a URL.
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    token: String,
    /// Only with `2fa_setup_required`: the modes the administrator allows, to offer nothing else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    policy: Option<TwoFactorSetupPolicyView>,
}

impl TwoFactorRequiredView {
    #[must_use]
    pub fn new(token: String) -> Self {
        Self {
            status: "2fa_required".to_string(),
            token,
            policy: None,
        }
    }

    /// The enrolment answer: the account must set up a second factor before any session.
    #[must_use]
    pub fn setup(token: String, policy: TwoFactorSetupPolicyView) -> Self {
        Self {
            status: "2fa_setup_required".to_string(),
            token,
            policy: Some(policy),
        }
    }
}

impl Validate for LoginView {
    fn validate(&self) -> Result<(), ValidationError> {
        // Only length and control characters: a malformed address simply matches no account (401).
        check_opaque("email", &self.email, MAX_EMAIL_LENGTH)?;
        check_opaque("password", &self.password, MAX_PASSWORD_LENGTH)?;
        check_opaque("device_info", &self.device_info, MAX_TOKEN_LENGTH)
    }
}
