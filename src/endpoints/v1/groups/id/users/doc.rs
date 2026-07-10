use super::get::endpoint::__path_get_group_members;
use super::id::doc::GroupsUsersIdDoc;
use super::post::endpoint::__path_add_user_to_group;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(add_user_to_group, get_group_members),
    components(schemas(
        super::get::view::GetGroupUsersResultView,
        super::post::view::PostUserGroupView
    ))
)]
pub struct Doc;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = Doc, tags = ["Groups"]),
    (path = "/{user_id}", api = GroupsUsersIdDoc, tags = ["Groups"]),
))]
pub struct GroupsUsersDoc;
