use super::delete::endpoint::__path_delete_group;
use super::get::endpoint::__path_get_group;
use super::patch::endpoint::__path_patch_group;
use super::users::doc::GroupsUsersDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = Doc),
    (path = "/users", api = GroupsUsersDoc),
))]
pub struct GroupsIdDoc;

#[derive(OpenApi)]
#[openapi(
    paths(get_group, delete_group, patch_group),
    components(schemas(
        super::get::view::GetGroupResultView,
        super::patch::view::PatchGroupView,
        crate::database::groups::get_group::Group,
    ))
)]
struct Doc;
