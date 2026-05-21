use crate::endpoints::v1::user::id::get::endpoint::__path_get;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(get), components())]
pub struct IdDoc;
