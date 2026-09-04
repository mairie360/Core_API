use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct RevokeSessionByIdQueryView {
    user_id: u64,
    id: Uuid,
    revoked_at: chrono::DateTime<chrono::Utc>,
    params: Vec<QueryParam>,
}

impl RevokeSessionByIdQueryView {
    pub fn new(user_id: u64, id: Uuid) -> Self {
        let revoked_at = chrono::Utc::now();
        Self {
            user_id,
            id,
            revoked_at,
            params: vec![
                QueryParam::DateTime(revoked_at),
                QueryParam::I64(user_id as i64),
                QueryParam::Uuid(id),
            ],
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

impl ApiRequestDto for RevokeSessionByIdQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE sessions
         SET revoked_at = $1
         WHERE user_id = $2
         AND id = $3"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
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
