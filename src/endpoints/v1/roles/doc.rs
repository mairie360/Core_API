use utoipa::OpenApi;

use crate::endpoints::v1::roles::get::endpoint::__path_get;

#[derive(OpenApi)]
#[openapi(paths(get), components(schemas(super::get::view::GetResponseView,)))]
pub struct RolesDoc;
