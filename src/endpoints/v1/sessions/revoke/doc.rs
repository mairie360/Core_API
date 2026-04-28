use crate::endpoints::v1::sessions::revoke::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::revoke))]
pub struct RevokeDoc;
