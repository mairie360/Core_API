use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub enum AccessType {
    Delete,
    Error,
    Read,
    Write,
}

impl AccessType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessType::Read => "read",
            AccessType::Write => "write",
            AccessType::Delete => "delete",
            AccessType::Error => "error",
        }
    }
}

impl From<&str> for AccessType {
    fn from(s: &str) -> Self {
        match s {
            "read" => AccessType::Read,
            "write" => AccessType::Write,
            "delete" => AccessType::Delete,
            _ => AccessType::Error,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AddAccessView {
    user_id: u64,
    resource_id: u64,
    ressource_type: String,
    access_type: AccessType,
}

impl AddAccessView {
    pub fn new(
        user_id: u64,
        resource_id: u64,
        ressource_type: &str,
        access_type: AccessType,
    ) -> Self {
        Self {
            user_id,
            resource_id,
            ressource_type: ressource_type.to_string(),
            access_type,
        }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn resource_id(&self) -> u64 {
        self.resource_id
    }

    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }

    pub fn access_type(&self) -> AccessType {
        self.access_type
    }
}
