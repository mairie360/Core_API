// The transitive dependencies (through mairie360_api_lib and the rest of the tree) bring several
// versions of some crates (base64, bitflags, ...); this repo does not control those versions,
// so this clippy::cargo lint is disabled rather than left failing for good.
#![allow(clippy::multiple_crate_versions)]
pub mod client_ip;
pub mod database;
pub mod email_change;
pub mod emails;
pub mod endpoints;
pub mod invitations;
pub mod login_alert;
pub mod passkey_managers;
pub mod passkeys;
pub mod passwords;
pub mod phone;
pub mod rate_limit;
pub mod redis_keys;
pub mod refresh_token;
pub mod request_log;
pub mod second_factor;
pub mod session_jwt;
pub mod session_revocation;
pub mod two_factor;
