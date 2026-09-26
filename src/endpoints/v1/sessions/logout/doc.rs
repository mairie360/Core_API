use crate::endpoints::v1::sessions::logout::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::logout))]
pub struct LogoutDoc;
