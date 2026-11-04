use crate::endpoints::v1::admin::keycloak::console_access::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::request_keycloak_console_access),
    components(schemas(super::view::KeycloakConsoleAccessView))
)]
pub struct ConsoleAccessDoc;
