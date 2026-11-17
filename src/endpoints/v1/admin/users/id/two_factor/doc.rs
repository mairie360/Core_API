use crate::endpoints::v1::admin::users::id::two_factor::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::admin_reset_user_two_factor))]
pub struct ResetTwoFactorDoc;
