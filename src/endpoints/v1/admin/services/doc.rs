use utoipa::OpenApi;

use crate::endpoints::v1::admin::services::endpoint::{
    __path_admin_create_service, __path_admin_delete_service, __path_admin_list_services,
    __path_admin_update_service,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        admin_list_services,
        admin_create_service,
        admin_update_service,
        admin_delete_service
    ),
    components(schemas(
        super::view::ServiceWriteView,
        super::view::ServiceView,
        super::view::ServicesView,
    ))
)]
pub struct ServicesDoc;
