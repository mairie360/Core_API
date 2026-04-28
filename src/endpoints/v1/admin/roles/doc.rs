use utoipa::OpenApi;

use crate::endpoints::v1::admin::roles::delete::endpoint::__path_delete;
use crate::endpoints::v1::admin::roles::get::endpoint::__path_get;
use crate::endpoints::v1::admin::roles::patch::endpoint::__path_patch;
use crate::endpoints::v1::admin::roles::post::endpoint::__path_post;
use crate::endpoints::v1::admin::roles::put::endpoint::__path_put;

#[derive(OpenApi)]
#[openapi(
    paths(delete, get, patch, post, put),
    components(schemas(
        super::view::RoleWriteView,
        super::get::view::GetResponseView,
        super::patch::view::PatchView
    ))
)]
pub struct RolesDoc;
