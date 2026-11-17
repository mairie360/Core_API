use crate::database::two_factor_policy::TwoFactorPolicyRecord;
use crate::endpoints::validation::{Validate, ValidationError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Passkeys cannot be enrolled yet (MAIR-810): allowing them would let an administrator remove the
/// authenticator application while nothing can replace it, locking every agent out.
pub const PASSKEYS_AVAILABLE: bool = false;

// The four flags are the independent columns of `two_factor_policy`, not the states of one machine.
#[allow(clippy::struct_excessive_bools)]
/// Two-factor policy of the instance.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct TwoFactorPolicyView {
    /// `true` when every account must have a second factor: after the password, an agent without
    /// one only gets `2fa_setup_required` and can do nothing but enrol.
    #[schema(example = true)]
    pub required: bool,
    /// The authenticator application (TOTP) is an accepted mode.
    #[schema(example = true)]
    pub totp_allowed: bool,
    /// Passkeys are an accepted mode. Always `false` until passkeys are available.
    #[schema(example = false)]
    pub passkey_allowed: bool,
    /// Backup codes are accepted to finish a login (a recovery, not a mode of their own).
    #[schema(example = true)]
    pub backup_codes_allowed: bool,
    /// Id of the administrator who made the last change, `null` for the initial value.
    #[schema(example = 1)]
    pub updated_by: Option<i32>,
    /// Date of the last change, RFC 3339 in UTC.
    #[schema(example = "2026-11-17T09:30:00Z")]
    pub updated_at: String,
}

impl From<TwoFactorPolicyRecord> for TwoFactorPolicyView {
    fn from(record: TwoFactorPolicyRecord) -> Self {
        Self {
            required: record.required,
            totp_allowed: record.totp_allowed,
            passkey_allowed: record.passkey_allowed,
            backup_codes_allowed: record.backup_codes_allowed,
            updated_by: record.updated_by,
            updated_at: record.updated_at,
        }
    }
}

// The four flags are the independent columns of `two_factor_policy`, not the states of one machine.
#[allow(clippy::struct_excessive_bools)]
/// New two-factor policy: the whole policy is replaced. A mode that does not exist (SMS, e-mail)
/// is an unknown field and is refused.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateTwoFactorPolicyView {
    /// Make the second factor mandatory for every account.
    #[schema(example = true)]
    pub required: bool,
    /// Accept the authenticator application (TOTP).
    #[schema(example = true)]
    pub totp_allowed: bool,
    /// Accept passkeys. Refused (`400`) until passkeys are available.
    #[schema(example = false)]
    pub passkey_allowed: bool,
    /// Accept backup codes to finish a login.
    #[schema(example = true)]
    pub backup_codes_allowed: bool,
}

impl Validate for UpdateTwoFactorPolicyView {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.passkey_allowed && !PASSKEYS_AVAILABLE {
            return Err(ValidationError::new(
                "passkey_allowed",
                "passkeys are not available yet",
            ));
        }
        if !self.totp_allowed && !self.passkey_allowed {
            return Err(ValidationError::new(
                "totp_allowed",
                "at least one mode (totp_allowed or passkey_allowed) must stay allowed",
            ));
        }
        Ok(())
    }
}
