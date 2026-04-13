use crate::endpoints::v1::admin::users::roles::doc::RolesDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/roles", api = RolesDoc)
))]
pub struct UsersDoc;
