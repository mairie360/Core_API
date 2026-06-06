use crate::endpoints::v1::admin::users::id::roles::delete::endpoint::__path_admin_delete_user_role;
use crate::endpoints::v1::admin::users::id::roles::post::endpoint::__path_admin_add_role_to_user;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(admin_delete_user_role, admin_add_role_to_user))]
pub struct RolesDoc;
