use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

/// Role to grant to the user of the path.
#[derive(Deserialize, ToSchema)]
pub struct AddRoleToUserView {
    /// Id of the role to grant, as returned by `GET /api/v1/admin/roles/`.
    #[schema(example = 2)]
    role_id: u64,
    /// Deprecated, optional: the user is the `userId` of the path. When sent, it must equal the
    /// path's `userId`, otherwise the request answers `400`.
    #[schema(example = 42, nullable = false)]
    #[deprecated(note = "the user is taken from the path")]
    user_id: Option<u64>,
}

impl AddRoleToUserView {
    pub const fn role_id(&self) -> u64 {
        self.role_id
    }

    /// The deprecated body `user_id`, only read to refuse a mismatch with the path.
    #[allow(deprecated)]
    pub const fn body_user_id(&self) -> Option<u64> {
        self.user_id
    }
}

impl Display for AddRoleToUserView {
    #[allow(deprecated)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AddRoleToUserView {{ role_id: {}, user_id: {:?} }}",
            self.role_id, self.user_id
        )
    }
}
