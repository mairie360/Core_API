use super::get::endpoint::__path_get;
use super::id::doc::GroupsUsersIdDoc;
use super::post::endpoint::__path_post;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(get, post),
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
