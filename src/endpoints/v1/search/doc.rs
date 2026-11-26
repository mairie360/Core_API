use crate::database::search::view::{SearchItem, SearchKind};
use crate::endpoints::v1::search::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::global_search),
    components(schemas(super::view::SearchResultView, SearchItem, SearchKind))
)]
pub struct SearchDoc;
