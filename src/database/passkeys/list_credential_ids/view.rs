use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// The credential ids a user already registered (hex), to exclude from a new registration so
/// the authenticator refuses to create a duplicate.
#[derive(Debug, serde::Deserialize)]
pub struct ListCredentialIdsQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl ListCredentialIdsQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for ListCredentialIdsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT encode(credential_id, 'hex') AS credential_id \
            FROM user_passkeys WHERE user_id = $1 ORDER BY id\
         ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ListCredentialIdsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListCredentialIdsQueryView: user_id = {}", self.user_id)
    }
}

/// One row of [`ListCredentialIdsQueryView`].
#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialIdRow {
    /// Hex-encoded credential id.
    pub credential_id: String,
}

impl CredentialIdRow {
    /// The credential id as bytes.
    ///
    /// # Errors
    ///
    /// When the stored value is not hex (cannot happen for a row written by Core).
    pub fn bytes(&self) -> Result<Vec<u8>, hex::FromHexError> {
        hex::decode(&self.credential_id)
    }
}
