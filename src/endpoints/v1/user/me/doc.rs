use crate::endpoints::v1::user::me::get::endpoint::__path_get;
use crate::endpoints::v1::user::me::patch::endpoint::__path_patch;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(get, patch),
    components(schemas(super::get::view::GetMeResponseView, super::patch::view::PatchMeView))
)]
pub struct MeDoc;
