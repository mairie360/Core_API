use crate::endpoints::v1::sessions::history::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::history),
    components(schemas(super::response_view::HistoryResponseView),)
)]
pub struct HistoryDoc;
