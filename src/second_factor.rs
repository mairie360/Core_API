//! Second authentication factor managed by Core (MAIR-805): TOTP (RFC 6238) and backup codes.
//!
//! The TOTP secret never reaches the database in clear: [`SecretCipher`] seals it with AES-256-GCM
//! under `TOTP_ENCRYPTION_KEY` (32 random bytes, base64), a key that lives outside the database, so
//! a dump of `user_totp` is useless on its own. A code is accepted within one 30-second step of the
//! clock and only if its step is later than the last one accepted for the account
//! (`user_totp.last_used_step`): a code cannot be replayed inside its window.
//!
//! Backup codes are random, single use, shown once and stored as Argon2 hashes only.

// aes-gcm 0.10 still builds its nonces with generic-array 0.14.
#![allow(deprecated)]

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose, Engine as _};
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
use sha1::Sha1;
use subtle::ConstantTimeEq;

/// Length of a TOTP period in seconds (RFC 6238 default, understood by every application).
pub const STEP_SECONDS: u64 = 30;
/// Number of digits of a TOTP code.
pub const DIGITS: u32 = 6;
/// Number of backup codes issued at confirmation and on regeneration.
pub const BACKUP_CODE_COUNT: usize = 10;
/// Name shown by the authenticator application next to the account.
pub const ISSUER: &str = "Mairie 360";

const SECRET_BYTES: usize = 20;
const NONCE_BYTES: usize = 12;
/// No look-alike characters (`0/o`, `1/l/i`) in a code the agent may have to copy by hand.
const BACKUP_ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

/// Why the second factor could not be set up or used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondFactorError {
    /// `TOTP_ENCRYPTION_KEY` is not 32 bytes of base64.
    InvalidKey,
    /// The sealed secret is malformed or was sealed with another key.
    Unseal,
}

impl std::fmt::Display for SecondFactorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKey => write!(f, "TOTP_ENCRYPTION_KEY must be 32 bytes encoded in base64"),
            Self::Unseal => write!(f, "The TOTP secret cannot be unsealed"),
        }
    }
}

impl std::error::Error for SecondFactorError {}

/// Seals and opens TOTP secrets (AES-256-GCM, random 96-bit nonce prepended to the ciphertext).
#[derive(Clone)]
pub struct SecretCipher {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for SecretCipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretCipher(<key>)")
    }
}

impl SecretCipher {
    /// # Errors
    ///
    /// [`SecondFactorError::InvalidKey`] unless `key` is the base64 of 32 bytes.
    pub fn from_base64(key: &str) -> Result<Self, SecondFactorError> {
        let bytes = general_purpose::STANDARD
            .decode(key.trim())
            .map_err(|_| SecondFactorError::InvalidKey)?;
        let cipher =
            Aes256Gcm::new_from_slice(&bytes).map_err(|_| SecondFactorError::InvalidKey)?;
        Ok(Self { cipher })
    }

    /// Reads `TOTP_ENCRYPTION_KEY`; `None` when it is not set (the second factor is then off).
    ///
    /// # Errors
    ///
    /// [`SecondFactorError::InvalidKey`] when it is set but is not 32 bytes of base64.
    pub fn from_env() -> Result<Option<Self>, SecondFactorError> {
        match std::env::var("TOTP_ENCRYPTION_KEY") {
            Ok(key) if !key.trim().is_empty() => Self::from_base64(&key).map(Some),
            _ => Ok(None),
        }
    }

    /// Seals `secret`: `nonce || ciphertext || tag`.
    ///
    /// # Panics
    ///
    /// Never in practice: AES-GCM only fails on an input larger than 2^36 bytes.
    #[must_use]
    pub fn seal(&self, secret: &[u8]) -> Vec<u8> {
        let mut nonce = [0u8; NONCE_BYTES];
        rand::fill(&mut nonce);
        let mut sealed = nonce.to_vec();
        sealed.extend(
            self.cipher
                .encrypt(Nonce::from_slice(&nonce), secret)
                .expect("AES-GCM sealing of a short secret cannot fail"),
        );
        sealed
    }

    /// # Errors
    ///
    /// [`SecondFactorError::Unseal`] when `sealed` is malformed, altered or sealed with another key.
    pub fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, SecondFactorError> {
        if sealed.len() <= NONCE_BYTES {
            return Err(SecondFactorError::Unseal);
        }
        let (nonce, ciphertext) = sealed.split_at(NONCE_BYTES);
        self.cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| SecondFactorError::Unseal)
    }
}

/// A new random TOTP secret (160 bits, the size RFC 4226 recommends).
#[must_use]
pub fn generate_secret() -> Vec<u8> {
    let mut secret = vec![0u8; SECRET_BYTES];
    rand::fill(secret.as_mut_slice());
    secret
}

/// Base32 of `secret`, what the agent types when the QR code cannot be scanned.
#[must_use]
pub fn secret_base32(secret: &[u8]) -> String {
    BASE32_NOPAD.encode(secret)
}

/// `otpauth://` URI of the enrolment (the front turns it into a QR code).
#[must_use]
pub fn otpauth_uri(secret: &[u8], account: &str) -> String {
    let full_label = format!("{ISSUER}:{account}");
    let label =
        percent_encoding::utf8_percent_encode(&full_label, percent_encoding::NON_ALPHANUMERIC);
    let issuer = percent_encoding::utf8_percent_encode(ISSUER, percent_encoding::NON_ALPHANUMERIC);
    format!(
        "otpauth://totp/{label}?secret={}&issuer={issuer}&algorithm=SHA1&digits={DIGITS}&period={STEP_SECONDS}",
        secret_base32(secret)
    )
}

/// Time step of the Unix time `unix_seconds`.
#[must_use]
pub const fn step_at(unix_seconds: u64) -> u64 {
    unix_seconds / STEP_SECONDS
}

/// Current Unix time in seconds.
#[must_use]
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// HOTP value (RFC 4226) of `counter`, as a zero-padded decimal string.
///
/// # Panics
///
/// Never: HMAC accepts a key of any length.
#[must_use]
pub fn code_for_step(secret: &[u8], step: u64) -> String {
    let mut mac = <Hmac<Sha1> as HmacKeyInit>::new_from_slice(secret)
        .expect("HMAC accepts a key of any length");
    mac.update(&step.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[digest.len() - 1] & 0x0f);
    let binary = u32::from_be_bytes([
        digest[offset] & 0x7f,
        digest[offset + 1],
        digest[offset + 2],
        digest[offset + 3],
    ]);
    format!(
        "{:0width$}",
        binary % 10u32.pow(DIGITS),
        width = DIGITS as usize
    )
}

/// Checks `code` against the steps around `unix_seconds`, and returns the step that matched.
///
/// The steps `now - 1 ..= now + 1` are tried (clock drift of a phone); any step that is not later
/// than `last_used_step` is refused. The returned step is the new `last_used_step`.
#[must_use]
pub fn verify_code(
    secret: &[u8],
    code: &str,
    unix_seconds: u64,
    last_used_step: Option<u64>,
) -> Option<u64> {
    let code = code.trim();
    if code.len() != DIGITS as usize || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let now = step_at(unix_seconds);
    // Every candidate is compared, whatever the first match, so the time does not tell which step
    // was right.
    let mut matched = None;
    for step in now.saturating_sub(1)..=now.saturating_add(1) {
        let expected = code_for_step(secret, step);
        let same: bool = expected.as_bytes().ct_eq(code.as_bytes()).into();
        if same && last_used_step.is_none_or(|last| step > last) {
            matched = Some(step);
        }
    }
    matched
}

/// One new backup code: two groups of four characters, e.g. `k3m9-x7pq` (about 40 bits).
#[must_use]
pub fn generate_backup_code() -> String {
    let mut raw = [0u8; 8];
    rand::fill(&mut raw);
    let chars: Vec<char> = raw
        .iter()
        .map(|b| char::from(BACKUP_ALPHABET[usize::from(*b) % BACKUP_ALPHABET.len()]))
        .collect();
    format!(
        "{}-{}",
        chars[..4].iter().collect::<String>(),
        chars[4..].iter().collect::<String>()
    )
}

/// The [`BACKUP_CODE_COUNT`] codes issued at confirmation.
#[must_use]
pub fn generate_backup_codes() -> Vec<String> {
    (0..BACKUP_CODE_COUNT)
        .map(|_| generate_backup_code())
        .collect()
}

/// Canonical form of a typed backup code: lowercase, without dash nor space.
#[must_use]
pub fn normalize_backup_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether `value` has the shape of a typed backup code (8 characters once normalized).
#[must_use]
pub fn looks_like_backup_code(value: &str) -> bool {
    let normalized = normalize_backup_code(value);
    normalized.len() == 8 && normalized.bytes().all(|b| BACKUP_ALPHABET.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 appendix B, SHA-1 secret "12345678901234567890", 8 digits there, 6 here (last six).
    const RFC_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn rfc6238_vectors() {
        for (time, expected) in [
            (59u64, "287082"),
            (1_111_111_109, "081804"),
            (1_234_567_890, "005924"),
        ] {
            assert_eq!(
                code_for_step(RFC_SECRET, step_at(time)),
                expected,
                "t={time}"
            );
        }
    }

    #[test]
    fn a_code_is_accepted_once_per_step() {
        let now = 1_700_000_000;
        // A random secret gives the next step the same code once in a million draws, which the drift
        // window then accepts as a later step: draw one whose neighbouring codes differ.
        let secret = std::iter::repeat_with(generate_secret)
            .find(|secret| {
                let code = code_for_step(secret, step_at(now));
                code != code_for_step(secret, step_at(now) + 1)
                    && code != code_for_step(secret, step_at(now) - 1)
            })
            .expect("a secret");
        let code = code_for_step(&secret, step_at(now));
        let step = verify_code(&secret, &code, now, None).expect("fresh code accepted");
        assert_eq!(step, step_at(now));
        // Same code again, same step: replay refused.
        assert_eq!(verify_code(&secret, &code, now, Some(step)), None);
        // And a code of an earlier step too.
        let earlier = code_for_step(&secret, step - 1);
        assert_eq!(verify_code(&secret, &earlier, now, Some(step)), None);
    }

    #[test]
    fn one_step_of_clock_drift_is_tolerated_two_are_not() {
        let secret = generate_secret();
        let now = 1_700_000_000;
        let next = code_for_step(&secret, step_at(now) + 1);
        assert!(verify_code(&secret, &next, now, None).is_some());
        let far = code_for_step(&secret, step_at(now) + 2);
        assert_eq!(verify_code(&secret, &far, now, None), None);
    }

    #[test]
    fn malformed_codes_are_refused() {
        let secret = generate_secret();
        for code in ["", "12345", "1234567", "12345a", "١٢٣٤٥٦"] {
            assert_eq!(
                verify_code(&secret, code, 1_700_000_000, None),
                None,
                "{code:?}"
            );
        }
    }

    #[test]
    fn sealed_secret_round_trips_and_is_bound_to_the_key() {
        let key = general_purpose::STANDARD.encode([7u8; 32]);
        let other = general_purpose::STANDARD.encode([8u8; 32]);
        let cipher = SecretCipher::from_base64(&key).unwrap();
        let secret = generate_secret();
        let sealed = cipher.seal(&secret);
        assert_ne!(&sealed[NONCE_BYTES..], secret.as_slice());
        assert_ne!(sealed, cipher.seal(&secret), "a fresh nonce per seal");
        assert_eq!(cipher.open(&sealed).unwrap(), secret);
        assert_eq!(
            SecretCipher::from_base64(&other).unwrap().open(&sealed),
            Err(SecondFactorError::Unseal)
        );
        let mut altered = sealed;
        let last = altered.len() - 1;
        altered[last] ^= 1;
        assert_eq!(cipher.open(&altered), Err(SecondFactorError::Unseal));
        assert_eq!(cipher.open(&[1, 2, 3]), Err(SecondFactorError::Unseal));
    }

    #[test]
    fn a_bad_key_is_refused() {
        for key in [
            "",
            "not base64!!",
            &general_purpose::STANDARD.encode([1u8; 16]),
        ] {
            assert_eq!(
                SecretCipher::from_base64(key).unwrap_err(),
                SecondFactorError::InvalidKey
            );
        }
    }

    #[test]
    fn backup_codes_are_distinct_typable_and_normalized() {
        let codes = generate_backup_codes();
        assert_eq!(codes.len(), BACKUP_CODE_COUNT);
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), BACKUP_CODE_COUNT);
        for code in &codes {
            assert!(looks_like_backup_code(code), "{code}");
            assert_eq!(
                normalize_backup_code(&code.to_uppercase()),
                normalize_backup_code(code)
            );
        }
        assert!(!looks_like_backup_code("123456"));
        assert_eq!(normalize_backup_code(" K3M9 - X7PQ "), "k3m9x7pq");
    }

    #[test]
    fn the_uri_carries_the_secret_and_the_parameters() {
        let uri = otpauth_uri(RFC_SECRET, "jean.dupont@mairie360.fr");
        assert!(uri.starts_with("otpauth://totp/Mairie%20360%3Ajean%2Edupont%40mairie360%2Efr?"));
        assert!(uri.contains(&format!("secret={}", secret_base32(RFC_SECRET))));
        assert!(
            uri.contains("period=30") && uri.contains("digits=6") && uri.contains("algorithm=SHA1")
        );
    }
}
