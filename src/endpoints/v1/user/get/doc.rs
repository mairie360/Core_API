use crate::database::users::list_directory::DirectoryUser;
use crate::endpoints::v1::user::get::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::list_directory_users),
    components(schemas(super::view::DirectoryUsersResultView, DirectoryUser))
)]
pub struct DirectoryDoc;
