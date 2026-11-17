use crate::endpoints::v1::auth::two_factor_setup::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::setup_login_two_factor, endpoint::confirm_login_two_factor),
    components(schemas(
        super::view::SetupTokenView,
        super::view::SetupConfirmView,
        super::view::SetupCompletedView
    ))
)]
pub struct TwoFactorSetupDoc;
