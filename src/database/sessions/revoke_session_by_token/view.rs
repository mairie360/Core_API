use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct RevokeSessionByTokenQueryView {
    user_id: u64,
    token_hash: String,
    revoked_at: chrono::DateTime<chrono::Utc>,
    params: Vec<QueryParam>,
}

impl RevokeSessionByTokenQueryView {
    pub fn new(user_id: u64, token_hash: &str) -> Self {
        let revoked_at = chrono::Utc::now();
        Self {
            user_id,
            token_hash: token_hash.to_string(),
            revoked_at,
            params: vec![
                QueryParam::DateTime(revoked_at),
                QueryParam::I64(user_id as i64),
                QueryParam::Text(token_hash.to_string()),
            ],
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }

    pub fn get_token_hash(&self) -> &str {
        self.token_hash.as_str()
    }

    pub fn get_revoked_at(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.revoked_at
    }
}

impl ApiRequestDto for RevokeSessionByTokenQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE sessions SET revoked_at = $1 WHERE user_id = $2 AND token_hash = $3"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}
impl Display for RevokeSessionByTokenQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeSessionQueryView: user_id = {}, token_hash = [PROTECTED]",
            self.user_id,
        )
    }
}
