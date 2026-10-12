//! Passkeys (`WebAuthn` / FIDO2): Core is the relying party (MAIR-505).
//!
//! A passkey ceremony takes two requests: the first one hands the browser a challenge (and
//! keeps the matching state), the second one brings back the authenticator's answer, checked
//! against that state. The state waits in Redis between the two ([`ChallengeStore`]), keyed by a
//! random challenge id and gone after [`CHALLENGE_TTL_SECONDS`] or its first use, so a challenge
//! is never answered twice and every replica can finish a ceremony another one started.
//!
//! [`WebauthnConfig`] reads the relying party from the environment: it is optional, like
//! Keycloak, and without it the passkey routes answer `503`.

mod config;
mod store;

pub use config::WebauthnConfig;
pub use store::{ChallengeStore, PendingAuthentication, PendingRegistration};

use uuid::Uuid;

/// Time to live of a pending ceremony: the browser dialog is answered within seconds, two
/// minutes leave room for a user looking for their security key.
pub const CHALLENGE_TTL_SECONDS: u64 = 2 * 60;

/// Namespace of the `WebAuthn` user handles (a random UUID chosen once for the platform).
const USER_HANDLE_NAMESPACE: Uuid = Uuid::from_bytes([
    0x4d, 0x61, 0x69, 0x72, 0x69, 0x65, 0x33, 0x36, 0x30, 0x50, 0x61, 0x73, 0x73, 0x6b, 0x65, 0x79,
]);

/// The `WebAuthn` user handle of an account: stable, derived from its id, and never the e-mail
/// (the authenticator stores it and sends it back with every discoverable assertion).
#[must_use]
pub fn user_handle(user_id: u64) -> Uuid {
    Uuid::new_v5(&USER_HANDLE_NAMESPACE, &user_id.to_be_bytes())
}

#[cfg(test)]
mod tests {
    use super::user_handle;

    #[test]
    fn user_handle_is_stable_and_distinct_per_account() {
        assert_eq!(user_handle(42), user_handle(42));
        assert_ne!(user_handle(42), user_handle(43));
    }
}
