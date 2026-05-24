use crate::endpoints::v1::admin::users::id::roles::doc::RolesDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/roles", api = RolesDoc)
))]
pub struct IdDoc;
