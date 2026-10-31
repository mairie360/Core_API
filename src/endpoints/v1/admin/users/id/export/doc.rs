use crate::endpoints::v1::admin::users::id::export::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(endpoint::admin_export_user_data))]
pub struct ExportUserDataDoc;
