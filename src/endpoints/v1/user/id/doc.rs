use crate::endpoints::v1::user::id::get::endpoint::__path_get_user;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(get_user), components())]
pub struct IdDoc;
