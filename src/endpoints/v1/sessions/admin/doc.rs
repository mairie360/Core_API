use crate::endpoints::v1::sessions::admin::audit::doc::AuditDoc;
use crate::endpoints::v1::sessions::admin::refresh::doc::RefreshDoc;
use crate::endpoints::v1::sessions::admin::revoke::doc::RevokeDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/audit", api = AuditDoc),
    (path = "/", api = RefreshDoc),
    (path = "/", api = RevokeDoc),
))]
pub struct AdminDoc;
