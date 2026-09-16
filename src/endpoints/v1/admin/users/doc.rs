use crate::endpoints::v1::admin::users::get::doc::ListUsersDoc;
use crate::endpoints::v1::admin::users::id::doc::IdDoc;
use crate::endpoints::v1::admin::users::post::doc::CreateUserDoc;
use utoipa::OpenApi;

// utoipa remplace (au lieu de fusionner) les opérations de deux `nest` qui aboutissent au même
// chemin : les opérations de `/` sont donc réunies dans un seul document avant d'être imbriquées.
#[derive(OpenApi)]
#[openapi(nest(
    (path = "/{userId}", api = IdDoc),
    (path = "/", api = RootDoc),
))]
pub struct UsersDoc;

struct RootDoc;

impl OpenApi for RootDoc {
    fn openapi() -> utoipa::openapi::OpenApi {
        let mut doc = CreateUserDoc::openapi();
        doc.merge(ListUsersDoc::openapi());
        doc
    }
}
