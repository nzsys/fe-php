pub mod parser;
pub mod validator;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub php: PhpConfig,
    pub logging: LoggingConfig,
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub waf: WafConfig,
    #[serde(default)]
    pub admin: AdminConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpConfig {
    pub libphp_path: PathBuf,
    pub document_root: PathBuf,
    #[serde(default = "default_workers")]
    pub worker_pool_size: usize,
    #[serde(default = "default_max_requests")]
    pub worker_max_requests: usize,
    #[serde(default)]
    pub opcache: OpcacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcacheConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_opcache_memory")]
    pub memory_size: String,
    #[serde(default = "default_max_files")]
    pub max_files: usize,
    #[serde(default)]
    pub validate_timestamps: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default = "default_log_output")]
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_metrics_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_waf_mode")]
    pub mode: String,
    #[serde(default)]
    pub rules_path: Option<PathBuf>,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_admin_socket")]
    pub unix_socket: PathBuf,
    #[serde(default = "default_admin_port")]
    pub http_port: u16,
    #[serde(default = "default_allowed_ips")]
    pub allowed_ips: Vec<String>,
}

// Default values
fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    num_cpus::get()
}

fn default_max_requests() -> usize {
    1000
}

fn default_true() -> bool {
    true
}

fn default_opcache_memory() -> String {
    "256M".to_string()
}

fn default_max_files() -> usize {
    10000
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_log_output() -> String {
    "stdout".to_string()
}

fn default_metrics_endpoint() -> String {
    "/_metrics".to_string()
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_waf_mode() -> String {
    "off".to_string()
}

fn default_rate_limit() -> u32 {
    100
}

fn default_window_seconds() -> u64 {
    60
}

fn default_burst() -> u32 {
    10
}

fn default_admin_socket() -> PathBuf {
    PathBuf::from("/var/run/fe-php.sock")
}

fn default_admin_port() -> u16 {
    9000
}

fn default_allowed_ips() -> Vec<String> {
    vec!["127.0.0.1".to_string()]
}

impl Default for OpcacheConfig {
    fn default() -> Self {
        Self {
            enable: default_true(),
            memory_size: default_opcache_memory(),
            max_files: default_max_files(),
            validate_timestamps: false,
        }
    }
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

impl Default for WafConfig {
    fn default() -> Self {
        Self {
            enable: false,
            mode: default_waf_mode(),
            rules_path: None,
            rate_limit: RateLimitConfig::default(),
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enable: false,
            unix_socket: default_admin_socket(),
            http_port: default_admin_port(),
            allowed_ips: default_allowed_ips(),
        }
    }
}

impl Config {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        parser::parse_config(path)
    }

    pub fn validate(&self) -> Result<Vec<String>> {
        validator::validate_config(self)
    }
}
