use crate::endpoints::v1::auth::login::doc::LoginDoc;
use crate::endpoints::v1::auth::register::doc::RegisterDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/register", api = RegisterDoc, tags = ["Authentication"]),
    (path = "/login", api = LoginDoc, tags = ["Authentication"]),
))]
pub struct AuthDoc;
