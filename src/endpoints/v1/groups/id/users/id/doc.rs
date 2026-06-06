use super::delete::endpoint::__path_remove_user_from_group;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(remove_user_from_group), components(schemas()))]
pub struct GroupsUsersIdDoc;
