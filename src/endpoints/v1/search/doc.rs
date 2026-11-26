use crate::database::search::view::{GlobalSearchItem, GlobalSearchKind};
use crate::endpoints::v1::search::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::global_search),
    components(schemas(
        super::view::GlobalSearchResultView,
        GlobalSearchItem,
        GlobalSearchKind
    ))
)]
pub struct SearchDoc;
