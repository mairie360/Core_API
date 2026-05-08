use crate::endpoints::v1::groups::get::view::Group;
use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupResultView {
    group: Group,
}
