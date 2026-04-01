use super::auth::doc::AuthDoc;
use super::sessions::doc::SessionsDoc;
use super::users::doc::UsersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/auth", api = AuthDoc),
    (path = "/sessions", api = SessionsDoc),
    (path = "/users", api = UsersDoc),
))]
pub struct V1Doc;
