use crate::endpoints::v1::admin::roles::doc::RolesDoc;
use crate::endpoints::v1::admin::sessions::doc::SessionsDoc;
use crate::endpoints::v1::admin::users::doc::UsersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/roles", api = RolesDoc, tags = ["Admin - Roles"]),
    (path = "/sessions", api = SessionsDoc, tags = ["Admin - Sessions"]),
    (path = "/users/{userId}", api = UsersDoc, tags = ["Admin - Users"]),
))]
pub struct AdminDoc;
