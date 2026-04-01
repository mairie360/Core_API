use crate::endpoints::v1::admin::sessions::doc::SessionsDoc;
use crate::endpoints::v1::admin::users::doc::UsersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/sessions", api = SessionsDoc),
    (path = "/users", api = UsersDoc),
))]
pub struct AdminDoc;
