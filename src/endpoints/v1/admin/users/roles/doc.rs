use crate::endpoints::v1::admin::users::roles::delete::endpoint::__path_delete;
use crate::endpoints::v1::admin::users::roles::post::endpoint::__path_post;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(post, delete))]
pub struct RolesDoc;
