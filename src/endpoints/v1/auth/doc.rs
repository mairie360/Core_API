use crate::endpoints::v1::auth::login::doc::LoginDoc;
use crate::endpoints::v1::auth::register::doc::RegisterDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/register", api = RegisterDoc),
    (path = "/login", api = LoginDoc),
))]
pub struct AuthDoc;
