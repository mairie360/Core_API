use crate::endpoints::v1::auth::passkey::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::passkey_login_options, endpoint::passkey_login),
    components(schemas(
        super::view::PasskeyLoginOptionsResponseView,
        super::view::PasskeyLoginView,
        crate::endpoints::v1::auth::login::view::LoginResponseView
    ))
)]
pub struct PasskeyLoginDoc;
