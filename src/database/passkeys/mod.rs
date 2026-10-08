//! `user_passkeys` (Database 3.1.0, MAIR-505): the `WebAuthn` credentials of the accounts.
//!
//! Credential ids are `BYTEA` in Postgres and travel as hex (`decode($n, 'hex')` /
//! `encode(credential_id, 'hex')`): the lib binds no binary parameter. The credential itself is
//! the `JSONB` serialisation of `webauthn_rs::prelude::Passkey`, read and written whole.

pub mod delete;
pub mod find_by_credential;
pub mod insert;
pub mod list;
pub mod list_credential_ids;
pub mod update_credential;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A registered passkey as shown to its owner: never the public key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PasskeySummary {
    /// Id of the passkey in `DELETE /api/v1/user/me/passkeys/{id}/`.
    #[schema(example = 12)]
    pub id: i32,
    /// Label chosen by the user at registration.
    #[schema(example = "iPhone de Jean")]
    pub label: String,
    #[schema(value_type = String, format = DateTime, example = "2026-10-08T09:30:00Z")]
    pub created_at: DateTime<Utc>,
    /// Last successful sign-in with this passkey, `null` when never used.
    #[schema(
        value_type = Option<String>,
        format = DateTime,
        example = "2026-10-08T14:02:11Z",
        nullable
    )]
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Hex encoding of a credential id, as bound to `decode($n, 'hex')`.
#[must_use]
pub fn credential_id_hex(credential_id: &[u8]) -> String {
    hex::encode(credential_id)
}
