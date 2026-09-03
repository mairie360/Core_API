use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetActiveSessionsQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl GetActiveSessionsQueryView {
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I64(user_id as i64)],
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for GetActiveSessionsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT * FROM v_sessions WHERE user_id = $1 AND is_active = true) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }

    // Reprend la clé du cache Redis manuel que l'endpoint gérait lui-même auparavant, désormais
    // pris en charge par le cache-aside intégré de `SmartDatabase`.
    fn cache_key(&self) -> Option<String> {
        Some(format!("sessions:{}", self.user_id))
    }
}

impl Display for GetActiveSessionsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetActiveSessionsQueryView: user_id = {}", self.user_id,)
    }
}
