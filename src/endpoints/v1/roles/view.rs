use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleWriteView {
    name: String,
    description: Option<String>,
    can_be_deleted: Option<bool>,
}
