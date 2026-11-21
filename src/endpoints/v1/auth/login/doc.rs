use crate::endpoints::v1::auth::login::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::login),
    components(schemas(
        super::view::LoginView,
        super::view::LoginResponseView,
        super::view::PasskeyOfferView,
        super::view::LoginFirstConnectionResponseView,
        super::view::TwoFactorRequiredView,
        super::view::TwoFactorSetupPolicyView
    ))
)]
pub struct LoginDoc;
