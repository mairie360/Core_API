use crate::endpoints::v1::auth::keycloak::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::keycloak_login),
    components(schemas(
        super::view::KeycloakLoginView,
        crate::endpoints::v1::auth::login::view::LoginResponseView
    ))
)]
pub struct KeycloakLoginDoc;
