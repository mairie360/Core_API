use crate::endpoints::v1::admin::sessions::revoke::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::revoke))]
pub struct RevokeDoc;
