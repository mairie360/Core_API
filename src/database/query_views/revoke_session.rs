use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;
use uuid::Uuid;

pub struct RevokeSessionQueryView {
    user_id: u64,
    token_hash: Option<String>,
    id: Option<Uuid>,
}

impl RevokeSessionQueryView {
    pub fn new(user_id: u64, token_hash: Option<String>, id: Option<Uuid>) -> Self {
        Self {
            user_id,
            token_hash,
            id,
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }

    pub fn get_token_hash(&self) -> Option<&str> {
        self.token_hash.as_deref()
    }

    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
}

impl DatabaseQueryView for RevokeSessionQueryView {
    fn get_request(&self) -> String {
        "DELETE FROM sessions WHERE user_id = $1 AND (token_hash = $2 OR id = $3)".to_string()
    }
}
impl Display for RevokeSessionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeSessionQueryView: user_id = {}, token_hash = [PROTECTED], id = {}",
            self.user_id,
            self.id.unwrap_or_default(),
        )
    }
}
