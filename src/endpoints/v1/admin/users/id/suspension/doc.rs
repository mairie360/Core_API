use crate::endpoints::v1::admin::users::id::suspension::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::admin_suspend_user, endpoint::admin_reactivate_user))]
pub struct SuspensionDoc;
