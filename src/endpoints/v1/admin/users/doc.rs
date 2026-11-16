use crate::endpoints::v1::admin::users::get::doc::ListUsersDoc;
use crate::endpoints::v1::admin::users::id::doc::IdDoc;
use crate::endpoints::v1::admin::users::post::doc::CreateUserDoc;
use utoipa::OpenApi;

// utoipa replaces (instead of merging) the operations of two `nest` that lead to the same
// path: the operations of `/` are therefore gathered in one document before being nested.
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
