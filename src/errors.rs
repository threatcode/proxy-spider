//! Custom error types for better error handling and user experience.
//!
//! This module provides structured error types with error codes for programmatic handling
//! and user-friendly messages with actionable suggestions.

use std::fmt;

/// Error codes for programmatic error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorCode {
    /// Configuration file not found or inaccessible
    ConfigNotFound,
    /// Configuration file has invalid syntax or values
    ConfigInvalid,
    /// Network request failed
    NetworkError,
    /// Proxy check failed
    ProxyCheckFailed,
    /// Invalid proxy format
    InvalidProxyFormat,
    /// File I/O error
    FileIoError,
    /// Database error (MaxMind)
    DatabaseError,
    /// Parsing error
    ParseError,
    /// Timeout error
    Timeout,
    /// Permission denied
    PermissionDenied,
    /// Resource not found
    NotFound,
    /// Internal error
    Internal,
}

impl ErrorCode {
    /// Get the error code as a string identifier
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigInvalid => "CONFIG_INVALID",
            Self::NetworkError => "NETWORK_ERROR",
            Self::ProxyCheckFailed => "PROXY_CHECK_FAILED",
            Self::InvalidProxyFormat => "INVALID_PROXY_FORMAT",
            Self::FileIoError => "FILE_IO_ERROR",
            Self::DatabaseError => "DATABASE_ERROR",
            Self::ParseError => "PARSE_ERROR",
            Self::Timeout => "TIMEOUT",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::NotFound => "NOT_FOUND",
            Self::Internal => "INTERNAL",
        }
    }

    /// Get a user-friendly description of the error
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::ConfigNotFound => "Configuration file not found",
            Self::ConfigInvalid => "Configuration file is invalid",
            Self::NetworkError => "Network request failed",
            Self::ProxyCheckFailed => "Proxy check failed",
            Self::InvalidProxyFormat => "Invalid proxy format",
            Self::FileIoError => "File operation failed",
            Self::DatabaseError => "Database operation failed",
            Self::ParseError => "Failed to parse data",
            Self::Timeout => "Operation timed out",
            Self::PermissionDenied => "Permission denied",
            Self::NotFound => "Resource not found",
            Self::Internal => "Internal error",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Structured error type with code, message, and optional suggestion
#[derive(Debug)]
pub struct ProxySpiderError {
    /// Error code for programmatic handling
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional suggestion for how to fix the error
    pub suggestion: Option<String>,
    /// Optional source error
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ProxySpiderError {
    /// Create a new error with code and message
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), suggestion: None, source: None }
    }

    /// Create a new error with code, message, and suggestion
    pub fn with_suggestion(
        code: ErrorCode,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            suggestion: Some(suggestion.into()),
            source: None,
        }
    }

    /// Add a suggestion to this error
    #[must_use]
    pub fn suggest(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Add a source error
    #[must_use]
    pub fn with_source(
        mut self,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Get the error code
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// Get the error message
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the suggestion, if any
    #[must_use]
    pub fn suggestion(&self) -> Option<&str> {
        self.suggestion.as_deref()
    }

    /// Create a configuration not found error
    pub fn config_not_found(path: impl fmt::Display) -> Self {
        Self::with_suggestion(
            ErrorCode::ConfigNotFound,
            format!("Configuration file not found: {path}"),
            "Ensure config.toml exists in the current directory or specify a custom path",
        )
    }

    /// Create a configuration invalid error
    pub fn config_invalid(reason: impl Into<String>) -> Self {
        Self::with_suggestion(
            ErrorCode::ConfigInvalid,
            format!("Invalid configuration: {}", reason.into()),
            "Check the configuration file for syntax errors or invalid values",
        )
    }

    /// Create a network error
    pub fn network_error(
        url: impl fmt::Display,
        reason: impl Into<String>,
    ) -> Self {
        Self::with_suggestion(
            ErrorCode::NetworkError,
            format!("Network request to {url} failed: {}", reason.into()),
            "Check your internet connection and ensure the URL is accessible",
        )
    }

    /// Create a proxy check failed error
    pub fn proxy_check_failed(
        proxy: impl fmt::Display,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorCode::ProxyCheckFailed,
            format!("Proxy {proxy} check failed: {}", reason.into()),
        )
    }

    /// Create an invalid proxy format error
    pub fn invalid_proxy_format(proxy: impl Into<String>) -> Self {
        Self::with_suggestion(
            ErrorCode::InvalidProxyFormat,
            format!("Invalid proxy format: {}", proxy.into()),
            "Expected format: [protocol://][username:password@]host:port",
        )
    }

    /// Create a file I/O error
    pub fn file_io_error(
        path: impl fmt::Display,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorCode::FileIoError,
            format!("File operation on {path} failed: {}", reason.into()),
        )
    }

    /// Create a database error
    pub fn database_error(
        db_name: impl fmt::Display,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorCode::DatabaseError,
            format!("{db_name} database error: {}", reason.into()),
        )
    }

    /// Create a parsing error
    pub fn parse_error(
        target: impl fmt::Display,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorCode::ParseError,
            format!("Failed to parse {target}: {}", reason.into()),
        )
    }

    /// Create a timeout error
    pub fn timeout(operation: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::Timeout,
            format!("Operation timed out: {operation}"),
        )
    }

    /// Create a permission denied error
    pub fn permission_denied(resource: impl fmt::Display) -> Self {
        Self::with_suggestion(
            ErrorCode::PermissionDenied,
            format!("Permission denied: {resource}"),
            "Ensure you have the necessary permissions to access this resource",
        )
    }

    /// Create a resource not found error
    pub fn not_found(resource: impl fmt::Display) -> Self {
        Self::new(ErrorCode::NotFound, format!("Resource not found: {resource}"))
    }

    /// Create an internal error
    pub fn internal(reason: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::Internal,
            format!("Internal error: {}", reason.into()),
        )
    }
}

impl fmt::Display for ProxySpiderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n💡 Suggestion: {suggestion}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProxySpiderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}
