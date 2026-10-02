use utoipa::ToSchema;

/// User to add to the group of the path.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostUserGroupView {
    /// Id of the user to add.
    #[schema(example = 42)]
    user_id: u64,
    /// Deprecated, optional: the group is the `group_id` of the path. When sent, it must equal
    /// the path's `group_id`, otherwise the request answers `400`.
    #[schema(example = 3, nullable = false)]
    #[deprecated(note = "the group is taken from the path")]
    group_id: Option<u64>,
}

impl PostUserGroupView {
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }

    /// The deprecated body `group_id`, only read to refuse a mismatch with the path.
    #[allow(deprecated)]
    pub const fn body_group_id(&self) -> Option<u64> {
        self.group_id
    }
}
