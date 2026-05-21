use crate::endpoints::v1::auth::reset_password::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::reset_password),
    components(schemas(super::view::ResetPasswordView))
)]
pub struct ResetPasswordDoc;
