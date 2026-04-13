use crate::endpoints::v1::admin::sessions::doc::SessionsDoc;
use crate::endpoints::v1::admin::users::doc::UsersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/sessions", api = SessionsDoc, tags = ["Admin"]),
    (path = "/users/{userId}", api = UsersDoc, tags = ["Admin"]),
))]
pub struct AdminDoc;
