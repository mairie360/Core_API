use utoipa::OpenApi;

use crate::endpoints::v1::roles::get::endpoint::__path_get_roles;

#[derive(OpenApi)]
#[openapi(
    paths(get_roles),
    components(schemas(super::get::view::GetResponseView,))
)]
pub struct RolesDoc;
