use crate::endpoints::v1::user::about::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::about),
    components(schemas(super::about_response_view::AboutResponseView),)
)]
pub struct AboutDoc;
