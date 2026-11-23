use crate::endpoints::v1::admin::users::id::sessions::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        endpoint::admin_revoke_user_sessions,
        endpoint::admin_revoke_user_session,
        endpoint::admin_get_user_connection_logs
    ),
    components(schemas(crate::database::security_alerts::ConnectionLog))
)]
pub struct UserSessionsDoc;
