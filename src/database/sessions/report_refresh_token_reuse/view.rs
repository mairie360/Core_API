use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

/// Answer of `report_refresh_token_reuse()` for a refresh token that matches no current token.
// The flags mirror the JSON answer of the database function, one per outcome it reports.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RefreshTokenReuse {
    /// The token is one a rotation replaced.
    pub known: bool,
    /// Replaced within the grace window while its session is still active: a concurrent renewal by
    /// another front, nothing was written.
    pub superseded: bool,
    /// Replaced longer ago: the reuse was logged (`REFRESH_REUSE`).
    pub reused: bool,
    /// The session was active and is now revoked.
    pub revoked: bool,
    pub session_id: Option<Uuid>,
    pub user_id: Option<i32>,
    /// The alert queued by this report, `None` when one already existed for the session.
    pub alert_id: Option<i64>,
}

/// `report_refresh_token_reuse(hash, grace_seconds, ip, device)` of Database, in one transaction.
///
/// Looks the hash up among every token the rotations of a session replaced. Within
/// `grace_seconds` of its replacement nothing is written; later the reuse is logged with the
/// address and the device, an active session is revoked and one `certain` alert is queued (the
/// agent and the administrators are e-mailed). Both ends of the window are read on the Postgres
/// clock, the one that wrote the replacement time.
#[derive(serde::Deserialize)]
pub struct ReportRefreshTokenReuseQueryView {
    params: Vec<QueryParam>,
}

impl ReportRefreshTokenReuseQueryView {
    #[must_use]
    pub fn new(
        token_hash: &str,
        grace_seconds: i32,
        ip_address: std::net::IpAddr,
        device_info: &str,
    ) -> Self {
        Self {
            params: vec![
                QueryParam::Text(token_hash.to_string()),
                QueryParam::I32(grace_seconds),
                QueryParam::IpAddr(ip_address),
                QueryParam::Text(device_info.to_string()),
            ],
        }
    }
}

impl ApiRequestDto for ReportRefreshTokenReuseQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT report_refresh_token_reuse($1, $2, $3, NULLIF($4, ''))"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ReportRefreshTokenReuseQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReportRefreshTokenReuseQueryView: token_hash = [PROTECTED]"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mair_776_the_query_view_never_displays_the_token_hash() {
        let view = ReportRefreshTokenReuseQueryView::new(
            "token-hash-value",
            10,
            std::net::IpAddr::from([127, 0, 0, 1]),
            "Firefox",
        );
        let shown = view.to_string();
        assert!(shown.contains("[PROTECTED]"));
        assert!(!shown.contains("token-hash-value"));
        assert_eq!(view.query_params().len(), 4);
    }

    #[test]
    fn mair_776_the_report_answer_reads_every_outcome() {
        let superseded: RefreshTokenReuse = serde_json::from_value(serde_json::json!({
            "known": true, "superseded": true, "reused": false, "revoked": false,
            "session_id": "00000000-0000-4000-8000-000000000001", "user_id": 4, "alert_id": null
        }))
        .unwrap();
        assert!(superseded.superseded && !superseded.reused);
        let unknown: RefreshTokenReuse = serde_json::from_value(serde_json::json!({
            "known": false, "superseded": false, "reused": false, "revoked": false,
            "session_id": null, "user_id": null, "alert_id": null
        }))
        .unwrap();
        assert!(!unknown.known && unknown.session_id.is_none());
    }
}
