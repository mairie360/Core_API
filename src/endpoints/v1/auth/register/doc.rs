use crate::endpoints::v1::auth::register::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::register),
    components(schemas(super::register_view::RegisterView))
)]
pub struct RegisterDoc;
