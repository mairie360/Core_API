use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RevokeSessionByIdQueryView {
    user_id: u64,
    id: Uuid,
    revoked_at: chrono::DateTime<chrono::Utc>,
}

impl RevokeSessionByIdQueryView {
    pub fn new(user_id: u64, id: Uuid) -> Self {
        Self {
            user_id,
            id,
            revoked_at: chrono::Utc::now(),
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn get_revoked_at(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.revoked_at
    }
}

impl DatabaseQueryView for RevokeSessionByIdQueryView {
    fn get_request(&self) -> String {
        "UPDATE sessions
         SET revoked_at = $1
         WHERE user_id = $2
         AND id = $3"
            .to_string()
    }
}

impl Display for RevokeSessionByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeSessionByIdQueryView: user_id = {}, id = {}",
            self.user_id, self.id,
        )
    }
}
