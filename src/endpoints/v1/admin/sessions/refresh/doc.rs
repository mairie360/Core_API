use crate::endpoints::v1::admin::sessions::refresh::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::refresh))]
pub struct RefreshDoc;
