use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PutView {
    name: Option<String>,
    description: Option<String>,
    can_be_deleted: Option<bool>,
}
