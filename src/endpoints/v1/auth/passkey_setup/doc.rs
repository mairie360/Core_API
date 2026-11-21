use crate::endpoints::v1::auth::passkey_setup::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::passkey_setup_options, endpoint::passkey_setup),
    components(schemas(super::view::PasskeySetupView, super::view::PasskeySetupCompletedView))
)]
pub struct PasskeySetupDoc;
