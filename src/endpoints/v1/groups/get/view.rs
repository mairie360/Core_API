use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct Group {
    id: u64,
    owner_id: u64,
    name: String,
    description: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupsResultView {
    groups: Vec<Group>,
}
