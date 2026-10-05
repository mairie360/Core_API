//! Address of the client behind the reverse proxies (MAIR-390).
//!
//! `X-Forwarded-For` / `Forwarded` are only trusted when the TCP peer is itself a trusted proxy,
//! and the client is the rightmost hop of the chain that is not a trusted proxy: a client cannot
//! choose its audit address or dodge the rate limiter by sending a forged header, since the
//! entries it adds sit on the left of the ones its own proxies append.
//!
//! Trusted proxies come from `TRUSTED_PROXIES` (comma-separated addresses or CIDR ranges). When
//! the variable is absent, the loopback and private ranges are trusted (in-cluster ingress,
//! compose nginx); set it to an empty string to trust no proxy at all.

use actix_web::HttpRequest;
use ipnet::IpNet;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::LazyLock;

/// Ranges trusted when `TRUSTED_PROXIES` is not set.
const DEFAULT_TRUSTED_PROXIES: &str =
    "127.0.0.0/8,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,::1/128,fc00::/7";

static TRUSTED_PROXIES: LazyLock<Vec<IpNet>> = LazyLock::new(|| {
    let raw = std::env::var("TRUSTED_PROXIES").unwrap_or_else(|_| DEFAULT_TRUSTED_PROXIES.into());
    parse_trusted_proxies(&raw)
});

/// Parses a comma-separated list of addresses or CIDR ranges, ignoring (and logging) invalid
/// entries.
#[must_use]
pub fn parse_trusted_proxies(raw: &str) -> Vec<IpNet> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            entry
                .parse::<IpNet>()
                .or_else(|_| entry.parse::<IpAddr>().map(IpNet::from))
                .map_err(|_| tracing::warn!("Ignoring invalid TRUSTED_PROXIES entry: {entry}"))
                .ok()
        })
        .collect()
}

/// Parses one hop of a forwarding header: a bare address, `ip:port`, `[ipv6]:port`, or the
/// `for=` value of a `Forwarded` element (possibly quoted).
fn parse_hop(hop: &str) -> Option<IpAddr> {
    let hop = hop.trim().trim_matches('"');
    if let Ok(ip) = hop.parse::<IpAddr>() {
        return Some(ip);
    }
    if let Some(rest) = hop.strip_prefix('[') {
        return rest.split(']').next()?.parse().ok();
    }
    hop.rsplit_once(':')
        .and_then(|(host, _port)| host.parse().ok())
}

/// Hops of the forwarding chain, leftmost (original client) first.
fn forwarded_hops(req: &HttpRequest) -> Vec<String> {
    let headers = req.headers();
    if let Some(value) = headers.get("forwarded").and_then(|v| v.to_str().ok()) {
        return value
            .split(',')
            .filter_map(|element| {
                element.split(';').find_map(|pair| {
                    let (name, value) = pair.split_once('=')?;
                    name.trim()
                        .eq_ignore_ascii_case("for")
                        .then(|| value.to_string())
                })
            })
            .collect();
    }
    headers
        .get_all("x-forwarded-for")
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::to_string)
        .collect()
}

/// Resolves the client address from the peer address and the forwarding chain.
#[must_use]
pub fn resolve_client_ip(peer: Option<IpAddr>, hops: &[String], trusted: &[IpNet]) -> IpAddr {
    let unspecified = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    let Some(mut client) = peer else {
        return unspecified;
    };
    let is_trusted = |ip: &IpAddr| trusted.iter().any(|net| net.contains(ip));
    for hop in hops.iter().rev() {
        if !is_trusted(&client) {
            break;
        }
        match parse_hop(hop) {
            Some(ip) => client = ip,
            // An unreadable hop ends the chain: everything on its left is client-controlled.
            None => break,
        }
    }
    client
}

/// Address of the client that sent `req`, see the module documentation.
#[must_use]
pub fn client_ip(req: &HttpRequest) -> IpAddr {
    let peer = req.peer_addr().map(|addr| addr.ip());
    resolve_client_ip(peer, &forwarded_hops(req), &TRUSTED_PROXIES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hops(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    #[test]
    fn untrusted_peer_ignores_the_header() {
        let trusted = parse_trusted_proxies(DEFAULT_TRUSTED_PROXIES);
        let client = resolve_client_ip(Some(ip("203.0.113.7")), &hops(&["1.2.3.4"]), &trusted);
        assert_eq!(client, ip("203.0.113.7"));
    }

    #[test]
    fn trusted_peer_uses_the_rightmost_untrusted_hop() {
        let trusted = parse_trusted_proxies(DEFAULT_TRUSTED_PROXIES);
        // "1.2.3.4" was forged by the client, the ingress appended the real address.
        let client = resolve_client_ip(
            Some(ip("10.42.0.12")),
            &hops(&["1.2.3.4", " 198.51.100.9", "10.42.0.3"]),
            &trusted,
        );
        assert_eq!(client, ip("198.51.100.9"));
    }

    #[test]
    fn empty_list_trusts_nobody() {
        let client = resolve_client_ip(Some(ip("10.0.0.1")), &hops(&["1.2.3.4"]), &[]);
        assert_eq!(client, ip("10.0.0.1"));
    }

    #[test]
    fn hops_with_ports_and_brackets_are_parsed() {
        assert_eq!(parse_hop("198.51.100.9:4711"), Some(ip("198.51.100.9")));
        assert_eq!(parse_hop("\"[2001:db8::1]:4711\""), Some(ip("2001:db8::1")));
        assert_eq!(parse_hop("2001:db8::1"), Some(ip("2001:db8::1")));
        assert_eq!(parse_hop("unknown"), None);
    }

    #[test]
    fn invalid_entries_are_skipped() {
        let trusted = parse_trusted_proxies("10.0.0.0/8, nope, 192.0.2.1");
        assert_eq!(trusted.len(), 2);
    }
}
