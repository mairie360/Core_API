use crate::endpoints::v1::auth::login::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::login),
    components(schemas(super::login_view::LoginView))
)]
pub struct LoginDoc;
