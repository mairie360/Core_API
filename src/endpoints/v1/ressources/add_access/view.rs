use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, ToSchema)]
pub enum AccessType {
    Delete,
    Error,
    Read,
    Update,
}

impl AccessType {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Error => "error",
        }
    }
}

impl From<&str> for AccessType {
    fn from(s: &str) -> Self {
        match s {
            "read" => Self::Read,
            "update" => Self::Update,
            "delete" => Self::Delete,
            _ => Self::Error,
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
    #[must_use]
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

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }

    #[must_use]
    pub const fn resource_id(&self) -> u64 {
        self.resource_id
    }

    #[must_use]
    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }

    #[must_use]
    pub const fn access_type(&self) -> AccessType {
        self.access_type
    }
}
