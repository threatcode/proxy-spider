//! Configuration validation module
//!
//! Provides comprehensive validation for configuration values to catch errors early
//! and provide helpful feedback to users.



use url::Url;

use crate::raw_config::RawConfig;

/// Validation error with field name and reason
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub reason: String,
    pub suggestion: Option<String>,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
            suggestion: None,
        }
    }

    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Field '{}': {}", self.field, self.reason)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, " (Suggestion: {suggestion})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// Result type for validation
pub type ValidationResult = Result<(), Vec<ValidationError>>;

/// Validate the entire configuration
pub fn validate_config(config: &RawConfig) -> ValidationResult {
    let mut errors = Vec::new();

    // Validate scraping config
    if let Err(mut e) = validate_scraping_config(config) {
        errors.append(&mut e);
    }

    // Validate checking config
    if let Err(mut e) = validate_checking_config(config) {
        errors.append(&mut e);
    }

    // Validate output config
    if let Err(mut e) = validate_output_config(config) {
        errors.append(&mut e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate scraping configuration
fn validate_scraping_config(config: &RawConfig) -> ValidationResult {
    let mut errors = Vec::new();

    // Validate timeout values
    if config.scraping.timeout <= 0.0 {
        errors.push(
            ValidationError::new(
                "scraping.timeout",
                "Timeout must be greater than 0",
            )
            .with_suggestion("Set a reasonable timeout like 60.0 seconds"),
        );
    } else if config.scraping.timeout > 300.0 {
        errors.push(
            ValidationError::new(
                "scraping.timeout",
                "Timeout is very high (>300s)",
            )
            .with_suggestion("Consider using a lower timeout to avoid long waits"),
        );
    }

    if config.scraping.connect_timeout <= 0.0 {
        errors.push(
            ValidationError::new(
                "scraping.connect_timeout",
                "Connect timeout must be greater than 0",
            )
            .with_suggestion("Set a reasonable timeout like 5.0 seconds"),
        );
    } else if config.scraping.connect_timeout > config.scraping.timeout {
        errors.push(
            ValidationError::new(
                "scraping.connect_timeout",
                "Connect timeout should not exceed total timeout",
            )
            .with_suggestion("Set connect_timeout <= timeout"),
        );
    }

    // Validate proxy URL if provided
    if !config.scraping.proxy.is_none() {
        if let Some(proxy_url) = &config.scraping.proxy {
            if let Err(e) = validate_proxy_url(proxy_url) {
                errors.push(
                    ValidationError::new("scraping.proxy", format!("Invalid proxy URL: {e}"))
                        .with_suggestion("Use format: protocol://host:port"),
                );
            }
        }
    }

    // Validate user agent
    if config.scraping.user_agent.is_empty() {
        errors.push(
            ValidationError::new(
                "scraping.user_agent",
                "User agent cannot be empty",
            )
            .with_suggestion("Set a valid user agent string"),
        );
    }

    // Validate sources
    let has_enabled_sources = config.scraping.http.enabled
        || config.scraping.socks4.enabled
        || config.scraping.socks5.enabled;

    if !has_enabled_sources {
        errors.push(
            ValidationError::new(
                "scraping.sources",
                "At least one proxy protocol must be enabled",
            )
            .with_suggestion("Enable http, socks4, or socks5 in config"),
        );
    }

    // Validate HTTP sources
    if config.scraping.http.enabled {
        if config.scraping.http.urls.is_empty() {
            errors.push(
                ValidationError::new(
                    "scraping.http.urls",
                    "HTTP is enabled but no URLs provided",
                )
                .with_suggestion("Add at least one HTTP proxy source URL"),
            );
        } else {
            for (i, source) in config.scraping.http.urls.iter().enumerate() {
                if let Err(e) = validate_source_url(source) {
                    errors.push(ValidationError::new(
                        format!("scraping.http.urls[{i}]"),
                        e,
                    ));
                }
            }
        }
    }

    // Validate SOCKS4 sources
    if config.scraping.socks4.enabled {
        if config.scraping.socks4.urls.is_empty() {
            errors.push(
                ValidationError::new(
                    "scraping.socks4.urls",
                    "SOCKS4 is enabled but no URLs provided",
                )
                .with_suggestion("Add at least one SOCKS4 proxy source URL"),
            );
        } else {
            for (i, source) in config.scraping.socks4.urls.iter().enumerate() {
                if let Err(e) = validate_source_url(source) {
                    errors.push(ValidationError::new(
                        format!("scraping.socks4.urls[{i}]"),
                        e,
                    ));
                }
            }
        }
    }

    // Validate SOCKS5 sources
    if config.scraping.socks5.enabled {
        if config.scraping.socks5.urls.is_empty() {
            errors.push(
                ValidationError::new(
                    "scraping.socks5.urls",
                    "SOCKS5 is enabled but no URLs provided",
                )
                .with_suggestion("Add at least one SOCKS5 proxy source URL"),
            );
        } else {
            for (i, source) in config.scraping.socks5.urls.iter().enumerate() {
                if let Err(e) = validate_source_url(source) {
                    errors.push(ValidationError::new(
                        format!("scraping.socks5.urls[{i}]"),
                        e,
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate checking configuration
fn validate_checking_config(config: &RawConfig) -> ValidationResult {
    let mut errors = Vec::new();

    // Validate check URL if provided
    if let Some(check_url) = &config.checking.check_url {
        if let Err(e) = validate_check_url(check_url) {
            errors.push(
                ValidationError::new("checking.check_url", format!("Invalid check URL: {e}"))
                    .with_suggestion("Use a valid HTTP/HTTPS URL"),
            );
        }
    }

    // Validate max concurrent checks
    if config.checking.max_concurrent_checks.get() == 0 {
        errors.push(
            ValidationError::new(
                "checking.max_concurrent_checks",
                "Must be greater than 0",
            )
            .with_suggestion("Set to at least 1, recommended: 512-2048"),
        );
    } else if config.checking.max_concurrent_checks.get() > 100_000 {
        errors.push(
            ValidationError::new(
                "checking.max_concurrent_checks",
                "Value is extremely high (>100,000)",
            )
            .with_suggestion(
                "This may cause system resource exhaustion. Consider a lower value.",
            ),
        );
    }

    // Validate timeout values
    if config.checking.timeout <= 0.0 {
        errors.push(
            ValidationError::new(
                "checking.timeout",
                "Timeout must be greater than 0",
            )
            .with_suggestion("Set a reasonable timeout like 60.0 seconds"),
        );
    } else if config.checking.timeout > 300.0 {
        errors.push(
            ValidationError::new(
                "checking.timeout",
                "Timeout is very high (>300s)",
            )
            .with_suggestion("Consider using a lower timeout for faster checking"),
        );
    }

    if config.checking.connect_timeout <= 0.0 {
        errors.push(
            ValidationError::new(
                "checking.connect_timeout",
                "Connect timeout must be greater than 0",
            )
            .with_suggestion("Set a reasonable timeout like 5.0 seconds"),
        );
    } else if config.checking.connect_timeout > config.checking.timeout {
        errors.push(
            ValidationError::new(
                "checking.connect_timeout",
                "Connect timeout should not exceed total timeout",
            )
            .with_suggestion("Set connect_timeout <= timeout"),
        );
    }

    // Validate user agent
    if config.checking.user_agent.is_empty() {
        errors.push(
            ValidationError::new(
                "checking.user_agent",
                "User agent cannot be empty",
            )
            .with_suggestion("Set a valid user agent string"),
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate output configuration
fn validate_output_config(config: &RawConfig) -> ValidationResult {
    let mut errors = Vec::new();

    // Validate that at least one output format is enabled
    if !config.output.txt.enabled && !config.output.json.enabled {
        errors.push(
            ValidationError::new(
                "output",
                "At least one output format must be enabled",
            )
            .with_suggestion("Enable either txt or json output"),
        );
    }

    // Validate path is not empty
    if config.output.path.as_os_str().is_empty() {
        errors.push(
            ValidationError::new("output.path", "Output path cannot be empty")
                .with_suggestion("Set to './out' or another valid directory"),
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a proxy URL
fn validate_proxy_url(url: &Url) -> Result<(), String> {
    match url.scheme() {
        "http" | "https" | "socks4" | "socks5" => {}
        scheme => {
            return Err(format!(
                "Unsupported proxy protocol: {scheme}. Use http, https, socks4, or socks5"
            ));
        }
    }

    if url.host_str().is_none() {
        return Err("Proxy URL must have a host".to_string());
    }

    if url.port().is_none() {
        return Err("Proxy URL must have a port".to_string());
    }

    Ok(())
}

/// Validate a check URL
fn validate_check_url(url: &Url) -> Result<(), String> {
    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(format!(
                "Unsupported check URL protocol: {scheme}. Use http or https"
            ));
        }
    }

    if url.host_str().is_none() {
        return Err("Check URL must have a host".to_string());
    }

    Ok(())
}

/// Validate a source URL (can be file:// or http(s)://)
fn validate_source_url(source: &crate::raw_config::SourceConfig) -> Result<(), String> {
    let url_str = match source {
        crate::raw_config::SourceConfig::Simple(url) => url,
        crate::raw_config::SourceConfig::Detailed { url, .. } => url,
    };

    // Try to parse as URL first
    if let Ok(url) = Url::parse(url_str) {
        match url.scheme() {
            "http" | "https" | "file" => return Ok(()),
            scheme => {
                return Err(format!(
                    "Unsupported source URL protocol: {scheme}. Use http, https, or file"
                ));
            }
        }
    }

    // If not a valid URL, assume it's a file path
    // File paths are validated at runtime
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;

    fn create_minimal_valid_config() -> RawConfig {
        RawConfig {
            debug: false,
            scraping: crate::raw_config::ScrapingConfig {
                max_proxies_per_source: 1000,
                timeout: 60.0,
                connect_timeout: 5.0,
                proxy: None,
                user_agent: "Test Agent".to_string(),
                http: crate::raw_config::ScrapingProtocolConfig {
                    enabled: true,
                    urls: vec![crate::raw_config::SourceConfig::Simple(
                        "https://example.com/proxies.txt".to_string(),
                    )],
                },
                socks4: crate::raw_config::ScrapingProtocolConfig {
                    enabled: false,
                    urls: vec![],
                },
                socks5: crate::raw_config::ScrapingProtocolConfig {
                    enabled: false,
                    urls: vec![],
                },
            },
            checking: crate::raw_config::CheckingConfig {
                check_url: Some(Url::parse("https://api.ipify.org").unwrap()),
                max_concurrent_checks: NonZeroUsize::new(1024).unwrap(),
                timeout: 60.0,
                connect_timeout: 5.0,
                user_agent: "Test Agent".to_string(),
            },
            output: crate::raw_config::OutputConfig {
                path: std::path::PathBuf::from("./out"),
                sort_by_speed: true,
                txt: crate::raw_config::TxtOutputConfig { enabled: true },
                json: crate::raw_config::JsonOutputConfig {
                    enabled: false,
                    include_asn: false,
                    include_geolocation: false,
                },
            },
        }
    }

    #[test]
    fn test_valid_config() {
        let config = create_minimal_valid_config();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_invalid_timeout() {
        let mut config = create_minimal_valid_config();
        config.scraping.timeout = -1.0;
        
        let result = validate_config(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field.contains("timeout")));
    }

    #[test]
    fn test_no_enabled_protocols() {
        let mut config = create_minimal_valid_config();
        config.scraping.http.enabled = false;
        
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_output_formats() {
        let mut config = create_minimal_valid_config();
        config.output.txt.enabled = false;
        config.output.json.enabled = false;
        
        let result = validate_config(&config);
        assert!(result.is_err());
    }
}
