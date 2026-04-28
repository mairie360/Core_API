use crate::endpoints::v1::admin::sessions::audit::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::audit),
    components(schemas(super::response_view::AuditResponseView),)
)]
pub struct AuditDoc;
