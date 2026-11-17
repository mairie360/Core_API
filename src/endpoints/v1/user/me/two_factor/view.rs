use crate::endpoints::validation::{check_opaque, Validate, ValidationError, MAX_PASSWORD_LENGTH};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Longest code accepted: a 6-digit TOTP code or a backup code with its dash and some spaces.
pub const MAX_CODE_LENGTH: usize = 32;

/// State of the second factor of the caller.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TwoFactorStatusView {
    /// `true` once the authenticator application is confirmed.
    #[schema(example = true)]
    pub enabled: bool,
    /// Backup codes not spent yet; `0` when the second factor is off.
    #[schema(example = 8)]
    pub backup_codes_remaining: i32,
}

/// Start of the enrolment of the authenticator application.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TwoFactorSetupView {
    /// `otpauth://` URI, to show as a QR code.
    #[schema(
        example = "otpauth://totp/Mairie%20360%3Ajean%2Edupont%40mairie360%2Efr?secret=JBSWY3DPEHPK3PXP&issuer=Mairie%20360&algorithm=SHA1&digits=6&period=30"
    )]
    pub otpauth_uri: String,
    /// The same secret in base32, for the agent who cannot scan the QR code.
    #[schema(example = "JBSWY3DPEHPK3PXP")]
    pub secret: String,
}

/// A code of the authenticator application (or a backup code).
#[derive(Deserialize, Serialize, ToSchema)]
pub struct TwoFactorCodeView {
    /// 6-digit code of the application. Spaces around it are ignored.
    #[schema(max_length = 32, example = "492039")]
    code: String,
}

impl TwoFactorCodeView {
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }
}

impl Validate for TwoFactorCodeView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("code", &self.code, MAX_CODE_LENGTH)
    }
}

/// Request to turn the second factor off.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct DisableTwoFactorView {
    /// Current password of the account.
    #[schema(max_length = 255, format = Password, example = "MotDePasse!123")]
    password: String,
    /// A 6-digit code of the application, or an unspent backup code.
    #[schema(max_length = 32, example = "492039")]
    code: String,
}

impl DisableTwoFactorView {
    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }

    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }
}

impl Validate for DisableTwoFactorView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("password", &self.password, MAX_PASSWORD_LENGTH)?;
        check_opaque("code", &self.code, MAX_CODE_LENGTH)
    }
}

/// The backup codes, shown once.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BackupCodesView {
    /// Ten single-use codes (`xxxx-xxxx`). They are not stored in clear and cannot be shown
    /// again: the agent must keep them now.
    #[schema(example = json!(["k3m9-x7pq", "h4wn-2tre"]))]
    pub backup_codes: Vec<String>,
}
