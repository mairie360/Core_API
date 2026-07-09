use crate::endpoints::v1::auth::force_change_password::doc::ForceChangePasswordDoc;
use crate::endpoints::v1::auth::forgot_password::doc::ForgotPasswordDoc;
use crate::endpoints::v1::auth::login::doc::LoginDoc;
use crate::endpoints::v1::auth::register::doc::RegisterDoc;
use crate::endpoints::v1::auth::reset_password::doc::ResetPasswordDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/force-change-password", api = ForceChangePasswordDoc, tags = ["Auth"]),
    (path = "/forgot-password", api = ForgotPasswordDoc, tags = ["Auth"]),
    (path = "/register", api = RegisterDoc, tags = ["Auth"]),
    (path = "/login", api = LoginDoc, tags = ["Auth"]),
    (path = "/reset-password", api = ResetPasswordDoc, tags = ["Auth"]),
))]
pub struct AuthDoc;
