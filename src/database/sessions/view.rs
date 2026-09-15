use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    #[must_use]
    pub const fn new(
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

    #[must_use]
    pub const fn id(&self) -> &Uuid {
        &self.id
    }

    #[must_use]
    pub const fn user_id(&self) -> i32 {
        self.user_id
    }

    #[must_use]
    pub fn device_info(&self) -> &str {
        &self.device_info
    }

    #[must_use]
    pub const fn ip_address(&self) -> &std::net::IpAddr {
        &self.ip_address
    }

    #[must_use]
    pub const fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    #[must_use]
    pub const fn expires_at(&self) -> &DateTime<Utc> {
        &self.expires_at
    }

    #[must_use]
    pub const fn revoked_at(&self) -> Option<&DateTime<Utc>> {
        self.revoked_at.as_ref()
    }
}
