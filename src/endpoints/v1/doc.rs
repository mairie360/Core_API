use super::auth::doc::AuthDoc;
use super::user::doc::UserDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/auth", api = AuthDoc),
    (path = "/user", api = UserDoc),
))]
pub struct V1Doc;
