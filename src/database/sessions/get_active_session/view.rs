use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetActiveSessionQueryView {
    user_id: u64,
    ip_address: std::net::IpAddr,
    device_info: String,
    params: Vec<QueryParam>,
}

impl GetActiveSessionQueryView {
    pub fn new(user_id: u64, ip_address: std::net::IpAddr, device_info: &str) -> Self {
        Self {
            user_id,
            ip_address,
            device_info: device_info.to_string(),
            params: vec![
                QueryParam::I64(user_id as i64),
                QueryParam::IpAddr(ip_address),
                QueryParam::Text(device_info.to_string()),
            ],
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }

    pub fn get_ip(&self) -> &std::net::IpAddr {
        &self.ip_address
    }

    pub fn get_device_info(&self) -> &str {
        &self.device_info
    }
}

impl ApiRequestDto for GetActiveSessionQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT * FROM v_sessions WHERE user_id = $1 AND ip_address = $2 AND device_info = $3) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetActiveSessionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetActiveSessionQueryView: user_id = {}, ip = {}, device_info = {}",
            self.user_id, self.ip_address, self.device_info,
        )
    }
}
