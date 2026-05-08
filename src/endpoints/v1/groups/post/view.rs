use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostGroupView {
    name: String,
    description: String,
}

impl PostGroupView {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}
