use crate::endpoints::v1::admin::users::id::patch::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::patch),
    components(schemas(super::view::PatchUserView))
)]
pub struct PatchUserDoc;
