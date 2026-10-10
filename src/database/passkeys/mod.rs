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

#[cfg(test)]
mod tests {
    use super::credential_id_hex;
    use super::delete::DeletePasskeyQueryView;
    use super::find_by_credential::FindPasskeyByCredentialQueryView;
    use super::insert::InsertPasskeyQueryView;
    use super::list::ListPasskeysQueryView;
    use super::list_credential_ids::{CredentialIdRow, ListCredentialIdsQueryView};
    use super::update_credential::UpdatePasskeyCredentialQueryView;
    use mairie360_api_lib::database::db_interface::ApiRequestDto;

    const CREDENTIAL: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];

    #[test]
    fn credential_ids_travel_as_hex() {
        let hex = credential_id_hex(&CREDENTIAL);
        assert_eq!(hex, "deadbeef");
        let row = CredentialIdRow { credential_id: hex };
        assert_eq!(row.bytes().unwrap(), CREDENTIAL);
        assert!(CredentialIdRow {
            credential_id: "zz".to_string()
        }
        .bytes()
        .is_err());
    }

    #[test]
    fn views_bind_their_parameters_and_log_ids_only() {
        let insert = InsertPasskeyQueryView::new(7, "deadbeef", "{}", "Mon téléphone");
        assert_eq!(insert.user_id(), 7);
        assert_eq!(insert.label(), "Mon téléphone");
        assert_eq!(insert.query_params().len(), 4);
        assert_eq!(insert.to_string(), "InsertPasskeyQueryView: user_id = 7");

        let list = ListPasskeysQueryView::new(7);
        assert_eq!(list.user_id(), 7);
        assert_eq!(list.query_params().len(), 1);
        assert_eq!(list.to_string(), "ListPasskeysQueryView: user_id = 7");

        let ids = ListCredentialIdsQueryView::new(7);
        assert_eq!(ids.user_id(), 7);
        assert_eq!(ids.query_params().len(), 1);
        assert_eq!(ids.to_string(), "ListCredentialIdsQueryView: user_id = 7");

        let find = FindPasskeyByCredentialQueryView::new("deadbeef");
        assert_eq!(find.query_params().len(), 1);
        // The credential id identifies a device: never in the logs.
        assert_eq!(find.to_string(), "FindPasskeyByCredentialQueryView");
        assert!(find.query_sql().contains("decode($1, 'hex')"));

        let update = UpdatePasskeyCredentialQueryView::new(12, "{}");
        assert_eq!(update.passkey_id(), 12);
        assert_eq!(update.query_params().len(), 2);
        assert_eq!(
            update.to_string(),
            "UpdatePasskeyCredentialQueryView: passkey_id = 12"
        );

        let delete = DeletePasskeyQueryView::new(12, 7);
        assert_eq!(delete.passkey_id(), 12);
        assert_eq!(delete.user_id(), 7);
        assert_eq!(delete.query_params().len(), 2);
        assert_eq!(
            delete.to_string(),
            "DeletePasskeyQueryView: passkey_id = 12, user_id = 7"
        );
    }
}
