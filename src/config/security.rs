use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::defaults::*;
use super::types::WafMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub mode: WafMode,
    #[serde(default)]
    pub rules_path: Option<PathBuf>,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

impl Default for WafConfig {
    fn default() -> Self {
        Self {
            enable: false,
            mode: WafMode::default(),
            rules_path: None,
            rate_limit: RateLimitConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_rate_limit")]
    pub requests_per_ip: u32,
    #[serde(default = "default_window_seconds")]
    pub window_seconds: u64,
    #[serde(default = "default_burst")]
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_ip: default_rate_limit(),
            window_seconds: default_window_seconds(),
            burst: default_burst(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub database_path: Option<PathBuf>,
    #[serde(default)]
    pub allowed_countries: Vec<String>,
    #[serde(default)]
    pub blocked_countries: Vec<String>,
}

impl Default for GeoIpConfig {
    fn default() -> Self {
        Self {
            enable: false,
            database_path: None,
            allowed_countries: Vec::new(),
            blocked_countries: Vec::new(),
        }
    }
}
