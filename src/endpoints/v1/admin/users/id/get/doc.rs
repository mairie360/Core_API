use crate::endpoints::v1::admin::users::id::get::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::admin_get_user),
    components(schemas(super::view::GetUserResultView))
)]
pub struct GetUserDoc;
