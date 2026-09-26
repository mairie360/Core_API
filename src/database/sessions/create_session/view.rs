use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct CreateSessionQueryView {
    id: Option<uuid::Uuid>,
    user_id: u64,
    token_hash: String,
    device_info: String,
    ip_address: std::net::IpAddr,
    params: Vec<QueryParam>,
}

impl CreateSessionQueryView {
    #[must_use]
    pub fn new(
        user_id: u64,
        token_hash: &str,
        device_info: &str,
        ip_address: std::net::IpAddr,
    ) -> Self {
        Self {
            id: None,
            user_id,
            token_hash: token_hash.to_string(),
            device_info: device_info.to_string(),
            ip_address,
            params: vec![
                QueryParam::I64(user_id as i64),
                QueryParam::Text(token_hash.to_string()),
                QueryParam::Text(device_info.to_string()),
                QueryParam::IpAddr(ip_address),
            ],
        }
    }

    /// Same as [`Self::new`], with the session id chosen by the caller instead of the database,
    /// so the JWT can carry it (MAIR-226).
    #[must_use]
    pub fn with_id(
        id: uuid::Uuid,
        user_id: u64,
        token_hash: &str,
        device_info: &str,
        ip_address: std::net::IpAddr,
    ) -> Self {
        let mut view = Self::new(user_id, token_hash, device_info, ip_address);
        view.id = Some(id);
        view.params.push(QueryParam::Uuid(id));
        view
    }

    #[must_use]
    pub const fn get_id(&self) -> Option<uuid::Uuid> {
        self.id
    }

    #[must_use]
    pub const fn get_user_id(&self) -> u64 {
        self.user_id
    }

    #[must_use]
    pub fn get_token_hash(&self) -> &str {
        &self.token_hash
    }

    #[must_use]
    pub fn get_device_info(&self) -> &str {
        &self.device_info
    }

    #[must_use]
    pub const fn get_ip_address(&self) -> &std::net::IpAddr {
        &self.ip_address
    }
}

impl ApiRequestDto for CreateSessionQueryView {
    fn query_sql(&self) -> &'static str {
        if self.id.is_some() {
            "INSERT INTO sessions (user_id, token_hash, device_info, ip_address, id) VALUES ($1, $2, $3, $4, $5)"
        } else {
            "INSERT INTO sessions (user_id, token_hash, device_info, ip_address) VALUES ($1, $2, $3, $4)"
        }
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for CreateSessionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CreateSessionQueryView: user_id = {}, token_hash = [PROTECTED], device_info = {}, ip_address = {}",
            self.user_id,
            self.device_info,
            self.ip_address,
        )
    }
}
