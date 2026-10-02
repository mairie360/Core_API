use crate::endpoints::v1::admin::keycloak::doc::KeycloakDoc;
use crate::endpoints::v1::admin::roles::doc::RolesDoc;
use crate::endpoints::v1::admin::users::doc::UsersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/keycloak", api = KeycloakDoc),
    (path = "/roles", api = RolesDoc),
    (path = "/users", api = UsersDoc),
))]
pub struct AdminDoc;
