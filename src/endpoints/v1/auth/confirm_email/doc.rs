use crate::endpoints::v1::auth::confirm_email::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::confirm_email),
    components(schemas(super::view::ConfirmEmailView))
)]
pub struct ConfirmEmailDoc;
