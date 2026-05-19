use crate::endpoints::v1::auth::login::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::login),
    components(schemas(super::view::LoginView, super::view::LoginResponseView))
)]
pub struct LoginDoc;
