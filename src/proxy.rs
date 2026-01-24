//! Proxy data models and checking logic.
//!
//! This module defines the core [`Proxy`] and [`ProxyType`] types,
//! and provides the logic for checking proxy connectivity and anonymity.

use std::{
    hash::{Hash, Hasher},
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::eyre::{WrapErr as _, eyre};

use crate::{
    config::{Config, HttpbinResponse},
    parsers::parse_ipv4,
};

/// Supported proxy protocols.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ProxyType {
    /// HTTP or HTTPS proxy.
    Http,
    /// SOCKS4 proxy.
    Socks4,
    /// SOCKS5 proxy.
    Socks5,
}

impl FromStr for ProxyType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("http") || s.eq_ignore_ascii_case("https") {
            Ok(Self::Http)
        } else if s.eq_ignore_ascii_case("socks4") {
            Ok(Self::Socks4)
        } else if s.eq_ignore_ascii_case("socks5") {
            Ok(Self::Socks5)
        } else {
            Err(eyre!("failed to convert {s} to ProxyType"))
        }
    }
}

impl ProxyType {
    /// Returns the lowercase string representation of the protocol.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Socks4 => "socks4",
            Self::Socks5 => "socks5",
        }
    }
}

/// Levels of proxy anonymity.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize,
    clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum AnonymityLevel {
    /// Reveals your real IP.
    Transparent,
    /// Hides your IP, but reveals proxy usage.
    Anonymous,
    /// Hides your IP and the fact that you're using a proxy.
    Elite,
}

impl AnonymityLevel {
    /// Returns a string representation of the anonymity level.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Transparent => "transparent",
            Self::Anonymous => "anonymous",
            Self::Elite => "elite",
        }
    }

    /// Detects anonymity level from httpbin response.
    #[must_use]
    pub fn from_httpbin(response: &HttpbinResponse) -> Self {
        let proxy_headers = [
            "via",
            "forwarded",
            "x-forwarded-for",
            "x-proxy-id",
            "x-real-ip",
            "client-ip",
            "proxy-connection",
        ];

        let mut found_proxy_header = false;
        let mut reveals_other_ip = false;

        let origin = parse_ipv4(&response.origin);

        for (name, value) in &response.headers {
            let name_lower = name.to_lowercase();
            if proxy_headers.contains(&name_lower.as_str()) {
                found_proxy_header = true;

                // Check if it reveals another IP (potential Real IP)
                if name_lower == "x-forwarded-for"
                    || name_lower == "x-real-ip"
                    || name_lower == "client-ip"
                {
                    if let Some(ref o) = origin {
                        for val in value.split(',') {
                            let trimmed = val.trim();
                            if !trimmed.is_empty() && trimmed != o {
                                reveals_other_ip = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if reveals_other_ip {
            Self::Transparent
        } else if found_proxy_header {
            Self::Anonymous
        } else {
            Self::Elite
        }
    }
}

/// Represents a proxy server with its configuration and optional check results.
#[derive(Debug, Clone, Eq)]
pub struct Proxy {
    /// The protocol used by the proxy.
    pub protocol: ProxyType,
    /// The host (IP address or domain name) of the proxy.
    pub host: String,
    /// The port number of the proxy.
    pub port: u16,
    /// Optional username for basic authentication.
    pub username: Option<String>,
    /// Optional password for basic authentication.
    pub password: Option<String>,
    /// The measured latency if the proxy has been checked.
    pub timeout: Option<Duration>,
    /// The IP address of the proxy as seen by the target (exit IP).
    pub exit_ip: Option<String>,
    /// The detected anonymity level.
    pub anonymity: Option<AnonymityLevel>,
    /// The calculated quality score (0-100).
    pub score: Option<u8>,
}

use compact_str::{CompactString, format_compact};

impl TryFrom<&mut Proxy> for reqwest::Proxy {
    type Error = crate::Error;

    #[inline]
    fn try_from(value: &mut Proxy) -> Result<Self, Self::Error> {
        let url = format_compact!(
            "{}://{}:{}",
            value.protocol.as_str(),
            value.host,
            value.port
        );
        let proxy = Self::all(url.as_str())
            .wrap_err("failed to create reqwest::Proxy")?;

        if let (Some(username), Some(password)) =
            (value.username.as_ref(), value.password.as_ref())
        {
            Ok(proxy.basic_auth(username, password))
        } else {
            Ok(proxy)
        }
    }
}

impl Proxy {
    /// Returns true if the proxy has been successfully checked.
    pub const fn is_checked(&self) -> bool {
        self.timeout.is_some()
    }

    pub fn calculate_score(
        latency: Duration,
        anonymity: AnonymityLevel,
        max_timeout: Duration,
    ) -> u8 {
        let anon_score = match anonymity {
            AnonymityLevel::Elite => 50.0,
            AnonymityLevel::Anonymous => 25.0,
            AnonymityLevel::Transparent => 0.0,
        };

        let max_timeout_secs = max_timeout.as_secs_f64();
        let lat_score = if max_timeout_secs > 0.0 {
            (1.0 - (latency.as_secs_f64() / max_timeout_secs).min(1.0)) * 50.0
        } else {
            0.0
        };

        (anon_score + lat_score).round() as u8
    }

    /// Checks the proxy's connectivity and anonymity.
    ///
    /// This method updates the `timeout`, `exit_ip`, `anonymity` and `score` fields if the check is successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails or if the response status is not successful.
    pub async fn check<R: reqwest::dns::Resolve + 'static>(
        &mut self,
        config: &Config,
        dns_resolver: Arc<R>,
    ) -> crate::Result<()> {
        if let Some(check_url) = &config.checking.check_url {
            let builder = reqwest::ClientBuilder::new()
                .user_agent(&config.checking.user_agent)
                .proxy(self.try_into()?)
                .timeout(config.checking.timeout)
                .connect_timeout(config.checking.connect_timeout)
                .pool_max_idle_per_host(0)
                .http1_only()
                .tcp_keepalive(None)
                .tcp_keepalive_interval(Duration::ZERO)
                .tcp_keepalive_retries(0)
                .dns_resolver(dns_resolver);
            #[cfg(any(
                target_os = "android",
                target_os = "fuchsia",
                target_os = "linux"
            ))]
            let builder = builder.tcp_user_timeout(None);
            let client = builder.build()?;
            let start = Instant::now();
            let response = client
                .get(check_url.clone())
                .send()
                .await?
                .error_for_status()?;
            drop(client);
            let latency = start.elapsed();
            self.timeout = Some(latency);
            let response_text = response.text().await.wrap_err("failed to read response text")?;
            
            if let Ok(httpbin) = serde_json::from_str::<HttpbinResponse>(&response_text) {
                let origin = parse_ipv4(&httpbin.origin);
                self.exit_ip = origin.clone();
                let anonymity = AnonymityLevel::from_httpbin(&httpbin);
                self.anonymity = Some(anonymity);
                
                self.score = Some(Self::calculate_score(
                    latency,
                    anonymity,
                    config.checking.timeout,
                ));
            } else {
                self.exit_ip = parse_ipv4(&response_text);
                self.anonymity = None;
                self.score = None;
            }
        }
        Ok(())
    }

    #[inline]
    #[must_use]
    pub fn to_string(&self, include_protocol: bool) -> CompactString {
        let mut s = CompactString::default();

        if include_protocol {
            s.push_str(self.protocol.as_str());
            s.push_str("://");
        }

        if let (Some(username), Some(password)) =
            (&self.username, &self.password)
        {
            s.push_str(username);
            s.push(':');
            s.push_str(password);
            s.push('@');
        }

        s.push_str(&self.host);
        s.push(':');
        s.push_str(itoa::Buffer::new().format(self.port));

        s
    }
}

#[expect(clippy::missing_trait_methods)]
impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        self.protocol == other.protocol
            && self.host == other.host
            && self.port == other.port
            && self.username == other.username
            && self.password == other.password
    }
}

#[expect(clippy::missing_trait_methods)]
impl Hash for Proxy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.protocol.hash(state);
        self.host.hash(state);
        self.port.hash(state);
        self.username.hash(state);
        self.password.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HashMap;

    #[test]
    fn test_anonymity_elite() {
        let response = HttpbinResponse {
            origin: "1.2.3.4".into(),
            headers: HashMap::default(),
        };
        assert_eq!(AnonymityLevel::from_httpbin(&response), AnonymityLevel::Elite);
    }

    #[test]
    fn test_anonymity_anonymous() {
        let mut headers = HashMap::default();
        headers.insert("Via".into(), "1.1 proxy".into());
        let response = HttpbinResponse {
            origin: "1.2.3.4".into(),
            headers,
        };
        assert_eq!(AnonymityLevel::from_httpbin(&response), AnonymityLevel::Anonymous);
    }

    #[test]
    fn test_anonymity_transparent() {
        let mut headers = HashMap::default();
        headers.insert("X-Forwarded-For".into(), "5.6.7.8".into());
        let response = HttpbinResponse {
            origin: "1.2.3.4".into(),
            headers,
        };
        assert_eq!(AnonymityLevel::from_httpbin(&response), AnonymityLevel::Transparent);
    }

    #[test]
    fn test_anonymity_transparent_real_ip_with_proxy() {
        let mut headers = HashMap::default();
        headers.insert("X-Forwarded-For".into(), "5.6.7.8, 1.2.3.4".into());
        let response = HttpbinResponse {
            origin: "1.2.3.4".into(),
            headers,
        };
        // It reveals 5.6.7.8 which is not origin 1.2.3.4
        assert_eq!(AnonymityLevel::from_httpbin(&response), AnonymityLevel::Transparent);
    }

    #[test]
    fn test_calculate_proxy_score() {
        let max_timeout = Duration::from_secs(10);
        
        // Elite + Perfect Speed = 100
        let score = Proxy::calculate_score(Duration::ZERO, AnonymityLevel::Elite, max_timeout);
        assert_eq!(score, 100);
        
        // Elite + 50% Speed = 75
        let score = Proxy::calculate_score(Duration::from_secs(5), AnonymityLevel::Elite, max_timeout);
        assert_eq!(score, 75);

        // Elite + 0% Speed = 50
        let score = Proxy::calculate_score(Duration::from_secs(10), AnonymityLevel::Elite, max_timeout);
        assert_eq!(score, 50);

        // Anonymous + Perfect Speed = 75 (25 + 50)
        let score = Proxy::calculate_score(Duration::ZERO, AnonymityLevel::Anonymous, max_timeout);
        assert_eq!(score, 75);

        // Transparent + Perfect Speed = 50 (0 + 50)
        let score = Proxy::calculate_score(Duration::ZERO, AnonymityLevel::Transparent, max_timeout);
        assert_eq!(score, 50);

        // Transparent + 0% Speed = 0
        let score = Proxy::calculate_score(Duration::from_secs(10), AnonymityLevel::Transparent, max_timeout);
        assert_eq!(score, 0);
    }
}
