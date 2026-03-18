use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::config::RateLimitConfig;
use crate::error::ErrorResponse;

struct IpState {
    tokens: f64,
    last_refill: Instant,
    active_connections: u32,
    last_seen: Instant,
}

struct RateLimiterInner {
    clients: Mutex<HashMap<IpAddr, IpState>>,
    requests_per_second: f64,
    burst: f64,
    max_connections_per_ip: u32,
}

#[derive(Clone)]
pub struct RateLimitState {
    enabled: bool,
    inner: Arc<RateLimiterInner>,
}

enum AcquireResult {
    Allowed,
    RateLimited { retry_after_secs: u64 },
    ConnectionLimitExceeded,
}

struct ConnectionGuard {
    inner: Arc<RateLimiterInner>,
    ip: IpAddr,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let mut clients = self.inner.clients.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = clients.get_mut(&self.ip) {
            state.active_connections = state.active_connections.saturating_sub(1);
        }
    }
}

impl RateLimitState {
    pub fn new(config: &RateLimitConfig) -> Self {
        Self {
            enabled: config.enabled,
            inner: Arc::new(RateLimiterInner {
                clients: Mutex::new(HashMap::new()),
                requests_per_second: config.requests_per_second as f64,
                burst: config.burst as f64,
                max_connections_per_ip: config.max_connections_per_ip,
            }),
        }
    }
}

pub fn spawn_cleanup_task(state: &RateLimitState) {
    if !state.enabled {
        return;
    }

    let inner = Arc::clone(&state.inner);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let staleness = std::time::Duration::from_secs(300);
        loop {
            interval.tick().await;
            let mut clients = inner.clients.lock().unwrap_or_else(|e| e.into_inner());
            clients.retain(|_ip, state| {
                state.active_connections > 0 || state.last_seen.elapsed() < staleness
            });
        }
    });
}

fn refill_tokens(ip_state: &mut IpState, rps: f64, burst: f64) {
    let elapsed = ip_state.last_refill.elapsed().as_secs_f64();
    ip_state.tokens = (ip_state.tokens + elapsed * rps).min(burst);
    ip_state.last_refill = Instant::now();
}

fn try_acquire(inner: &RateLimiterInner, ip: IpAddr) -> AcquireResult {
    let mut clients = inner.clients.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();

    let state = clients.entry(ip).or_insert_with(|| IpState {
        tokens: inner.burst,
        last_refill: now,
        active_connections: 0,
        last_seen: now,
    });

    state.last_seen = now;

    // Check connection limit first (cheapest check)
    if state.active_connections >= inner.max_connections_per_ip {
        return AcquireResult::ConnectionLimitExceeded;
    }

    // Refill tokens based on elapsed time
    refill_tokens(state, inner.requests_per_second, inner.burst);

    // Try to consume one token
    if state.tokens >= 1.0 {
        state.tokens -= 1.0;
        state.active_connections += 1;
        AcquireResult::Allowed
    } else {
        let deficit = 1.0 - state.tokens;
        let retry_after = (deficit / inner.requests_per_second).ceil() as u64;
        AcquireResult::RateLimited {
            retry_after_secs: retry_after.max(1),
        }
    }
}

fn extract_client_ip(req: &axum::extract::Request, peer: SocketAddr) -> IpAddr {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .unwrap_or_else(|| peer.ip())
}

pub async fn rate_limit_middleware(
    State(state): State<RateLimitState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    if !state.enabled {
        return next.run(req).await;
    }

    let ip = extract_client_ip(&req, peer);

    match try_acquire(&state.inner, ip) {
        AcquireResult::Allowed => {
            let _guard = ConnectionGuard {
                inner: Arc::clone(&state.inner),
                ip,
            };
            next.run(req).await
        }
        AcquireResult::RateLimited { retry_after_secs } => {
            tracing::warn!(%ip, "rate limited");
            let mut response = ErrorResponse::new(StatusCode::TOO_MANY_REQUESTS).into_response();
            response
                .headers_mut()
                .insert("retry-after", retry_after_secs.to_string().parse().unwrap());
            response
        }
        AcquireResult::ConnectionLimitExceeded => {
            tracing::warn!(%ip, "connection limit exceeded");
            let mut response = ErrorResponse::new(StatusCode::SERVICE_UNAVAILABLE).into_response();
            response
                .headers_mut()
                .insert("retry-after", "1".parse().unwrap());
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(rps: u32, burst: u32, max_conn: u32) -> RateLimitConfig {
        RateLimitConfig {
            enabled: true,
            requests_per_second: rps,
            burst,
            max_connections_per_ip: max_conn,
        }
    }

    #[test]
    fn new_ip_gets_full_bucket() {
        let state = RateLimitState::new(&test_config(10, 10, 100));
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        match try_acquire(&state.inner, ip) {
            AcquireResult::Allowed => {}
            _ => panic!("first request should be allowed"),
        }
    }

    #[test]
    fn burst_requests_allowed_then_limited() {
        let state = RateLimitState::new(&test_config(10, 5, 100));
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        // First 5 (burst) requests should succeed
        for i in 0..5 {
            // Decrement active_connections so connection limit doesn't interfere
            match try_acquire(&state.inner, ip) {
                AcquireResult::Allowed => {
                    let mut clients = state.inner.clients.lock().unwrap();
                    clients.get_mut(&ip).unwrap().active_connections -= 1;
                }
                _ => panic!("request {i} should be allowed"),
            }
        }

        // 6th request should be rate limited
        match try_acquire(&state.inner, ip) {
            AcquireResult::RateLimited { .. } => {}
            _ => panic!("burst+1 request should be rate limited"),
        }
    }

    #[test]
    fn connection_limit_enforced() {
        let state = RateLimitState::new(&test_config(1000, 1000, 2));
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        // Use up both connection slots
        match try_acquire(&state.inner, ip) {
            AcquireResult::Allowed => {}
            _ => panic!("first connection should be allowed"),
        }
        match try_acquire(&state.inner, ip) {
            AcquireResult::Allowed => {}
            _ => panic!("second connection should be allowed"),
        }

        // Third should be rejected
        match try_acquire(&state.inner, ip) {
            AcquireResult::ConnectionLimitExceeded => {}
            _ => panic!("third connection should exceed limit"),
        }
    }

    #[test]
    fn connection_guard_decrements_on_drop() {
        let state = RateLimitState::new(&test_config(1000, 1000, 2));
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        // Acquire both slots
        match try_acquire(&state.inner, ip) {
            AcquireResult::Allowed => {}
            _ => panic!("should be allowed"),
        }
        match try_acquire(&state.inner, ip) {
            AcquireResult::Allowed => {}
            _ => panic!("should be allowed"),
        }

        // Drop a guard to free one slot
        {
            let _guard = ConnectionGuard {
                inner: Arc::clone(&state.inner),
                ip,
            };
            // Guard drop will decrement by 1
        }

        // Now one slot should be free, but we have 2 active - 1 from guard = 1 active
        // Actually the guard added nothing — let's manually check
        let clients = state.inner.clients.lock().unwrap();
        // After two try_acquire: active=2, after guard drop: active=1
        assert_eq!(clients.get(&ip).unwrap().active_connections, 1);
    }

    #[test]
    fn different_ips_independent() {
        let state = RateLimitState::new(&test_config(10, 1, 100));
        let ip_a: IpAddr = "1.1.1.1".parse().unwrap();
        let ip_b: IpAddr = "2.2.2.2".parse().unwrap();

        // Exhaust IP A's bucket
        match try_acquire(&state.inner, ip_a) {
            AcquireResult::Allowed => {
                let mut clients = state.inner.clients.lock().unwrap();
                clients.get_mut(&ip_a).unwrap().active_connections -= 1;
            }
            _ => panic!("should be allowed"),
        }
        // IP A should now be limited
        match try_acquire(&state.inner, ip_a) {
            AcquireResult::RateLimited { .. } => {}
            _ => panic!("IP A should be rate limited"),
        }

        // IP B should still work
        match try_acquire(&state.inner, ip_b) {
            AcquireResult::Allowed => {}
            _ => panic!("IP B should be allowed"),
        }
    }

    #[test]
    fn disabled_state_fields() {
        let config = RateLimitConfig {
            enabled: false,
            requests_per_second: 100,
            burst: 50,
            max_connections_per_ip: 256,
        };
        let state = RateLimitState::new(&config);
        assert!(!state.enabled);
    }

    #[test]
    fn extract_ip_from_x_forwarded_for() {
        let req = axum::http::Request::builder()
            .header("x-forwarded-for", "203.0.113.50, 70.41.3.18")
            .body(axum::body::Body::empty())
            .unwrap();
        let peer: SocketAddr = "127.0.0.1:1234".parse().unwrap();

        let ip = extract_client_ip(&req, peer);
        assert_eq!(ip, "203.0.113.50".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn extract_ip_falls_back_to_peer() {
        let req = axum::http::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();
        let peer: SocketAddr = "192.168.1.1:5678".parse().unwrap();

        let ip = extract_client_ip(&req, peer);
        assert_eq!(ip, "192.168.1.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn extract_ip_invalid_xff_falls_back() {
        let req = axum::http::Request::builder()
            .header("x-forwarded-for", "not-an-ip, 1.2.3.4")
            .body(axum::body::Body::empty())
            .unwrap();
        let peer: SocketAddr = "10.0.0.1:9999".parse().unwrap();

        let ip = extract_client_ip(&req, peer);
        assert_eq!(ip, "10.0.0.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn retry_after_computed_correctly() {
        // rps=1, burst=1: after 1 request, bucket is empty
        let state = RateLimitState::new(&test_config(1, 1, 100));
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        // Consume the single token
        match try_acquire(&state.inner, ip) {
            AcquireResult::Allowed => {
                let mut clients = state.inner.clients.lock().unwrap();
                clients.get_mut(&ip).unwrap().active_connections -= 1;
            }
            _ => panic!("first request should be allowed"),
        }

        // Next request should be rate limited with retry_after = ceil(1.0 / 1.0) = 1
        match try_acquire(&state.inner, ip) {
            AcquireResult::RateLimited { retry_after_secs } => {
                assert_eq!(retry_after_secs, 1);
            }
            _ => panic!("should be rate limited"),
        }
    }
}
