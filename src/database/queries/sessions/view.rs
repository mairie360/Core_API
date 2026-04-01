use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, sqlx::FromRow)]
pub struct Session {
    id: Uuid,
    user_id: i32,
    device_info: String,
    ip_address: std::net::IpAddr,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn new(
        id: Uuid,
        user_id: i32,
        device_info: String,
        ip_address: std::net::IpAddr,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        revoked_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            device_info,
            ip_address,
            created_at,
            expires_at,
            revoked_at,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn user_id(&self) -> i32 {
        self.user_id
    }

    pub fn device_info(&self) -> &str {
        &self.device_info
    }

    pub fn ip_address(&self) -> &std::net::IpAddr {
        &self.ip_address
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn expires_at(&self) -> &DateTime<Utc> {
        &self.expires_at
    }

    pub fn revoked_at(&self) -> Option<&DateTime<Utc>> {
        self.revoked_at.as_ref()
    }
}
