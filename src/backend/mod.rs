pub mod embedded;
pub mod fastcgi;
pub mod static_files;
pub mod router;

use crate::php::{PhpRequest, PhpResponse};
use anyhow::Result;
use std::fmt;
use std::time::Duration;

pub trait Backend: Send + Sync {
    fn execute(&self, request: PhpRequest) -> Result<PhpResponse, BackendError>;

    fn health_check(&self) -> Result<HealthStatus>;

    fn backend_type(&self) -> BackendType;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendType {
    Embedded,
    FastCGI,
    Static,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Embedded => write!(f, "embedded"),
            Self::FastCGI => write!(f, "fastcgi"),
            Self::Static => write!(f, "static"),
        }
    }
}

impl std::str::FromStr for BackendType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "embedded" => Ok(Self::Embedded),
            "fastcgi" => Ok(Self::FastCGI),
            "static" => Ok(Self::Static),
            _ => Err(anyhow::anyhow!("Invalid backend type: '{}'", s)),
        }
    }
}

#[derive(Debug)]
pub enum BackendError {
    ConnectionFailed(String),
    Timeout,
    ProtocolError(String),
    PhpError(String),
    IoError(std::io::Error),
    NotFound(String),
    Other(anyhow::Error),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            Self::Timeout => write!(f, "Request timeout"),
            Self::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            Self::PhpError(msg) => write!(f, "PHP error: {}", msg),
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::NotFound(path) => write!(f, "Not found: {}", path),
            Self::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for BackendError {}

impl From<std::io::Error> for BackendError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<anyhow::Error> for BackendError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err)
    }
}

/// Health check status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub latency: Option<Duration>,
}

impl HealthStatus {
    pub fn healthy(message: impl Into<String>) -> Self {
        Self {
            healthy: true,
            message: message.into(),
            latency: None,
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: message.into(),
            latency: None,
        }
    }

    pub fn with_latency(mut self, latency: Duration) -> Self {
        self.latency = Some(latency);
        self
    }
}

#[derive(Debug, Clone)]
pub enum PathPattern {
    Exact(String),
    Prefix(String),
    Suffix(String),
    Regex(regex::Regex),
}

impl PathPattern {
    pub fn matches(&self, path: &str) -> bool {
        match self {
            Self::Exact(pattern) => path == pattern,
            Self::Prefix(prefix) => {
                let prefix_clean = prefix.trim_end_matches('*').trim_end_matches('/');
                path.starts_with(prefix_clean)
            }
            Self::Suffix(suffix) => {
                let suffix_clean = suffix.trim_start_matches('*');
                path.ends_with(suffix_clean)
            }
            Self::Regex(regex) => regex.is_match(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_pattern_exact() {
        let pattern = PathPattern::Exact("/api/user".to_string());
        assert!(pattern.matches("/api/user"));
        assert!(!pattern.matches("/api/users"));
        assert!(!pattern.matches("/api"));
    }

    #[test]
    fn test_path_pattern_prefix() {
        let pattern = PathPattern::Prefix("/api/*".to_string());
        assert!(pattern.matches("/api/user"));
        assert!(pattern.matches("/api/users"));
        assert!(pattern.matches("/api/"));
        assert!(!pattern.matches("/other"));
    }

    #[test]
    fn test_path_pattern_suffix() {
        let pattern = PathPattern::Suffix("*.php".to_string());
        assert!(pattern.matches("/index.php"));
        assert!(pattern.matches("/api/user.php"));
        assert!(!pattern.matches("/index.html"));
    }

    #[test]
    fn test_path_pattern_regex() {
        let pattern = PathPattern::Regex(regex::Regex::new(r"^/api/v\d+/").unwrap());
        assert!(pattern.matches("/api/v1/user"));
        assert!(pattern.matches("/api/v2/users"));
        assert!(!pattern.matches("/api/user"));
        assert!(!pattern.matches("/api/v/user"));
    }
}
