use crate::endpoints::v1::admin::security_alerts::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::list_security_alerts),
    components(schemas(
        crate::database::security_alerts::AdminSecurityAlert,
        crate::database::security_alerts::AlertUser
    ))
)]
pub struct SecurityAlertsDoc;
