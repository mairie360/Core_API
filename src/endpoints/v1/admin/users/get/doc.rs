use crate::database::admin::list_users::{AdminUserRole, AdminUserRow};
use crate::endpoints::v1::admin::users::get::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::admin_list_users),
    components(schemas(super::view::AdminListUsersResultView, AdminUserRow, AdminUserRole))
)]
pub struct ListUsersDoc;
