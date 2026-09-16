use crate::endpoints::v1::admin::users::id::password::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::admin_reset_user_password),
    components(schemas(super::view::AdminResetPasswordView))
)]
pub struct ResetPasswordDoc;
