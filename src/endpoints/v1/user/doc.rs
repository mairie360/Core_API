use crate::endpoints::v1::user::about::doc::AboutDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = AboutDoc, tags = ["Users"]),
))]
pub struct UserDoc;
