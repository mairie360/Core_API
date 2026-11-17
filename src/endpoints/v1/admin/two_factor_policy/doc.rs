use crate::endpoints::v1::admin::two_factor_policy::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::get_two_factor_policy, endpoint::update_two_factor_policy),
    components(schemas(
        super::view::TwoFactorPolicyView,
        super::view::UpdateTwoFactorPolicyView
    ))
)]
pub struct TwoFactorPolicyDoc;
