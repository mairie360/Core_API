use crate::endpoints::v1::user::me::get::endpoint::__path_get_me;
use crate::endpoints::v1::user::me::patch::endpoint::__path_patch_me;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(get_me, patch_me),
    components(schemas(super::get::view::GetMeResponseView, super::patch::view::PatchMeView))
)]
pub struct MeDoc;
