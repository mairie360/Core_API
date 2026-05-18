use super::delete::endpoint::__path_delete;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(delete), components(schemas()))]
pub struct GroupsUsersIdDoc;
