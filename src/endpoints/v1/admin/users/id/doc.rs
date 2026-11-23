use crate::endpoints::v1::admin::users::id::delete::doc::DeleteUserDoc;
use crate::endpoints::v1::admin::users::id::erase::doc::EraseUserDoc;
use crate::endpoints::v1::admin::users::id::export::doc::ExportUserDataDoc;
use crate::endpoints::v1::admin::users::id::get::doc::GetUserDoc;
use crate::endpoints::v1::admin::users::id::password::doc::ResetPasswordDoc;
use crate::endpoints::v1::admin::users::id::patch::doc::PatchUserDoc;
use crate::endpoints::v1::admin::users::id::roles::doc::RolesDoc;
use crate::endpoints::v1::admin::users::id::sessions::doc::UserSessionsDoc;
use crate::endpoints::v1::admin::users::id::suspension::doc::SuspensionDoc;
use crate::endpoints::v1::admin::users::id::two_factor::doc::ResetTwoFactorDoc;
use utoipa::OpenApi;

// See `UsersDoc`: GET, PATCH and DELETE share the path `/{userId}/` and must be merged
// before nesting, otherwise only the last nested document is published.
#[derive(OpenApi)]
#[openapi(nest(
    (path = "/roles", api = RolesDoc),
    (path = "/", api = RootDoc),
))]
pub struct IdDoc;

struct RootDoc;

impl OpenApi for RootDoc {
    fn openapi() -> utoipa::openapi::OpenApi {
        let mut doc = DeleteUserDoc::openapi();
        doc.merge(GetUserDoc::openapi());
        doc.merge(PatchUserDoc::openapi());
        doc.merge(ResetPasswordDoc::openapi());
        doc.merge(EraseUserDoc::openapi());
        doc.merge(ExportUserDataDoc::openapi());
        doc.merge(ResetTwoFactorDoc::openapi());
        doc.merge(UserSessionsDoc::openapi());
        doc.merge(SuspensionDoc::openapi());
        doc
    }
}
