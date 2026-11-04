use crate::endpoints::v1::admin::keycloak::console_access::doc::ConsoleAccessDoc;
use crate::endpoints::v1::admin::keycloak::migration::doc::MigrationDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/migration", api = MigrationDoc),
    (path = "/console-access", api = ConsoleAccessDoc),
))]
pub struct KeycloakDoc;
