//! Opaque refresh tokens (MAIR-390).
//!
//! The client receives a random 256-bit token; the `sessions.token_hash` column only ever holds
//! its SHA-256 digest, so a read of the table (backup, SQL injection, leaked dump) does not hand
//! out usable sessions. A slow hash is not needed: the token has 256 bits of entropy, it cannot be
//! guessed from its digest.

use base64::{engine::general_purpose, Engine as _};
use sha2::{Digest, Sha256};

/// Generates a new refresh token: 32 random bytes, base64url without padding.
#[must_use]
pub fn generate() -> String {
    let mut buffer = [0u8; 32];
    rand::fill(&mut buffer);
    general_purpose::URL_SAFE_NO_PAD.encode(buffer)
}

/// Value stored in `sessions.token_hash` for `token`: its SHA-256 digest, lowercase hex.
#[must_use]
pub fn hash(token: &str) -> String {
    use std::fmt::Write as _;
    Sha256::digest(token.as_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_url_safe() {
        let (a, b) = (generate(), generate());
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
        assert!(a
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn hash_is_stable_hex_and_differs_from_the_token() {
        let token = generate();
        assert_eq!(hash(&token), hash(&token));
        assert_eq!(hash(&token).len(), 64);
        assert_ne!(hash(&token), token);
        assert_eq!(
            hash("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
