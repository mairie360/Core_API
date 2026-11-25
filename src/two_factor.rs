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
    ListSpentBackupCodesQueryView, SpendBackupCodeQueryView, TotpRecord, UseTotpStepQueryView,
    Written,
};
use crate::database::two_factor_policy::{GetTwoFactorPolicyQueryView, TwoFactorPolicyRecord};
use crate::database::users::delete_user::IsUserActiveQueryView;
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
    /// Wrong, expired or replayed code. A spent backup code is also reported as this variant by
    /// [`verify_code`] (the account routes do not tell it apart), and as [`VerifyError::SpentBackupCode`]
    /// only by [`verify_code_under_policy`] (the sign-in).
    InvalidCode,
    /// A backup code of the account that was already spent, confirmed against its spent row. Only
    /// [`verify_code_under_policy`] looks for it: a handler that matches on it must call that function.
    SpentBackupCode,
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
/// A spent backup code is refused as [`VerifyError::InvalidCode`], like any code that matches nothing:
/// the account routes (disable, regenerate) have no use for the distinction and never run the extra
/// lookup. The sign-in goes through [`verify_code_under_policy`], which reports
/// [`VerifyError::SpentBackupCode`].
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
    verify_code_telling_spent(db, cipher, user_id, code, false).await
}

async fn verify_code_telling_spent(
    db: &SmartDatabase,
    cipher: &SecretCipher,
    user_id: u64,
    code: &str,
    tell_spent: bool,
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
        verify_backup_code(db, user_id, code, tell_spent).await
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
        .fetch_one::<Written, _>(&UseTotpStepQueryView::new(user_id, step, &record.secret))
        .await
    {
        Ok(_) => Ok(()),
        Err(ApiLibError::Database(DbError::NotFound)) => Err(VerifyError::InvalidCode),
        Err(e) => Err(internal("spend the time step", &e)),
    }
}

/// Whether `normalized` is one of the backup codes of `user_id` that were already spent.
///
/// A hash match is not enough: a regeneration may delete the row while the (slow) hash comparison runs, so
/// the matched row is looked up again and must still be there, spent.
async fn is_spent_backup_code(
    db: &SmartDatabase,
    user_id: u64,
    normalized: &str,
) -> Result<bool, VerifyError> {
    let spent: Vec<BackupCodeRecord> = db
        .fetch_all(&ListSpentBackupCodesQueryView::new(user_id))
        .await
        .map_err(|e| internal("list the spent backup codes", &e))?;
    for stored in spent {
        if passwords::verify(normalized, &stored.code_hash)
            .await
            .unwrap_or(false)
        {
            let still_there: Vec<BackupCodeRecord> = db
                .fetch_all(&ListSpentBackupCodesQueryView::new(user_id))
                .await
                .map_err(|e| internal("confirm the spent backup code", &e))?;
            return Ok(still_there.iter().any(|row| row.id == stored.id));
        }
    }
    Ok(false)
}

/// With `tell_spent`, a code that matches a spent row is [`VerifyError::SpentBackupCode`]; without it,
/// every refusal is [`VerifyError::InvalidCode`] and no spent row is read.
async fn verify_backup_code(
    db: &SmartDatabase,
    user_id: u64,
    code: &str,
    tell_spent: bool,
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
                // No row: another request spent it first, or the regeneration deleted it. Only a
                // matching spent row proves the first; the second is a plain invalid code.
                Err(ApiLibError::Database(DbError::NotFound)) => {
                    refuse_unspent_match(db, user_id, &normalized, tell_spent).await
                }
                Err(e) => Err(internal("spend the backup code", &e)),
            };
        }
    }
    // No unspent code matches: a spent one tells the agent to pick another code.
    refuse_unspent_match(db, user_id, &normalized, tell_spent).await
}

async fn refuse_unspent_match(
    db: &SmartDatabase,
    user_id: u64,
    normalized: &str,
    tell_spent: bool,
) -> Result<(), VerifyError> {
    // The code is already rejected here: the lookup only refines the message. If it fails, the attempt
    // must still be refused (and counted by the caller) as a plain invalid code, not turned into a 500.
    if tell_spent {
        match is_spent_backup_code(db, user_id, normalized).await {
            Ok(true) => return Err(VerifyError::SpentBackupCode),
            Ok(false) => {}
            Err(_) => tracing::warn!(
                "Second factor: spent backup code lookup failed, refusing as invalid"
            ),
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
    // The account row is locked first: two requests that each spent a valid code serialize here,
    // so the second replaces the first's codes instead of both sets surviving.
    let _active: bool = tx
        .fetch_scalar(&IsUserActiveQueryView::locked(user_id))
        .await
        .map_err(|e| internal("lock the account", &e))?;
    tx.execute(&DeleteBackupCodesQueryView::new(user_id))
        .await
        .map_err(|e| internal("delete the backup codes", &e))?;
    let codes = second_factor::generate_backup_codes();
    // Hashed in the canonical form the login normalizes to, all at once on the blocking pool:
    // one after the other, the argon2 hashes added up to about half a second per request.
    let normalized: Vec<String> = codes
        .iter()
        .map(|code| second_factor::normalize_backup_code(code))
        .collect();
    let hashes =
        futures_util::future::try_join_all(normalized.iter().map(|code| passwords::hash(code)))
            .await
            .map_err(|e| internal("hash a backup code", &e))?;
    for hash in &hashes {
        tx.execute(&InsertBackupCodeQueryView::new(user_id, hash))
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
    /// Unix second the token stops being valid. Stored next to the counter because rewriting the
    /// counter must not give the token a new lifetime: Redis only knows the TTL of the write.
    pub expires_at: u64,
}

/// Which intermediate token: the second step of a login, or the mandatory enrolment of an agent
/// without a second factor (MAIR-808).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingKind {
    Login,
    Setup,
}

impl PendingKind {
    fn key(self, token: &str) -> String {
        match self {
            Self::Login => format!("2fa_login/{token}"),
            Self::Setup => format!("2fa_setup/{token}"),
        }
    }

    /// Marker written once, atomically, by the request that spends the token.
    fn spent_key(self, token: &str) -> String {
        match self {
            Self::Login => format!("2fa_spent/{token}"),
            Self::Setup => format!("2fa_setup_spent/{token}"),
        }
    }
}

async fn begin(
    redis: &Redis,
    kind: PendingKind,
    user_id: u64,
    device_info: &str,
) -> Result<String, String> {
    let token = refresh_token::generate();
    let pending = PendingLogin {
        user_id,
        device_info: device_info.to_string(),
        failures: 0,
        expires_at: second_factor::now_unix() + LOGIN_TOKEN_TTL_SECONDS,
    };
    let value = serde_json::to_string(&pending).map_err(|e| e.to_string())?;
    redis
        .set_ex(&kind.key(&token), value, LOGIN_TOKEN_TTL_SECONDS)
        .await
        .map_err(|e| e.to_string())?;
    Ok(token)
}

async fn read(redis: &Redis, kind: PendingKind, token: &str) -> Option<PendingLogin> {
    let raw = redis.secure_get::<String>(&kind.key(token)).await.ok()??;
    let pending: PendingLogin = serde_json::from_str(&raw).ok()?;
    (second_factor::now_unix() < pending.expires_at).then_some(pending)
}

/// Counts a wrong code. The token keeps its original expiry: the write only lasts for what is
/// left of it.
async fn record_failure(
    redis: &Redis,
    kind: PendingKind,
    token: &str,
    mut pending: PendingLogin,
) -> bool {
    pending.failures += 1;
    let remaining = pending.expires_at.saturating_sub(second_factor::now_unix());
    if pending.failures >= MAX_LOGIN_FAILURES || remaining == 0 {
        let _ = redis.secure_delete(&kind.key(token)).await;
        return true;
    }
    if let Ok(value) = serde_json::to_string(&pending) {
        // `set_ex` overwrites: `secure_set_ex` only writes an absent key (`SET … NX`).
        let _ = redis.set_ex(&kind.key(token), value, remaining).await;
    }
    false
}

/// Spends the token: `Ok(true)` for the one request that wins, decided by a single `SET … NX`.
async fn consume(redis: &Redis, kind: PendingKind, token: &str) -> Result<bool, String> {
    let won = redis
        .secure_set_ex(&kind.spent_key(token), "1", LOGIN_TOKEN_TTL_SECONDS)
        .await
        .map_err(|e| e.to_string())?;
    if won {
        let _ = redis.secure_delete(&kind.key(token)).await;
    }
    Ok(won)
}

/// Opens a login token for `user_id` (password already verified).
///
/// # Errors
///
/// A message for the logs when Redis refuses the write.
pub async fn begin_login(redis: &Redis, user_id: u64, device_info: &str) -> Result<String, String> {
    begin(redis, PendingKind::Login, user_id, device_info).await
}

/// The login `token` stands for, `None` when unknown, expired, burnt or spent.
pub async fn read_login(redis: &Redis, token: &str) -> Option<PendingLogin> {
    read(redis, PendingKind::Login, token).await
}

/// Counts a wrong code; burns the token at [`MAX_LOGIN_FAILURES`]. Returns whether it is burnt.
pub async fn record_login_failure(redis: &Redis, token: &str, pending: PendingLogin) -> bool {
    record_failure(redis, PendingKind::Login, token, pending).await
}

/// Spends the login token: `Ok(true)` for the one request that wins.
///
/// `Ok(false)` when another request already spent it, so two requests holding valid codes cannot
/// both get a session.
///
/// # Errors
///
/// A message for the logs when Redis refuses the write: nothing was spent, no session may follow.
pub async fn consume_login(redis: &Redis, token: &str) -> Result<bool, String> {
    consume(redis, PendingKind::Login, token).await
}

/// Opens the enrolment token of an agent who must set up a second factor (MAIR-808): the password
/// is verified, no session exists. It only works on `POST /auth/2fa/setup` and `confirm`.
///
/// # Errors
///
/// A message for the logs when Redis refuses the write.
pub async fn begin_setup(redis: &Redis, user_id: u64, device_info: &str) -> Result<String, String> {
    begin(redis, PendingKind::Setup, user_id, device_info).await
}

/// The enrolment `token` stands for, `None` when unknown, expired, burnt or spent.
pub async fn read_setup(redis: &Redis, token: &str) -> Option<PendingLogin> {
    read(redis, PendingKind::Setup, token).await
}

/// Counts a wrong code of the enrolment; burns the token at [`MAX_LOGIN_FAILURES`].
pub async fn record_setup_failure(redis: &Redis, token: &str, pending: PendingLogin) -> bool {
    record_failure(redis, PendingKind::Setup, token, pending).await
}

/// Spends the enrolment token: `Ok(false)` when it was already spent.
///
/// # Errors
///
/// A message for the logs when Redis refuses the write: nothing was spent.
pub async fn consume_setup(redis: &Redis, token: &str) -> Result<bool, String> {
    consume(redis, PendingKind::Setup, token).await
}

/// The policy of the instance (MAIR-808).
///
/// # Errors
///
/// [`ApiLibError`] when the database fails.
pub async fn load_policy(db: &SmartDatabase) -> Result<TwoFactorPolicyRecord, ApiLibError> {
    db.fetch_one::<TwoFactorPolicyRecord, _>(&GetTwoFactorPolicyQueryView::new())
        .await
}

/// [`verify_code`] under the policy, for the sign-in: a mode the administrator removed is not accepted, and
/// a backup code that was already spent is reported as [`VerifyError::SpentBackupCode`] so the agent is
/// told to pick another one.
///
/// # Errors
///
/// [`VerifyError::NotEnabled`] when the user's only mode is not allowed, [`VerifyError::InvalidCode`]
/// for a backup code while backup codes are not allowed, otherwise as [`verify_code`], plus
/// [`VerifyError::SpentBackupCode`].
pub async fn verify_code_under_policy(
    db: &SmartDatabase,
    cipher: &SecretCipher,
    policy: &TwoFactorPolicyRecord,
    user_id: u64,
    code: &str,
) -> Result<(), VerifyError> {
    if !policy.totp_allowed {
        return Err(VerifyError::NotEnabled);
    }
    if !policy.backup_codes_allowed && second_factor::looks_like_backup_code(code) {
        return Err(VerifyError::InvalidCode);
    }
    verify_code_telling_spent(db, cipher, user_id, code, true).await
}
