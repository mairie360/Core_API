use crate::endpoints::v1::admin::users::id::delete::doc::DeleteUserDoc;
use crate::endpoints::v1::admin::users::id::get::doc::GetUserDoc;
use crate::endpoints::v1::admin::users::id::password::doc::ResetPasswordDoc;
use crate::endpoints::v1::admin::users::id::patch::doc::PatchUserDoc;
use crate::endpoints::v1::admin::users::id::roles::doc::RolesDoc;
use utoipa::OpenApi;

// Cf. `UsersDoc` : GET, PATCH et DELETE partagent le chemin `/{userId}/` et doivent être fusionnés
// avant l'imbrication, sinon seul le dernier document imbriqué est publié.
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
        doc
    }
}
