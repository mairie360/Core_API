use chrono::{DateTime, Utc};
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct CreateSessionQueryView {
    user_id: u64,
    token_hash: String,
    device_info: String,
    ip_address: std::net::IpAddr,
    expires_at: DateTime<Utc>,
}

impl CreateSessionQueryView {
    pub fn new(
        user_id: u64,
        token_hash: String,
        device_info: String,
        ip_address: std::net::IpAddr,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            token_hash,
            device_info,
            ip_address,
            expires_at,
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }

    pub fn get_token_hash(&self) -> &str {
        &self.token_hash
    }

    pub fn get_device_info(&self) -> &str {
        &self.device_info
    }

    pub fn get_ip_address(&self) -> &std::net::IpAddr {
        &self.ip_address
    }

    pub fn get_expires_at(&self) -> &DateTime<Utc> {
        &self.expires_at
    }
}

impl DatabaseQueryView for CreateSessionQueryView {
    fn get_request(&self) -> String {
        "INSERT INTO sessions (user_id, token_hash, device_info, ip_address, expires_at) VALUES ($1, $2, $3, $4, $5)".to_string()
    }
}

impl Display for CreateSessionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CreateSessionQueryView: user_id = {}, token_hash = [PROTECTED], device_info = {}, ip_address = {}, expires_at = {:?}",
            self.user_id,
            self.device_info,
            self.ip_address,
            self.expires_at
        )
    }
}
