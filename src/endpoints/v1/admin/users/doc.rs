use crate::endpoints::v1::admin::users::id::doc::IdDoc;
use crate::endpoints::v1::admin::users::post::doc::CreateUserDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/{userId}", api = IdDoc),
    (path = "/", api = CreateUserDoc)
))]
pub struct UsersDoc;
