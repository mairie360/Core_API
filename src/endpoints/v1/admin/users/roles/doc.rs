use crate::endpoints::v1::admin::users::roles::assign::endpoint::__path_assign;
use crate::endpoints::v1::admin::users::roles::remove::endpoint::__path_remove;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(assign, remove))]
pub struct RolesDoc;
