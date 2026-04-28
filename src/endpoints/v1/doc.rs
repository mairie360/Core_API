// use super::admin::doc::AdminDoc;
use super::auth::doc::AuthDoc;
// use super::roles::doc::RolesDoc;
use super::sessions::doc::SessionsDoc;
use super::user::doc::UserDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    // (path = "/admin", api = AdminDoc),
    (path = "/auth", api = AuthDoc),
    // (path = "/roles", api = RolesDoc),
    (path = "/sessions", api = SessionsDoc),
    (path = "/user", api = UserDoc),
))]
pub struct V1Doc;
