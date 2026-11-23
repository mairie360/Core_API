use crate::endpoints::v1::admin::users::id::service::endpoint;
use crate::endpoints::v1::admin::users::id::service::view::UserServiceView;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::admin_set_user_service),
    components(schemas(UserServiceView))
)]
pub struct UserServiceDoc;
