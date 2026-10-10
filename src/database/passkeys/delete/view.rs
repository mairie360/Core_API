use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Deletes one passkey of a user and returns the number of rows deleted.
///
/// Read with `fetch_scalar::<i32>`: `0` when the passkey does not exist or belongs to someone
/// else, which the handler answers `404` either way.
#[derive(Debug, serde::Deserialize)]
pub struct DeletePasskeyQueryView {
    passkey_id: u64,
    user_id: u64,
    params: Vec<QueryParam>,
}

impl DeletePasskeyQueryView {
    #[must_use]
    pub fn new(passkey_id: u64, user_id: u64) -> Self {
        Self {
            passkey_id,
            user_id,
            params: vec![
                QueryParam::I32(id_to_sql(passkey_id)),
                QueryParam::I32(id_to_sql(user_id)),
            ],
        }
    }

    #[must_use]
    pub const fn passkey_id(&self) -> u64 {
        self.passkey_id
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for DeletePasskeyQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH deleted AS (\
            DELETE FROM user_passkeys WHERE id = $1 AND user_id = $2 RETURNING id\
         ) SELECT count(*)::int FROM deleted"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DeletePasskeyQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeletePasskeyQueryView: passkey_id = {}, user_id = {}",
            self.passkey_id, self.user_id
        )
    }
}
