use crate::endpoints::v1::admin::users::id::patch::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::admin_patch_user),
    components(schemas(super::view::PatchUserView))
)]
pub struct PatchUserDoc;
