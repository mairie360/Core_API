use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct RemoveAccessView {
    access_id: u64,
}

impl RemoveAccessView {
    pub fn new(access_id: u64) -> Self {
        Self { access_id }
    }

    pub fn access_id(&self) -> u64 {
        self.access_id
    }
}
