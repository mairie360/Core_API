use super::get::endpoint::__path_get;
use super::id::doc::GroupsIdDoc;
use super::post::endpoint::__path_post;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = Doc, tags = ["Groups"]),
    (path = "/{group_id}", api = GroupsIdDoc, tags = ["Groups"]),

))]
pub struct GroupsDoc;

#[derive(OpenApi)]
#[openapi(
    paths(get, post),
    components(schemas(
        super::get::view::GetGroupsResultView,
        super::post::view::PostGroupView,
    ))
)]
struct Doc;
