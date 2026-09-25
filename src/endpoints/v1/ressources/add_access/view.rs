use utoipa::ToSchema;

/// Permission granted on a resource instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, ToSchema)]
pub enum AccessType {
    /// Delete the instance.
    Delete,
    /// Not a real permission: always rejected with `400`.
    Error,
    /// Read the instance.
    Read,
    /// Modify the instance.
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

/// Access to grant on a resource instance.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AddAccessView {
    /// Id of the user who receives the access.
    #[schema(example = 42)]
    user_id: u64,
    /// Id of the resource instance (for example the group id when `ressource_type` is `groups`).
    #[schema(example = 3)]
    resource_id: u64,
    /// Resource type, as named in the `resources` table (`groups`, `events`, `users`, `roles`,
    /// ...). Only `groups` and `events` instances have an owner.
    #[schema(example = "groups")]
    ressource_type: String,
    /// Permission granted.
    #[schema(example = "Read")]
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
