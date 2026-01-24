//! HTTP client utilities and middleware.
//!
//! This module provides functions for creating configured `reqwest` clients
//! and implements a custom [`RetryMiddleware`] for handling transient network errors.

use std::{
    io,
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::config::Config;

const DEFAULT_MAX_RETRIES: u32 = 2;
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(8);

static RETRY_STATUSES: &[reqwest::StatusCode] = &[
    reqwest::StatusCode::REQUEST_TIMEOUT,
    reqwest::StatusCode::TOO_MANY_REQUESTS,
    reqwest::StatusCode::INTERNAL_SERVER_ERROR,
    reqwest::StatusCode::BAD_GATEWAY,
    reqwest::StatusCode::SERVICE_UNAVAILABLE,
    reqwest::StatusCode::GATEWAY_TIMEOUT,
];

/// Basic authentication credentials.
#[derive(Clone, serde::Deserialize)]
pub struct BasicAuth {
    /// The username for authentication.
    pub username: String,
    /// The optional password for authentication.
    pub password: Option<String>,
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    if let Some(val) = headers.get("retry-after-ms")
        && let Ok(s) = val.to_str()
        && let Ok(ms) = s.parse()
    {
        return Some(Duration::from_millis(ms));
    }

    if let Some(val) = headers.get(reqwest::header::RETRY_AFTER)
        && let Ok(s) = val.to_str()
    {
        if let Ok(sec) = s.parse() {
            return Some(Duration::from_secs(sec));
        }

        if let Ok(parsed) = httpdate::parse_http_date(s)
            && let Ok(dur) = parsed.duration_since(SystemTime::now())
        {
            return Some(dur);
        }
    }
    None
}

fn calculate_retry_timeout(
    headers: Option<&reqwest::header::HeaderMap>,
    attempt: u32,
) -> Option<Duration> {
    if let Some(h) = headers
        && let Some(after) = parse_retry_after(h)
    {
        if after > Duration::from_secs(60) {
            return None;
        }
        return Some(after);
    }

    let base = INITIAL_RETRY_DELAY
        .saturating_mul(2_u32.pow(attempt))
        .min(MAX_RETRY_DELAY);
    let jitter = 0.25_f64.mul_add(-rand::random::<f64>(), 1.0);
    Some(base.mul_f64(jitter))
}

/// Middleware that automatically retries failed requests based on status codes and delays.
pub struct RetryMiddleware;

#[async_trait::async_trait]
impl reqwest_middleware::Middleware for RetryMiddleware {
    async fn handle(
        &self,
        req: reqwest::Request,
        extensions: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let mut attempt: u32 = 0;
        loop {
            let req = req.try_clone().ok_or_else(|| {
                reqwest_middleware::Error::middleware(io::Error::other(
                    "Request object is not cloneable",
                ))
            })?;

            match next.clone().run(req, extensions).await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_client_error() || status.is_server_error() {
                        if attempt < DEFAULT_MAX_RETRIES
                            && RETRY_STATUSES.contains(&status)
                            && let Some(delay) = calculate_retry_timeout(
                                Some(resp.headers()),
                                attempt,
                            )
                        {
                            tokio::time::sleep(delay).await;
                            attempt = attempt.saturating_add(1);
                            continue;
                        }
                        resp.error_for_status_ref()?;
                    }
                    return Ok(resp);
                }
                Err(err) => {
                    if attempt < DEFAULT_MAX_RETRIES
                        && err.is_connect()
                        && let Some(delay) =
                            calculate_retry_timeout(None, attempt)
                    {
                        tokio::time::sleep(delay).await;
                        attempt = attempt.saturating_add(1);
                        continue;
                    }
                    return Err(err);
                }
            }
        }
    }
}

/// Creates a new `reqwest` client with middleware and DNS resolver.
///
/// # Errors
///
/// Returns an error if the client cannot be built.
pub fn create_client<R: reqwest::dns::Resolve + 'static>(
    config: &Config,
    dns_resolver: Arc<R>,
    timeout: Option<Duration>,
) -> reqwest::Result<reqwest_middleware::ClientWithMiddleware> {
    let mut builder = reqwest::ClientBuilder::new()
        .user_agent(&config.scraping.user_agent)
        .connect_timeout(config.scraping.connect_timeout)
        .pool_idle_timeout(Duration::from_secs(5))
        .dns_resolver(dns_resolver);

    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    }

    if let Some(proxy) = &config.scraping.proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy.clone())?);
    }

    let client = builder.build()?;
    let client_with_middleware = reqwest_middleware::ClientBuilder::new(client)
        .with(RetryMiddleware)
        .build();

    Ok(client_with_middleware)
}
