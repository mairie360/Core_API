use crate::endpoints::v1::admin::sessions::audit::doc::AuditDoc;
use crate::endpoints::v1::admin::sessions::refresh::doc::RefreshDoc;
use crate::endpoints::v1::admin::sessions::revoke::doc::RevokeDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = AuditDoc),
    (path = "/", api = RefreshDoc),
    (path = "/", api = RevokeDoc),
))]
pub struct SessionsDoc;
