use super::get::doc::GetDoc;
use super::history::doc::HistoryDoc;
use super::logout::doc::LogoutDoc;
use super::refresh::doc::RefreshDoc;
use super::revoke::doc::RevokeDoc;
use super::revoke_by_id::doc::RevokeByIdDoc;
use super::revoke_others::doc::RevokeOthersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = GetDoc),
    (path = "/", api = HistoryDoc),
    (path = "/", api = LogoutDoc),
    (path = "/", api = RefreshDoc),
    (path = "/", api = RevokeDoc),
    (path = "/", api = RevokeByIdDoc),
    (path = "/", api = RevokeOthersDoc),
))]
pub struct SessionsDoc;
