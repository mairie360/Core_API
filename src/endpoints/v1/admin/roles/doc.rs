use utoipa::OpenApi;

use crate::endpoints::v1::admin::roles::delete::endpoint::__path_admin_delete_role;
use crate::endpoints::v1::admin::roles::get::endpoint::__path_admin_get_role;
use crate::endpoints::v1::admin::roles::patch::endpoint::__path_admin_patch_role;
use crate::endpoints::v1::admin::roles::post::endpoint::__path_admin_post_role;
use crate::endpoints::v1::admin::roles::put::endpoint::__path_admin_put_role;

#[derive(OpenApi)]
#[openapi(
    paths(
        admin_delete_role,
        admin_get_role,
        admin_patch_role,
        admin_post_role,
        admin_put_role
    ),
    components(schemas(
        super::view::RoleWriteView,
        super::get::view::GetResponseView,
        super::patch::view::PatchView,
    ))
)]
pub struct RolesDoc;
