use crate::endpoints::v1::admin::users::id::get::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::get),
    components(schemas(super::view::GetUserResultView))
)]
pub struct GetUserDoc;
