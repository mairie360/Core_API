use super::delete::endpoint::__path_delete_group;
use super::get::endpoint::__path_get_group;
use super::users::doc::GroupsUsersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = Doc, tags = ["Groups"]),
    (path = "/users", api = GroupsUsersDoc, tags = ["Groups"]),
))]
pub struct GroupsIdDoc;

#[derive(OpenApi)]
#[openapi(
    paths(get_group, delete_group),
    components(schemas(super::get::view::GetGroupResultView,))
)]
struct Doc;
