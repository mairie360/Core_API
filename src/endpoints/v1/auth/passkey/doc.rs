use crate::endpoints::v1::auth::passkey::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::passkey_login_options, endpoint::passkey_login),
    components(schemas(super::view::PasskeyLoginOptionsView, super::view::PasskeyLoginView))
)]
pub struct PasskeyLoginDoc;
