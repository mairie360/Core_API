use crate::endpoints::v1::admin::keycloak::doc::KeycloakDoc;
use crate::endpoints::v1::admin::roles::doc::RolesDoc;
use crate::endpoints::v1::admin::two_factor_policy::doc::TwoFactorPolicyDoc;
use crate::endpoints::v1::admin::users::doc::UsersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/keycloak", api = KeycloakDoc),
    (path = "/roles", api = RolesDoc),
    (path = "/two-factor-policy", api = TwoFactorPolicyDoc),
    (path = "/users", api = UsersDoc),
))]
pub struct AdminDoc;
