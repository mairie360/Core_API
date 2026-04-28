use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RevokeSessionQueryView {
    user_id: u64,
    id: Uuid,
    token_hash: String,
    revoked_at: chrono::DateTime<chrono::Utc>,
}

impl RevokeSessionQueryView {
    pub fn new(user_id: u64, id: Uuid, token_hash: &str) -> Self {
        Self {
            user_id,
            id,
            token_hash: token_hash.to_string(),
            revoked_at: chrono::Utc::now(),
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

impl DatabaseQueryView for RevokeSessionQueryView {
    fn get_request(&self) -> String {
        "UPDATE sessions
         SET revoked_at = $1
         WHERE user_id = $2
         AND id = $3
         AND token_hash = $4"
            .to_string()
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
