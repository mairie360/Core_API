use crate::endpoints::v1::auth::force_change_password::doc::ForceChangePasswordDoc;
use crate::endpoints::v1::auth::forgot_password::doc::ForgotPasswordDoc;
use crate::endpoints::v1::auth::login::doc::LoginDoc;
use crate::endpoints::v1::auth::reset_password::doc::ResetPasswordDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/force_change_password", api = ForceChangePasswordDoc),
    (path = "/forgot_password", api = ForgotPasswordDoc),
    (path = "/login", api = LoginDoc),
    (path = "/reset_password", api = ResetPasswordDoc),
))]
pub struct AuthDoc;
