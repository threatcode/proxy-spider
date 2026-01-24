//! Configuration management module.
//!
//! This module defines the application's configuration structure,
//! methods for loading and validating configuration from TOML files,
//! and utilities for handling context-specific paths (e.g., Docker).

use std::{
    collections::hash_map,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use color_eyre::eyre::{OptionExt as _, WrapErr as _};

use crate::{
    HashMap, http::BasicAuth, proxy::{AnonymityLevel, ProxyType}, raw_config,
    utils::is_docker,
};

pub const APP_DIRECTORY_NAME: &str = "proxy_spider";

#[derive(serde::Deserialize)]
pub struct HttpbinResponse {
    pub origin: String,
    pub headers: crate::HashMap<String, String>,
}

/// A proxy source configuration.
#[derive(Clone)]
pub struct Source {
    /// The URL of the proxy source.
    pub url: String,
    /// Optional basic authentication for the source.
    pub basic_auth: Option<BasicAuth>,
    /// Optional custom headers for the request.
    pub headers: Option<HashMap<String, String>>,
}

/// Configuration for the proxy scraping process.
#[derive(Clone)]
pub struct ScrapingConfig {
    /// Maximum number of proxies to extract from a single source.
    pub max_proxies_per_source: usize,
    /// Total timeout for scraping a source.
    pub timeout: Duration,
    /// Connection timeout for scraping a source.
    pub connect_timeout: Duration,
    /// Optional proxy to use for scraping.
    pub proxy: Option<url::Url>,
    /// User agent string to use for scraping requests.
    pub user_agent: String,
    /// Map of enabled protocols and their configured sources.
    pub sources: HashMap<ProxyType, Vec<Arc<Source>>>,
}

/// Configuration for the proxy checking process.
#[derive(Clone)]
pub struct CheckingConfig {
    /// The URL used to test proxy connectivity and anonymity.
    pub check_url: Option<url::Url>,
    /// Maximum number of concurrent proxy checks.
    pub max_concurrent_checks: usize,
    /// Total timeout for checking a single proxy.
    pub timeout: Duration,
    /// Connection timeout for checking a single proxy.
    pub connect_timeout: Duration,
    /// User agent string to use for checking requests.
    pub user_agent: String,
}

/// Configuration for plain text output.
#[derive(Clone)]
pub struct TxtOutputConfig {
    /// Whether plain text output is enabled.
    pub enabled: bool,
    /// Custom format for plain text output.
    pub format: Option<String>,
}

/// Configuration for JSON output.
#[derive(Clone)]
pub struct JsonOutputConfig {
    /// Whether JSON output is enabled.
    pub enabled: bool,
    /// Whether to include ASN information.
    pub include_asn: bool,
    /// Whether to include geolocation information.
    pub include_geolocation: bool,
}

/// Predefined quality profiles for proxy selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Profile {
    /// Optimized for web scraping (Anonymous+, decent speed).
    Scraping,
    /// Optimized for anonymity (Elite only).
    Stealth,
    /// Optimized for latency (Fastest proxies regardless of anonymity).
    Speed,
}

impl Profile {
    /// Applies the profile settings to the given filters.
    pub fn apply(self, filters: &mut OutputFilters) {
        match self {
            Self::Scraping => {
                filters.min_anonymity = Some(AnonymityLevel::Anonymous);
                filters.max_latency = Some(Duration::from_secs(5));
            }
            Self::Stealth => {
                filters.min_anonymity = Some(AnonymityLevel::Elite);
                filters.max_latency = Some(Duration::from_secs(5));
            }
            Self::Speed => {
                filters.max_latency = Some(Duration::from_secs(1));
            }
        }
    }
}

/// Configuration for filtering output results.
#[derive(Clone, Default)]
pub struct OutputFilters {
    /// Minimum anonymity level required.
    pub min_anonymity: Option<AnonymityLevel>,
    /// Maximum latency allowed.
    pub max_latency: Option<Duration>,
    /// List of allowed country codes.
    pub only_cc: Option<Vec<String>>,
}

/// Configuration for the processed output.
#[derive(Clone)]
pub struct OutputConfig {
    /// The base directory where output files will be saved.
    pub path: PathBuf,
    /// Whether to sort proxies by speed in the output.
    pub sort_by_speed: bool,
    /// Whether to rank proxies by score.
    pub rank: bool,
    /// Number of top proxies to include in the output.
    pub top: Option<usize>,
    /// The selected quality profile, if any.
    pub profile: Option<Profile>,
    /// Configuration for TXT output.
    pub txt: TxtOutputConfig,
    /// Configuration for JSON output.
    pub json: JsonOutputConfig,
    /// Filters to apply before generating output.
    pub filters: OutputFilters,
}

/// Configuration for the optional proxy rotation server.
#[derive(Clone)]
pub struct ServerConfig {
    /// Whether the proxy server is enabled.
    pub enabled: bool,
    /// The address and port the server binds to.
    pub bind_addr: std::net::SocketAddr,
    /// Whether to enable TOR isolation.
    pub tor_isolation: bool,
    /// Optional authentication token for the server.
    pub auth: Option<String>,
    /// Method used for proxy rotation ("random" or "sequent").
    pub rotation_method: String,
    /// Rotate to the next proxy after this many requests.
    pub rotate_after_requests: usize,
    /// Whether to rotate to the next proxy if an error occurs.
    pub rotate_on_error: bool,
    /// Whether to remove a proxy from the pool if it fails.
    pub remove_on_error: bool,
    /// Maximum number of errors allowed for a proxy before removal.
    pub max_errors: Option<isize>,
    /// Maximum number of redirects allowed.
    pub max_redirs: Option<usize>,
    /// Maximum number of retries allowed.
    pub max_retries: Option<usize>,
    /// Optional filter by country code.
    pub country_filter: Option<Vec<String>>,
    /// Whether to sync the proxy pool.
    pub sync: bool,
    /// Whether to enable verbose logging for the server.
    pub verbose: bool,
    /// Timeout for server requests.
    pub timeout: Duration,
    /// Optional path to save server logs.
    pub output: Option<PathBuf>,
}

/// The complete application configuration.
#[derive(Clone)]
pub struct Config {
    /// Whether debug logging is enabled.
    pub debug: bool,
    /// Scraping configuration.
    pub scraping: ScrapingConfig,
    /// Checking configuration.
    pub checking: CheckingConfig,
    /// Output configuration.
    pub output: OutputConfig,
    /// Server configuration.
    pub server: ServerConfig,
}

async fn get_output_path(
    raw_config: &raw_config::RawConfig,
) -> crate::Result<PathBuf> {
    let output_path = if is_docker().await {
        let mut path = tokio::task::spawn_blocking(dirs::data_local_dir)
            .await
            .wrap_err(
                "failed to spawn task for getting user's local data directory",
            )?
            .ok_or_eyre("failed to get user's local data directory")?;
        path.push(APP_DIRECTORY_NAME);
        path
    } else {
        raw_config.output.path.clone()
    };
    tokio::fs::create_dir_all(&output_path).await.wrap_err_with(|| {
        format!("failed to create output directory: {}", output_path.display())
    })?;
    Ok(output_path)
}

impl Config {
    /// Returns true if ASN database lookup is enabled.
    #[inline]
    #[must_use]
    pub const fn asn_enabled(&self) -> bool {
        self.output.json.enabled && self.output.json.include_asn
    }

    /// Returns true if geolocation database lookup is enabled.
    #[inline]
    #[must_use]
    pub const fn geolocation_enabled(&self) -> bool {
        self.output.json.enabled && self.output.json.include_geolocation
    }

    /// Returns an iterator over the enabled proxy protocols.
    pub fn enabled_protocols(
        &self,
    ) -> hash_map::Keys<'_, ProxyType, Vec<Arc<Source>>> {
        self.scraping.sources.keys()
    }

    /// Returns true if the given protocol is enabled for scraping.
    pub fn protocol_is_enabled(&self, protocol: ProxyType) -> bool {
        self.scraping.sources.contains_key(&protocol)
    }

    /// Creates a [`Config`] from a [`raw_config::RawConfig`].
    ///
    /// This method performs validation and environment-specific path handling.
    ///
    /// # Errors
    ///
    /// Returns an error if the raw configuration is invalid or if path handling fails.
    pub async fn from_raw_config(
        raw_config: raw_config::RawConfig,
    ) -> crate::Result<Self> {
        if let Err(errors) = crate::validation::validate_config(&raw_config) {
            use std::fmt::Write as _;
            let mut error_msg =
                String::from("Configuration validation failed:\n");
            for error in errors {
                writeln!(error_msg, "  - {error}").unwrap();
            }
            return Err(crate::errors::ProxySpiderError::config_invalid(
                error_msg,
            )
            .into());
        }

        let output_path = get_output_path(&raw_config).await?;

        let max_concurrent_checks =
            if let Ok(lim) = rlimit::increase_nofile_limit(u64::MAX) {
                let lim = usize::try_from(lim).unwrap_or(usize::MAX);

                if raw_config.checking.max_concurrent_checks.get() > lim {
                    tracing::warn!(
                        "max_concurrent_checks config value is too high for \
                         your OS. It will be ignored and {lim} will be used."
                    );
                    lim
                } else {
                    raw_config.checking.max_concurrent_checks.get()
                }
            } else {
                raw_config.checking.max_concurrent_checks.get()
            };

        Ok(Self {
            debug: raw_config.debug,
            scraping: ScrapingConfig {
                max_proxies_per_source: raw_config
                    .scraping
                    .max_proxies_per_source,
                timeout: Duration::from_secs_f64(raw_config.scraping.timeout),
                connect_timeout: Duration::from_secs_f64(
                    raw_config.scraping.connect_timeout,
                ),
                proxy: raw_config.scraping.proxy,
                user_agent: raw_config.scraping.user_agent,
                sources: [
                    (ProxyType::Http, raw_config.scraping.http),
                    (ProxyType::Socks4, raw_config.scraping.socks4),
                    (ProxyType::Socks5, raw_config.scraping.socks5),
                ]
                .into_iter()
                .filter_map(|(proxy_type, section)| {
                    section.enabled.then(|| {
                        (
                            proxy_type,
                            section
                                .urls
                                .into_iter()
                                .map(Into::into)
                                .map(Arc::new)
                                .collect(),
                        )
                    })
                })
                .collect(),
            },
            checking: CheckingConfig {
                check_url: raw_config.checking.check_url,
                max_concurrent_checks,
                timeout: Duration::from_secs_f64(raw_config.checking.timeout),
                connect_timeout: Duration::from_secs_f64(
                    raw_config.checking.connect_timeout,
                ),
                user_agent: raw_config.checking.user_agent,
            },
            output: OutputConfig {
                path: output_path,
                sort_by_speed: raw_config.output.sort_by_speed,
                txt: TxtOutputConfig {
                    enabled: raw_config.output.txt.enabled,
                    format: raw_config.output.txt.format,
                },
                json: JsonOutputConfig {
                    enabled: raw_config.output.json.enabled,
                    include_asn: raw_config.output.json.include_asn,
                    include_geolocation: raw_config
                        .output
                        .json
                        .include_geolocation,
                },
                rank: false,
                top: None,
                profile: None,
                filters: OutputFilters::default(),
            },
            server: ServerConfig {
                enabled: raw_config.server.enabled,
                bind_addr: format!(
                    "{}:{}",
                    raw_config.server.bind_address, raw_config.server.port
                )
                .parse()
                .wrap_err("failed to parse server bind address")?,
                tor_isolation: raw_config.server.tor_isolation,
                auth: None,
                rotation_method: "random".to_string(),
                rotate_after_requests: 1,
                rotate_on_error: false,
                remove_on_error: false,
                max_errors: Some(3),
                max_redirs: None,
                max_retries: None,
                country_filter: None,
                sync: false,
                verbose: false,
                timeout: Duration::from_secs(30),
                output: None,
            },
        })
    }
}

impl From<raw_config::SourceConfig> for Source {
    fn from(sc: raw_config::SourceConfig) -> Self {
        match sc {
            raw_config::SourceConfig::Simple(url) => {
                Self { url, basic_auth: None, headers: None }
            }
            raw_config::SourceConfig::Detailed { url, basic_auth, headers } => {
                Self { url, basic_auth, headers }
            }
        }
    }
}

/// Loads the application configuration from the default location.
///
/// This function reads the `config.toml` file, validates it,
/// and returns an [`Arc<Config>`].
///
/// # Errors
///
/// Returns an error if the config file cannot be read or is invalid.
pub async fn load_config() -> crate::Result<Arc<Config>> {
    let raw_config_path = raw_config::get_config_path();
    let raw_config = raw_config::read_config(Path::new(&raw_config_path))
        .await
        .wrap_err_with(move || {
            format!("failed to load config from {raw_config_path}")
        })?;

    let config = Config::from_raw_config(raw_config)
        .await
        .wrap_err("failed to create Config from RawConfig")?;

    Ok(Arc::new(config))
}
