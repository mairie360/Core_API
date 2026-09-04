use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct RevokeSessionQueryView {
    user_id: u64,
    id: Uuid,
    token_hash: String,
    revoked_at: chrono::DateTime<chrono::Utc>,
    params: Vec<QueryParam>,
}

impl RevokeSessionQueryView {
    pub fn new(user_id: u64, id: Uuid, token_hash: &str) -> Self {
        let revoked_at = chrono::Utc::now();
        Self {
            user_id,
            id,
            token_hash: token_hash.to_string(),
            revoked_at,
            params: vec![
                QueryParam::DateTime(revoked_at),
                QueryParam::I64(user_id as i64),
                QueryParam::Uuid(id),
                QueryParam::Text(token_hash.to_string()),
            ],
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn get_token_hash(&self) -> &str {
        &self.token_hash
    }

    pub fn get_revoked_at(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.revoked_at
    }
}

impl ApiRequestDto for RevokeSessionQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE sessions
         SET revoked_at = $1
         WHERE user_id = $2
         AND id = $3
         AND token_hash = $4"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RevokeSessionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeSessionQueryView: user_id = {}, id = {}, token_hash = {}",
            self.user_id, self.id, self.token_hash,
        )
    }
}
