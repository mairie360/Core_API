//! Two-factor authentication flows shared by the login and the account routes (MAIR-805).
//!
//! The primitives (TOTP, sealing, backup codes) live in [`crate::second_factor`]; this module
//! runs them against the database and Redis:
//!
//! * [`verify_code`] checks a TOTP code or a backup code of a user, spending it atomically;
//! * [`issue_backup_codes`] replaces the backup codes of a user;
//! * the **login token** is the intermediate credential `POST /auth/login` returns for a protected
//!   account: random, kept in Redis for five minutes, usable only on `POST /auth/login/2fa`
//!   (it opens no route), single use, and burnt after five wrong codes.

use crate::database::second_factor::{
    BackupCodeRecord, CountBackupCodesQueryView, DeleteBackupCodesQueryView, GetTotpQueryView,
    InsertBackupCodeQueryView, IsTotpEnabledQueryView, ListBackupCodesQueryView,
    SpendBackupCodeQueryView, TotpRecord, UseTotpStepQueryView, Written,
};
use crate::passwords;
use crate::refresh_token;
use crate::second_factor::{self, SecretCipher};
use base64::{engine::general_purpose, Engine as _};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::smart_db::{SmartDatabase, SmartTransaction};
use serde::{Deserialize, Serialize};

/// Lifetime of the login token: five minutes to type the code.
pub const LOGIN_TOKEN_TTL_SECONDS: u64 = 5 * 60;
/// Wrong codes after which the login token is burnt.
pub const MAX_LOGIN_FAILURES: u32 = 5;

/// Why a code was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyError {
    /// Wrong, expired, replayed or already spent code.
    InvalidCode,
    /// The account has no active second factor.
    NotEnabled,
    /// Database or sealing failure (logged where it happens).
    Internal,
}

fn internal(context: &str, e: &impl std::fmt::Display) -> VerifyError {
    tracing::error!("Second factor: {context}: {e}");
    VerifyError::Internal
}

/// Whether `user_id` has an active second factor.
///
/// # Errors
///
/// [`ApiLibError`] when the database fails.
pub async fn is_enabled(db: &SmartDatabase, user_id: u64) -> Result<bool, ApiLibError> {
    db.fetch_one::<bool, _>(&IsTotpEnabledQueryView::new(user_id))
        .await
}

/// Number of backup codes of `user_id` that are not spent yet.
///
/// # Errors
///
/// [`ApiLibError`] when the database fails.
pub async fn backup_codes_remaining(db: &SmartDatabase, user_id: u64) -> Result<i32, ApiLibError> {
    db.fetch_one::<i32, _>(&CountBackupCodesQueryView::new(user_id))
        .await
}

/// Checks `code`, a TOTP code or a backup code, for `user_id`, and spends it.
///
/// # Errors
///
/// [`VerifyError::InvalidCode`] for a code that is wrong, replayed or spent, [`VerifyError::NotEnabled`]
/// when the user has no active second factor, [`VerifyError::Internal`] otherwise.
pub async fn verify_code(
    db: &SmartDatabase,
    cipher: &SecretCipher,
    user_id: u64,
    code: &str,
) -> Result<(), VerifyError> {
    let record: TotpRecord = match db
        .fetch_one::<TotpRecord, _>(&GetTotpQueryView::new(user_id))
        .await
    {
        Ok(record) if record.confirmed => record,
        Ok(_) | Err(ApiLibError::Database(DbError::NotFound)) => {
            return Err(VerifyError::NotEnabled)
        }
        Err(e) => return Err(internal("read the TOTP record", &e)),
    };
    if second_factor::looks_like_backup_code(code) {
        verify_backup_code(db, user_id, code).await
    } else {
        verify_totp(db, cipher, user_id, &record, code).await
    }
}

/// Checks a TOTP `code` against the stored `record` and spends its time step.
///
/// # Errors
///
/// Same as [`verify_code`].
pub async fn verify_totp(
    db: &SmartDatabase,
    cipher: &SecretCipher,
    user_id: u64,
    record: &TotpRecord,
    code: &str,
) -> Result<(), VerifyError> {
    let sealed = general_purpose::STANDARD
        .decode(&record.secret)
        .map_err(|e| internal("decode the sealed secret", &e))?;
    let secret = cipher
        .open(&sealed)
        .map_err(|e| internal("unseal the secret", &e))?;
    let last = record
        .last_used_step
        .and_then(|step| u64::try_from(step).ok());
    let step = second_factor::verify_code(&secret, code, second_factor::now_unix(), last)
        .ok_or(VerifyError::InvalidCode)?;
    // The step is spent atomically: a replay of the same code, even a simultaneous one, loses.
    match db
        .fetch_one::<Written, _>(&UseTotpStepQueryView::new(user_id, step))
        .await
    {
        Ok(_) => Ok(()),
        Err(ApiLibError::Database(DbError::NotFound)) => Err(VerifyError::InvalidCode),
        Err(e) => Err(internal("spend the time step", &e)),
    }
}

async fn verify_backup_code(
    db: &SmartDatabase,
    user_id: u64,
    code: &str,
) -> Result<(), VerifyError> {
    let normalized = second_factor::normalize_backup_code(code);
    let codes: Vec<BackupCodeRecord> = db
        .fetch_all(&ListBackupCodesQueryView::new(user_id))
        .await
        .map_err(|e| internal("list the backup codes", &e))?;
    for stored in codes {
        let matches = passwords::verify(&normalized, &stored.code_hash)
            .await
            .unwrap_or(false);
        if matches {
            return match db
                .fetch_one::<Written, _>(&SpendBackupCodeQueryView::new(user_id, stored.id))
                .await
            {
                Ok(_) => Ok(()),
                Err(ApiLibError::Database(DbError::NotFound)) => Err(VerifyError::InvalidCode),
                Err(e) => Err(internal("spend the backup code", &e)),
            };
        }
    }
    Err(VerifyError::InvalidCode)
}

/// Replaces the backup codes of `user_id` with new ones inside `tx` and returns them in clear:
/// the only time they are visible.
///
/// # Errors
///
/// [`VerifyError::Internal`] when hashing or the database fails; `tx` must then be dropped.
pub async fn issue_backup_codes(
    tx: &mut SmartTransaction,
    user_id: u64,
) -> Result<Vec<String>, VerifyError> {
    tx.execute(&DeleteBackupCodesQueryView::new(user_id))
        .await
        .map_err(|e| internal("delete the backup codes", &e))?;
    let codes = second_factor::generate_backup_codes();
    for code in &codes {
        // Hashed in the canonical form the login normalizes to.
        let hash = passwords::hash(&second_factor::normalize_backup_code(code))
            .await
            .map_err(|e| internal("hash a backup code", &e))?;
        tx.execute(&InsertBackupCodeQueryView::new(user_id, &hash))
            .await
            .map_err(|e| internal("store a backup code", &e))?;
    }
    Ok(codes)
}

/// What the login token stands for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingLogin {
    pub user_id: u64,
    pub device_info: String,
    pub failures: u32,
}

fn login_key(token: &str) -> String {
    format!("2fa_login/{token}")
}

/// Opens a login token for `user_id` (password already verified).
///
/// # Errors
///
/// A message for the logs when Redis refuses the write.
pub async fn begin_login(redis: &Redis, user_id: u64, device_info: &str) -> Result<String, String> {
    let token = refresh_token::generate();
    let pending = PendingLogin {
        user_id,
        device_info: device_info.to_string(),
        failures: 0,
    };
    let value = serde_json::to_string(&pending).map_err(|e| e.to_string())?;
    redis
        .secure_set_ex(&login_key(&token), value, LOGIN_TOKEN_TTL_SECONDS)
        .await
        .map_err(|e| e.to_string())?;
    Ok(token)
}

/// The login `token` stands for, `None` when unknown, expired, burnt or spent.
pub async fn read_login(redis: &Redis, token: &str) -> Option<PendingLogin> {
    let raw = redis.secure_get::<String>(&login_key(token)).await.ok()??;
    serde_json::from_str(&raw).ok()
}

/// Counts a wrong code; burns the token at [`MAX_LOGIN_FAILURES`]. Returns whether it is burnt.
pub async fn record_login_failure(redis: &Redis, token: &str, mut pending: PendingLogin) -> bool {
    pending.failures += 1;
    if pending.failures >= MAX_LOGIN_FAILURES {
        let _ = redis.secure_delete(&login_key(token)).await;
        return true;
    }
    if let Ok(value) = serde_json::to_string(&pending) {
        // `set_ex` overwrites: `secure_set_ex` only writes an absent key (`SET … NX`).
        let _ = redis
            .set_ex(&login_key(token), value, LOGIN_TOKEN_TTL_SECONDS)
            .await;
    }
    false
}

/// Spends the login token. Returns `false` when it was already spent (two requests racing).
pub async fn consume_login(redis: &Redis, token: &str) -> bool {
    let key = login_key(token);
    let present = redis
        .secure_get::<String>(&key)
        .await
        .ok()
        .flatten()
        .is_some();
    if present {
        let _ = redis.secure_delete(&key).await;
    }
    present
}
