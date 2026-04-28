use super::get::doc::GetDoc;
use super::history::doc::HistoryDoc;
use super::refresh::doc::RefreshDoc;
use super::revoke::doc::RevokeDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = GetDoc, tags = ["Sessions"]),
    (path = "/", api = HistoryDoc, tags = ["Sessions"]),
    (path = "/", api = RefreshDoc, tags = ["Sessions"]),
    (path = "/", api = RevokeDoc, tags = ["Sessions"]),
))]
pub struct SessionsDoc;
