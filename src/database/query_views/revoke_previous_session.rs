use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct RevokePreviousSessionQueryView {
    user_id: u64,
    ip_address: std::net::IpAddr,
    device_info: String,
}

impl RevokePreviousSessionQueryView {
    pub fn new(user_id: u64, ip_address: std::net::IpAddr, device_info: &str) -> Self {
        Self {
            user_id,
            ip_address,
            device_info: device_info.to_string(),
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

impl DatabaseQueryView for RevokePreviousSessionQueryView {
    fn get_request(&self) -> String {
        "DELETE FROM sessions WHERE user_id = $1 AND ip_address = $2 AND device_info = $3".to_string()
    }
}
impl Display for RevokePreviousSessionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokePreviousSessionQueryView: user_id = {}, ip = {}, device_info = {}",
            self.user_id, self.ip_address, self.device_info,
        )
    }
}
