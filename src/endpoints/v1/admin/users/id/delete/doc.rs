use crate::endpoints::v1::admin::users::id::delete::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::delete))]
pub struct DeleteUserDoc;
