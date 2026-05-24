use crate::endpoints::v1::admin::users::post::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::post),
    components(schemas(super::view::CreateUserView))
)]
pub struct CreateUserDoc;
