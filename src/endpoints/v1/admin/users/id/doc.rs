use crate::endpoints::v1::admin::users::id::delete::doc::DeleteUserDoc;
use crate::endpoints::v1::admin::users::id::get::doc::GetUserDoc;
use crate::endpoints::v1::admin::users::id::patch::doc::PatchUserDoc;
use crate::endpoints::v1::admin::users::id::roles::doc::RolesDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/roles", api = RolesDoc),
    (path = "/", api = DeleteUserDoc),
    (path = "/", api = GetUserDoc),
    (path = "/", api = PatchUserDoc),
))]
pub struct IdDoc;
