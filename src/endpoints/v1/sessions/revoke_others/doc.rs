use crate::endpoints::v1::sessions::revoke_others::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::revoke_other_sessions),
    components(schemas(endpoint::RevokeOthersResponse))
)]
pub struct RevokeOthersDoc;
