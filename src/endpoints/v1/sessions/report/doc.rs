use crate::endpoints::v1::sessions::report::endpoint;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::report_session_use),
    components(schemas(
        endpoint::ReportSessionView,
        endpoint::ReportSessionResponse,
        crate::database::security_alerts::SuspicionLevel
    ))
)]
pub struct ReportDoc;
