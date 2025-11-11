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
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub redis: RedisConfig,
    #[serde(default)]
    pub upstream: UpstreamConfig,
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
    #[serde(default)]
    pub geoip: GeoIpConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub cert_path: Option<PathBuf>,
    #[serde(default)]
    pub key_path: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub http2: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub database_path: Option<PathBuf>,
    #[serde(default = "default_allowed_countries")]
    pub allowed_countries: Vec<String>,
    #[serde(default = "default_blocked_countries")]
    pub blocked_countries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_redis_url")]
    pub url: String,
    #[serde(default = "default_session_ttl")]
    pub session_ttl: u64,
    #[serde(default = "default_redis_pool_size")]
    pub pool_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_servers")]
    pub servers: Vec<UpstreamServer>,
    #[serde(default = "default_lb_strategy")]
    pub load_balancing_strategy: String,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamServer {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_upstream_weight")]
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
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

fn default_allowed_countries() -> Vec<String> {
    vec![]
}

fn default_blocked_countries() -> Vec<String> {
    vec![]
}

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_session_ttl() -> u64 {
    3600 // 1 hour
}

fn default_redis_pool_size() -> u32 {
    10
}

fn default_servers() -> Vec<UpstreamServer> {
    vec![]
}

fn default_lb_strategy() -> String {
    "round_robin".to_string()
}

fn default_upstream_weight() -> u32 {
    1
}

fn default_failure_threshold() -> u32 {
    5
}

fn default_success_threshold() -> u32 {
    2
}

fn default_timeout_seconds() -> u64 {
    60
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
            geoip: GeoIpConfig::default(),
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

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enable: false,
            cert_path: None,
            key_path: None,
            http2: true,
        }
    }
}

impl Default for GeoIpConfig {
    fn default() -> Self {
        Self {
            enable: false,
            database_path: None,
            allowed_countries: default_allowed_countries(),
            blocked_countries: default_blocked_countries(),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            enable: false,
            url: default_redis_url(),
            session_ttl: default_session_ttl(),
            pool_size: default_redis_pool_size(),
        }
    }
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            enable: false,
            servers: default_servers(),
            load_balancing_strategy: default_lb_strategy(),
            circuit_breaker: CircuitBreakerConfig::default(),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enable: true,
            failure_threshold: default_failure_threshold(),
            success_threshold: default_success_threshold(),
            timeout_seconds: default_timeout_seconds(),
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
