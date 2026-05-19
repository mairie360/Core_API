use crate::endpoints::v1::auth::forgot_password::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::forgot_password),
    components(schemas(super::view::ForgotPasswordView))
)]
pub struct ForgotPasswordDoc;
