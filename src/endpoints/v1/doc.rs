use super::auth::doc::AuthDoc;
use super::sessions::doc::SessionsDoc;
use super::user::doc::UserDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/auth", api = AuthDoc),
    (path = "/sessions", api = SessionsDoc),
    (path = "/user", api = UserDoc),
))]
pub struct V1Doc;
