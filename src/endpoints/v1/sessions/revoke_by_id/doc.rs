use crate::endpoints::v1::sessions::revoke_by_id::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::revoke_session_by_id))]
pub struct RevokeByIdDoc;
