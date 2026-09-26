use utoipa::ToSchema;

/// ACL entry to remove.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct RemoveAccessView {
    /// Id of the entry, as returned in `accesses[].id` by `POST /api/v1/ressources/{id}/access`.
    #[schema(example = 12)]
    access_id: u64,
}

impl RemoveAccessView {
    #[must_use]
    pub const fn new(access_id: u64) -> Self {
        Self { access_id }
    }

    #[must_use]
    pub const fn access_id(&self) -> u64 {
        self.access_id
    }
}
