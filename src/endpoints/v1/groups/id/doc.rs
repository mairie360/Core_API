use super::delete::endpoint::__path_delete;
use super::get::endpoint::__path_get;
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
    paths(get, delete),
    components(schemas(super::get::view::GetGroupResultView,))
)]
struct Doc;
