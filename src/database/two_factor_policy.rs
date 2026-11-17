//! Queries of the two-factor policy of the instance (MAIR-808): the single row of
//! `two_factor_policy`. Every change is audited by a database trigger.

use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};

// The four flags are the independent columns of `two_factor_policy`, not the states of one machine.
#[allow(clippy::struct_excessive_bools)]
/// The policy as stored.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct TwoFactorPolicyRecord {
    /// Every account must have a second factor.
    pub required: bool,
    /// The authenticator application (TOTP) is accepted.
    pub totp_allowed: bool,
    /// Passkeys are accepted.
    pub passkey_allowed: bool,
    /// Backup codes are accepted to finish a login.
    pub backup_codes_allowed: bool,
    /// Administrator who made the last change, `None` for the initial value.
    pub updated_by: Option<i32>,
    /// Date of the last change, RFC 3339.
    pub updated_at: String,
}

/// Reads the policy: always one row.
#[derive(serde::Deserialize)]
pub struct GetTwoFactorPolicyQueryView {
    params: Vec<QueryParam>,
}

impl GetTwoFactorPolicyQueryView {
    #[must_use]
    pub const fn new() -> Self {
        Self { params: Vec::new() }
    }
}

impl Default for GetTwoFactorPolicyQueryView {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequestDto for GetTwoFactorPolicyQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT required, totp_allowed, passkey_allowed, backup_codes_allowed, updated_by, \
                   to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at \
            FROM two_factor_policy WHERE id = 1\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Replaces the policy and records the author; returns the new row.
#[derive(serde::Deserialize)]
pub struct UpdateTwoFactorPolicyQueryView {
    params: Vec<QueryParam>,
}

impl UpdateTwoFactorPolicyQueryView {
    #[must_use]
    // The four flags are the independent columns of `two_factor_policy`, not the states of one machine.
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn new(
        admin_id: u64,
        required: bool,
        totp_allowed: bool,
        passkey_allowed: bool,
        backup_codes_allowed: bool,
    ) -> Self {
        Self {
            params: vec![
                QueryParam::Bool(required),
                QueryParam::Bool(totp_allowed),
                QueryParam::Bool(passkey_allowed),
                QueryParam::Bool(backup_codes_allowed),
                QueryParam::I32(id_to_sql(admin_id)),
            ],
        }
    }
}

impl ApiRequestDto for UpdateTwoFactorPolicyQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH t AS (\
            UPDATE two_factor_policy SET required = $1, totp_allowed = $2, passkey_allowed = $3, \
                backup_codes_allowed = $4, updated_by = $5, updated_at = now() WHERE id = 1 \
            RETURNING required, totp_allowed, passkey_allowed, backup_codes_allowed, updated_by, \
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at\
        ) SELECT row_to_json(t) FROM t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}
