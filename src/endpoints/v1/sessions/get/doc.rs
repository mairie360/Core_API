use crate::endpoints::v1::sessions::get::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::get),
    components(schemas(super::response_view::GetResponseView),)
)]
pub struct GetDoc;
