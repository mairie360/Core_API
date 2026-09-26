use crate::endpoints::v1::admin::keycloak::migration::doc::MigrationDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/migration", api = MigrationDoc),
))]
pub struct KeycloakDoc;
