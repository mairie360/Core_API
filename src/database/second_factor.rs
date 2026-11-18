//! Queries of the second factor (MAIR-805): `user_totp` and `user_backup_codes`.
//!
//! The sealed secret travels as base64 text (`encode` / `decode` in SQL): the query parameters
//! have no byte-array variant. Every write that must be atomic (spending a time step, a backup
//! code, confirming an enrolment) is one `UPDATE … RETURNING`, so two simultaneous requests cannot
//! both succeed: the second one gets `DbError::NotFound`.

use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};

macro_rules! user_view {
    ($(#[$doc:meta])* $name:ident, $sql:expr) => {
        $(#[$doc])*
        #[derive(serde::Deserialize)]
        pub struct $name {
            params: Vec<QueryParam>,
        }

        impl $name {
            #[must_use]
            pub fn new(user_id: u64) -> Self {
                Self {
                    params: vec![QueryParam::I32(id_to_sql(user_id))],
                }
            }
        }

        impl ApiRequestDto for $name {
            fn query_sql(&self) -> &'static str {
                $sql
            }

            fn query_params(&self) -> &[QueryParam] {
                &self.params
            }
        }
    };
}

/// Row of [`GetTotpQueryView`].
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct TotpRecord {
    /// Sealed secret (`nonce || ciphertext || tag`), base64.
    pub secret: String,
    /// `false` while the enrolment waits for its first code.
    pub confirmed: bool,
    /// Last accepted 30-second time step, `None` before the first use.
    pub last_used_step: Option<i64>,
}

user_view!(
    /// The TOTP record of a user; `DbError::NotFound` when none was started.
    GetTotpQueryView,
    "SELECT row_to_json(t) FROM (\
        SELECT encode(secret_encrypted, 'base64') AS secret, confirmed_at IS NOT NULL AS confirmed, \
               last_used_step \
        FROM user_totp WHERE user_id = $1\
    ) t"
);

user_view!(
    /// Whether the second factor is active (`true`) for the user; always one row.
    IsTotpEnabledQueryView,
    "SELECT to_json(EXISTS (SELECT 1 FROM user_totp WHERE user_id = $1 AND confirmed_at IS NOT NULL))"
);

/// Stores a new pending secret, replacing a pending one. `DbError::NotFound` when the second
/// factor is already active (the enrolment must start with a disable).
#[derive(serde::Deserialize)]
pub struct StartTotpEnrolmentQueryView {
    params: Vec<QueryParam>,
}

impl StartTotpEnrolmentQueryView {
    #[must_use]
    pub fn new(user_id: u64, sealed_secret_base64: &str) -> Self {
        Self {
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::Text(sealed_secret_base64.to_string()),
            ],
        }
    }
}

impl ApiRequestDto for StartTotpEnrolmentQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH t AS (\
            INSERT INTO user_totp (user_id, secret_encrypted) VALUES ($1, decode($2, 'base64')) \
            ON CONFLICT (user_id) DO UPDATE SET secret_encrypted = EXCLUDED.secret_encrypted, \
                last_used_step = NULL, updated_at = now() \
            WHERE user_totp.confirmed_at IS NULL \
            RETURNING user_id\
        ) SELECT row_to_json(t) FROM t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Row of the `RETURNING` queries: only proves that a row was written.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Written {
    pub user_id: Option<i32>,
    pub id: Option<i32>,
}

/// Activates a pending enrolment and records the step of its first code.
///
/// Only the pending secret the code was checked against is activated: a setup that replaced it in
/// the meantime leaves the row alone and the caller gets `DbError::NotFound`.
#[derive(serde::Deserialize)]
pub struct ConfirmTotpQueryView {
    params: Vec<QueryParam>,
}

impl ConfirmTotpQueryView {
    #[must_use]
    pub fn new(user_id: u64, step: u64, sealed_secret_base64: &str) -> Self {
        Self {
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::I64(i64::try_from(step).unwrap_or(i64::MAX)),
                QueryParam::Text(sealed_secret_base64.to_string()),
            ],
        }
    }
}

impl ApiRequestDto for ConfirmTotpQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH t AS (\
            UPDATE user_totp SET confirmed_at = now(), last_used_step = $2, updated_at = now() \
            WHERE user_id = $1 AND confirmed_at IS NULL AND secret_encrypted = decode($3, 'base64') \
            RETURNING user_id\
        ) SELECT row_to_json(t) FROM t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Spends the time step of an accepted code.
///
/// Only a step later than the last one is written, so a replay, even a simultaneous one, gets
/// `DbError::NotFound`. The step is spent on the secret that supplied the code: a factor reset and
/// re-enrolled in between (another secret) is left alone, so a delayed verification cannot move
/// its anti-replay step.
#[derive(serde::Deserialize)]
pub struct UseTotpStepQueryView {
    params: Vec<QueryParam>,
}

impl UseTotpStepQueryView {
    #[must_use]
    pub fn new(user_id: u64, step: u64, sealed_secret_base64: &str) -> Self {
        Self {
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::I64(i64::try_from(step).unwrap_or(i64::MAX)),
                QueryParam::Text(sealed_secret_base64.to_string()),
            ],
        }
    }
}

impl ApiRequestDto for UseTotpStepQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH t AS (\
            UPDATE user_totp SET last_used_step = $2, updated_at = now() \
            WHERE user_id = $1 AND confirmed_at IS NOT NULL \
              AND secret_encrypted = decode($3, 'base64') \
              AND (last_used_step IS NULL OR last_used_step < $2) RETURNING user_id\
        ) SELECT row_to_json(t) FROM t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

user_view!(
    /// Takes a shared lock on the active factor of the user until the transaction ends.
    ///
    /// `DbError::NotFound` when there is none. A reset (`DELETE FROM user_totp`) waits for the
    /// transaction that holds it, so a session created under this lock is committed before the
    /// reset revokes the sessions (MAIR-805).
    LockActiveTotpQueryView,
    "SELECT row_to_json(t) FROM (\
        SELECT user_id FROM user_totp WHERE user_id = $1 AND confirmed_at IS NOT NULL FOR SHARE\
    ) t"
);

user_view!(
    /// Removes the TOTP record (disable, or reset by an administrator).
    DeleteTotpQueryView,
    "DELETE FROM user_totp WHERE user_id = $1"
);

user_view!(
    /// Removes every backup code of the user.
    DeleteBackupCodesQueryView,
    "DELETE FROM user_backup_codes WHERE user_id = $1"
);

/// Row of [`ListBackupCodesQueryView`].
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct BackupCodeRecord {
    pub id: i32,
    pub code_hash: String,
}

user_view!(
    /// The backup codes not spent yet.
    ListBackupCodesQueryView,
    "SELECT row_to_json(t) FROM (\
        SELECT id, code_hash FROM user_backup_codes WHERE user_id = $1 AND used_at IS NULL ORDER BY id\
    ) t"
);

user_view!(
    /// The backup codes already spent, to tell a spent code from a wrong one at login.
    ListSpentBackupCodesQueryView,
    "SELECT row_to_json(t) FROM (\
        SELECT id, code_hash FROM user_backup_codes WHERE user_id = $1 AND used_at IS NOT NULL ORDER BY id\
    ) t"
);

user_view!(
    /// Number of backup codes not spent yet; always one row.
    CountBackupCodesQueryView,
    "SELECT to_json((SELECT count(*)::int FROM user_backup_codes WHERE user_id = $1 AND used_at IS NULL))"
);

/// Stores the hash of one backup code.
#[derive(serde::Deserialize)]
pub struct InsertBackupCodeQueryView {
    params: Vec<QueryParam>,
}

impl InsertBackupCodeQueryView {
    #[must_use]
    pub fn new(user_id: u64, code_hash: &str) -> Self {
        Self {
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::Text(code_hash.to_string()),
            ],
        }
    }
}

impl ApiRequestDto for InsertBackupCodeQueryView {
    fn query_sql(&self) -> &'static str {
        "INSERT INTO user_backup_codes (user_id, code_hash) VALUES ($1, $2)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Spends a backup code. `DbError::NotFound` when it was spent meanwhile.
#[derive(serde::Deserialize)]
pub struct SpendBackupCodeQueryView {
    params: Vec<QueryParam>,
}

impl SpendBackupCodeQueryView {
    #[must_use]
    pub fn new(user_id: u64, code_id: i32) -> Self {
        Self {
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::I32(code_id),
            ],
        }
    }
}

impl ApiRequestDto for SpendBackupCodeQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH c AS (\
            UPDATE user_backup_codes SET used_at = now() \
            WHERE user_id = $1 AND id = $2 AND used_at IS NULL RETURNING id\
        ) SELECT row_to_json(c) FROM c"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Row of [`GetAccountEmailQueryView`].
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct AccountEmail {
    pub email: String,
}

user_view!(
    /// E-mail of an active account, the label of the authenticator entry.
    GetAccountEmailQueryView,
    "SELECT row_to_json(t) FROM (\
        SELECT email FROM users WHERE id = $1 AND NOT COALESCE(is_archived, FALSE)\
    ) t"
);
