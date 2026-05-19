use crate::endpoints::v1::auth::force_change_password::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::force_change_password),
    components(schemas(super::view::ForceChangePasswordView))
)]
pub struct ForceChangePasswordDoc;
