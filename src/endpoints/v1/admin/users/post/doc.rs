use crate::endpoints::v1::admin::users::post::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::admin_post_user),
    components(schemas(super::view::CreateUserView))
)]
pub struct CreateUserDoc;
