use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct Role {
    id: u64,
    name: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetResponseView {
    roles: Vec<Role>,
}
