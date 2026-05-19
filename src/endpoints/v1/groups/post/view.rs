use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostGroupView {
    name: String,
    description: String,
}

impl PostGroupView {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostGroupResultView {
    id: u64,
}

impl PostGroupResultView {
    pub fn new(id: u64) -> Self {
        Self { id }
    }
}
