use crate::endpoints::v1::auth::jwks::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::jwks),
    components(schemas(super::view::JwksView, super::view::JwkView))
)]
pub struct JwksDoc;
