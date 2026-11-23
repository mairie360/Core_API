//! Queries of the stolen-session alerts (MAIR-773): `report_suspicious_session_use()` of Database,
//! `session_security_alerts` (alerts, their delivery and their acknowledgement) and `connection_logs`
//! (the sign-in journal an administrator reads).

use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// How sure the front proxy is that the session was used by someone else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuspicionLevel {
    /// The session is bound to a device key and the request came without a valid proof.
    Certain,
    /// An unbound session was used from another network than the one it was opened from.
    Possible,
    /// A bound session changed network with a valid proof: logged, no alert.
    IpChange,
}

impl SuspicionLevel {
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Certain => "certain",
            Self::Possible => "possible",
            Self::IpChange => "ip_change",
        }
    }
}

/// `report_suspicious_session_use(session, user, level, ip, device)`: logs the use, revokes the session
/// for `certain` / `possible` and queues one alert per session and level.
#[derive(serde::Deserialize)]
pub struct ReportSuspiciousSessionQueryView {
    params: Vec<QueryParam>,
}

impl ReportSuspiciousSessionQueryView {
    #[must_use]
    pub fn new(
        session_id: Uuid,
        user_id: u64,
        level: SuspicionLevel,
        ip_address: std::net::IpAddr,
        device_info: &str,
    ) -> Self {
        Self {
            params: vec![
                QueryParam::Uuid(session_id),
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::Text(level.as_sql().to_string()),
                QueryParam::IpAddr(ip_address),
                QueryParam::Text(device_info.to_string()),
            ],
        }
    }
}

impl ApiRequestDto for ReportSuspiciousSessionQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT report_suspicious_session_use($1, $2, $3, $4, NULLIF($5, ''))"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Answer of [`ReportSuspiciousSessionQueryView`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReportOutcome {
    /// The session belongs to that user.
    pub known: bool,
    /// The session was active and is now revoked.
    pub revoked: bool,
    /// The alert queued by this report, `None` when one already existed for this session and level.
    pub alert_id: Option<i64>,
}

/// An alert of a session as the agent or an administrator sees it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SecurityAlert {
    #[schema(example = 12)]
    pub id: i64,
    /// `certain` or `possible`.
    #[schema(example = "certain")]
    pub level: String,
    /// Address the session was used from.
    #[schema(example = "203.0.113.7")]
    pub ip_address: Option<String>,
    /// User agent of that use, when the front sent it.
    #[schema(example = "Mozilla/5.0 (X11; Linux x86_64) Firefox/140.0")]
    pub device_info: Option<String>,
    /// When the use was reported (RFC 3339).
    #[schema(example = "2026-11-23T08:12:00+00:00")]
    pub created_at: String,
}

/// The latest alert of a user that the user has not acknowledged yet.
#[derive(serde::Deserialize)]
pub struct PendingSecurityAlertQueryView {
    params: Vec<QueryParam>,
}

impl PendingSecurityAlertQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
}

impl ApiRequestDto for PendingSecurityAlertQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT id, level, host(ip_address) AS ip_address, device_info, created_at \
            FROM session_security_alerts \
            WHERE user_id = $1 AND acknowledged_at IS NULL \
            ORDER BY created_at DESC, id DESC LIMIT 1\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Marks an alert of the user as read; no row when it is not theirs, unknown or already read.
#[derive(serde::Deserialize)]
pub struct AcknowledgeSecurityAlertQueryView {
    params: Vec<QueryParam>,
}

impl AcknowledgeSecurityAlertQueryView {
    #[must_use]
    pub fn new(user_id: u64, alert_id: u64) -> Self {
        Self {
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::I64(i64::try_from(alert_id).unwrap_or(i64::MAX)),
            ],
        }
    }
}

impl ApiRequestDto for AcknowledgeSecurityAlertQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH acknowledged AS (\
            UPDATE session_security_alerts SET acknowledged_at = now() \
            WHERE user_id = $1 AND id = $2 AND acknowledged_at IS NULL RETURNING id\
        ) SELECT to_json(id) FROM acknowledged"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// An alert of the last days with its agent, for the administration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AdminSecurityAlert {
    #[schema(example = 12)]
    pub id: i64,
    #[schema(example = "certain")]
    pub level: String,
    #[schema(example = "203.0.113.7")]
    pub ip_address: Option<String>,
    #[schema(example = "Mozilla/5.0 (X11; Linux x86_64) Firefox/140.0")]
    pub device_info: Option<String>,
    #[schema(example = "2026-11-23T08:12:00+00:00")]
    pub created_at: String,
    /// When the agent read the alert at sign-in, `null` until then.
    #[schema(example = json!(null))]
    pub acknowledged_at: Option<String>,
    pub user: AlertUser,
}

/// The agent an alert concerns.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AlertUser {
    #[schema(example = 42)]
    pub id: i64,
    #[schema(example = "Camille")]
    pub first_name: String,
    #[schema(example = "Martin")]
    pub last_name: String,
}

/// Number of days of alerts the administration reads.
pub const ADMIN_ALERT_DAYS: i32 = 30;

/// The alerts of the last [`ADMIN_ALERT_DAYS`] days, newest first (500 at most).
#[derive(serde::Deserialize)]
pub struct AdminSecurityAlertsQueryView {
    params: Vec<QueryParam>,
}

impl AdminSecurityAlertsQueryView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: vec![QueryParam::I32(ADMIN_ALERT_DAYS)],
        }
    }
}

impl Default for AdminSecurityAlertsQueryView {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequestDto for AdminSecurityAlertsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT a.id, a.level, host(a.ip_address) AS ip_address, a.device_info, \
                   a.created_at, a.acknowledged_at, \
                   json_build_object('id', u.id, 'first_name', u.first_name, 'last_name', u.last_name) AS user \
            FROM session_security_alerts a JOIN users u ON u.id = a.user_id \
            WHERE a.created_at > now() - make_interval(days => $1) \
            ORDER BY a.created_at DESC, a.id DESC LIMIT 500\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// One line of the sign-in journal of an agent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ConnectionLog {
    /// `LOGIN`, `LOGOUT`, `REFRESH`, `EXPIRED`, `CLEANUP`, `STOLEN_SESSION_ATTEMPT`,
    /// `SUSPICIOUS_IP_CHANGE` or `IP_CHANGE`.
    #[schema(example = "LOGIN")]
    pub action: String,
    #[schema(example = "4f7c2a9e-8d31-4c55-9a0b-3e1f6d2c8b70")]
    pub session_id: Option<String>,
    #[schema(example = "203.0.113.7")]
    pub ip_address: Option<String>,
    #[schema(example = "Mozilla/5.0 (X11; Linux x86_64) Firefox/140.0")]
    pub device_info: Option<String>,
    #[schema(example = "2026-11-23T08:12:00+00:00")]
    pub occurred_at: Option<String>,
}

/// The latest 100 lines of the sign-in journal of a user, newest first.
#[derive(serde::Deserialize)]
pub struct ConnectionLogsQueryView {
    params: Vec<QueryParam>,
}

impl ConnectionLogsQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
}

impl ApiRequestDto for ConnectionLogsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT action_type::text AS action, session_id::text AS session_id, \
                   host(ip_address) AS ip_address, device_info, \"timestamp\" AS occurred_at \
            FROM connection_logs WHERE user_id = $1 \
            ORDER BY \"timestamp\" DESC NULLS LAST, id LIMIT 100\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// An alert claimed for delivery, with the agent it is sent to.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaimedAlert {
    pub id: i64,
    pub user_id: i64,
    pub level: String,
    pub ip_address: Option<String>,
    pub device_info: Option<String>,
    pub created_at: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}

/// Claims up to 20 unsent alerts (5 attempts at most, one per minute) for this replica: the rows are
/// locked with `SKIP LOCKED` and their attempt counted in the same statement, so two replicas never
/// send the same alert at once.
#[derive(serde::Deserialize)]
pub struct ClaimUnsentAlertsQueryView {
    params: Vec<QueryParam>,
}

impl ClaimUnsentAlertsQueryView {
    #[must_use]
    pub const fn new() -> Self {
        Self { params: Vec::new() }
    }
}

impl Default for ClaimUnsentAlertsQueryView {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequestDto for ClaimUnsentAlertsQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH claimed AS (\
            UPDATE session_security_alerts SET attempts = attempts + 1, last_attempt_at = now() \
            WHERE id IN (\
                SELECT id FROM session_security_alerts \
                WHERE sent_at IS NULL AND attempts < 5 \
                  AND (last_attempt_at IS NULL OR last_attempt_at < now() - interval '1 minute') \
                ORDER BY id LIMIT 20 FOR UPDATE SKIP LOCKED\
            ) RETURNING *\
        ) SELECT row_to_json(t) FROM (\
            SELECT c.id, c.user_id, c.level, host(c.ip_address) AS ip_address, c.device_info, \
                   c.created_at, u.email, u.first_name, u.last_name \
            FROM claimed c JOIN users u ON u.id = c.user_id\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Records the delivery of an alert: sent, or the reason it failed (never a value, 200 characters).
#[derive(serde::Deserialize)]
pub struct RecordAlertDeliveryQueryView {
    params: Vec<QueryParam>,
}

impl RecordAlertDeliveryQueryView {
    #[must_use]
    pub fn sent(alert_id: i64) -> Self {
        Self {
            params: vec![QueryParam::I64(alert_id), QueryParam::Text(String::new())],
        }
    }

    #[must_use]
    pub fn failed(alert_id: i64, reason: &str) -> Self {
        Self {
            params: vec![
                QueryParam::I64(alert_id),
                QueryParam::Text(reason.chars().take(200).collect()),
            ],
        }
    }
}

impl ApiRequestDto for RecordAlertDeliveryQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE session_security_alerts SET \
            sent_at = CASE WHEN $2 = '' THEN now() ELSE sent_at END, \
            last_error = NULLIF($2, '') \
         WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// E-mail address of an active administrator (one JSON string per row).
#[derive(serde::Deserialize)]
pub struct ActiveAdminEmailsQueryView {
    params: Vec<QueryParam>,
}

impl ActiveAdminEmailsQueryView {
    #[must_use]
    pub const fn new() -> Self {
        Self { params: Vec::new() }
    }
}

impl Default for ActiveAdminEmailsQueryView {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequestDto for ActiveAdminEmailsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_json(u.email) FROM users u \
         JOIN user_roles ur ON ur.user_id = u.id JOIN roles r ON r.id = ur.role_id \
         WHERE r.name = 'Admin' AND NOT COALESCE(u.is_archived, false) \
           AND u.status IS DISTINCT FROM 'suspended' \
         ORDER BY u.id"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}
