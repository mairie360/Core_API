use utoipa::OpenApi;

use crate::endpoints::v1::ressources::add_access::endpoint as add_access_endpoint;
use crate::endpoints::v1::ressources::remove_access::endpoint as remove_access_endpoint;

#[derive(OpenApi)]
#[openapi(
    paths(add_access_endpoint::add_access, remove_access_endpoint::remove_access),
    components(schemas(
        super::add_access::view::AddAccessView,
        super::remove_access::view::RemoveAccessView
    ))
)]
pub struct RessourcesDoc;
