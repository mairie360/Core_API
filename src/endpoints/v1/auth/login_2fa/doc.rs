use crate::endpoints::v1::auth::login_2fa::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::login_two_factor),
    components(schemas(super::view::LoginTwoFactorView))
)]
pub struct LoginTwoFactorDoc;
