use crate::endpoints::v1::auth::logout::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::logout), components(schemas(super::view::LogoutView)))]
pub struct LogoutDoc;
