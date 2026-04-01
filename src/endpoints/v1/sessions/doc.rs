use super::get::doc::GetDoc;
use super::history::doc::HistoryDoc;
use super::refresh::doc::RefreshDoc;
use super::revoke::doc::RevokeDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = GetDoc),
    (path = "/history", api = HistoryDoc),
    (path = "/", api = RefreshDoc),
    (path = "/", api = RevokeDoc),
))]
pub struct SessionsDoc;
