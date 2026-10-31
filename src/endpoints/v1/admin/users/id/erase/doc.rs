use crate::endpoints::v1::admin::users::id::erase::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::admin_erase_user),
    components(schemas(super::view::ErasureView))
)]
pub struct EraseUserDoc;
