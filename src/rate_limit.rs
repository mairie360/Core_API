//! Rate limiting of the public authentication routes (MAIR-390).
//!
//! Fixed-window counters kept in memory, per replica: with `n` replicas an attacker gets at most
//! `n` times the budget, which still turns password spraying and account enumeration from
//! thousands of attempts per minute into a handful. `main.rs` registers one [`RateLimits`] as app
//! data; handlers take it as `Option<web::Data<RateLimits>>`, so an app built without it (the
//! integration tests) is not limited.
//!
//! Budgets are counted per client address ([`crate::client_ip`]) and, where the route names an
//! account, per e-mail address as well, so spreading the attempts over many addresses does not
//! lift the per-account budget. `RATE_LIMIT_ENABLED=false` turns everything off (load tests).

use actix_web::http::header::RETRY_AFTER;
use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Entries kept before expired windows are swept.
const SWEEP_THRESHOLD: usize = 10_000;

/// Refusal of a rate-limited request: `429` with a `Retry-After` header (seconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyRequests {
    pub retry_after_seconds: u64,
}

impl std::fmt::Display for TooManyRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Too many requests, please try again later.")
    }
}

impl ResponseError for TooManyRequests {
    fn status_code(&self) -> StatusCode {
        StatusCode::TOO_MANY_REQUESTS
    }

    fn error_response(&self) -> HttpResponse {
        too_many_requests_response(self.retry_after_seconds)
    }
}

/// `429` response shared by every limited route.
#[must_use]
pub fn too_many_requests_response(retry_after_seconds: u64) -> HttpResponse {
    HttpResponse::TooManyRequests()
        .insert_header((RETRY_AFTER, retry_after_seconds.to_string()))
        .body(
            TooManyRequests {
                retry_after_seconds,
            }
            .to_string(),
        )
}

#[derive(Debug, Clone, Copy)]
struct Window {
    start: Instant,
    count: u32,
}

/// At most `limit` hits per key within each `period`.
#[derive(Debug)]
pub struct RateLimiter {
    limit: u32,
    period: Duration,
    windows: Mutex<HashMap<String, Window>>,
}

impl RateLimiter {
    #[must_use]
    pub fn new(limit: u32, period: Duration) -> Self {
        Self {
            limit,
            period,
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Counts one hit for `key`.
    ///
    /// # Errors
    ///
    /// [`TooManyRequests`] once `key` used up its budget for the current window.
    pub fn hit(&self, key: &str) -> Result<(), TooManyRequests> {
        self.hit_at(key, Instant::now())
    }

    fn hit_at(&self, key: &str, now: Instant) -> Result<(), TooManyRequests> {
        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if windows.len() >= SWEEP_THRESHOLD {
            let period = self.period;
            windows.retain(|_, window| now.duration_since(window.start) < period);
        }
        let window = windows.entry(key.to_string()).or_insert(Window {
            start: now,
            count: 0,
        });
        if now.duration_since(window.start) >= self.period {
            *window = Window {
                start: now,
                count: 0,
            };
        }
        if window.count >= self.limit {
            let remaining = self.period.saturating_sub(now.duration_since(window.start));
            drop(windows);
            return Err(TooManyRequests {
                retry_after_seconds: remaining.as_secs().max(1),
            });
        }
        window.count += 1;
        drop(windows);
        Ok(())
    }
}

/// Budgets of the public authentication routes.
#[derive(Debug)]
pub struct RateLimits {
    /// `POST /auth/login` and `POST /auth/keycloak`, per client address.
    pub login_per_ip: RateLimiter,
    /// `POST /auth/login`, per e-mail address.
    pub login_per_email: RateLimiter,
    /// `POST /auth/forgot_password`, per client address.
    pub forgot_password_per_ip: RateLimiter,
    /// `POST /auth/forgot_password`, per e-mail address.
    pub forgot_password_per_email: RateLimiter,
    /// `POST /auth/reset_password` and `POST /auth/force_change_password`, per client address.
    pub password_token_per_ip: RateLimiter,
    /// `POST /sessions/refresh`, per client address.
    pub refresh_per_ip: RateLimiter,
}

impl Default for RateLimits {
    fn default() -> Self {
        const MINUTE: Duration = Duration::from_secs(60);
        Self {
            login_per_ip: RateLimiter::new(30, MINUTE),
            login_per_email: RateLimiter::new(10, 15 * MINUTE),
            forgot_password_per_ip: RateLimiter::new(10, 15 * MINUTE),
            forgot_password_per_email: RateLimiter::new(3, 60 * MINUTE),
            password_token_per_ip: RateLimiter::new(10, 15 * MINUTE),
            refresh_per_ip: RateLimiter::new(60, MINUTE),
        }
    }
}

impl RateLimits {
    /// The limits to register in `main.rs`, `None` when `RATE_LIMIT_ENABLED` is `false`.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let enabled = std::env::var("RATE_LIMIT_ENABLED")
            .map_or(true, |value| !value.trim().eq_ignore_ascii_case("false"));
        enabled.then(Self::default)
    }
}

/// Key of a per-address budget.
#[must_use]
pub fn ip_key(ip: IpAddr) -> String {
    ip.to_string()
}

/// Key of a per-account budget: the e-mail address, trimmed and lowercased so case variants share
/// one budget.
#[must_use]
pub fn email_key(email: &str) -> String {
    email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_is_enforced_then_renewed() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let start = Instant::now();
        assert!(limiter.hit_at("a", start).is_ok());
        assert!(limiter.hit_at("a", start).is_ok());
        let refused = limiter.hit_at("a", start + Duration::from_secs(10));
        assert_eq!(
            refused,
            Err(TooManyRequests {
                retry_after_seconds: 50
            })
        );
        // Other keys keep their own budget.
        assert!(limiter.hit_at("b", start).is_ok());
        // A new window starts once the period is over.
        assert!(limiter.hit_at("a", start + Duration::from_secs(60)).is_ok());
    }

    #[test]
    fn email_keys_ignore_case_and_spaces() {
        assert_eq!(
            email_key(" Jean.Dupont@Mairie360.fr "),
            "jean.dupont@mairie360.fr"
        );
    }

    #[test]
    fn response_carries_retry_after() {
        let response = too_many_requests_response(42);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(RETRY_AFTER).unwrap(), "42");
    }
}
