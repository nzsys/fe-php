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
    pub geoip: GeoIpConfig,
    #[serde(default)]
    pub redis: RedisConfig,
    #[serde(default)]
    pub tracing: TracingConfig,
    #[serde(default)]
    pub load_balancing: LoadBalancingConfig,
    #[serde(default)]
    pub deployment: DeploymentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default)]
    pub enable_http2: bool,
    #[serde(default)]
    pub multi_process: bool,
    #[serde(default = "default_workers")]
    pub process_count: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub cert_path: Option<PathBuf>,
    #[serde(default)]
    pub key_path: Option<PathBuf>,
    #[serde(default)]
    pub ca_cert_path: Option<PathBuf>,
    #[serde(default)]
    pub alpn_protocols: Vec<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enable: false,
            cert_path: None,
            key_path: None,
            ca_cert_path: None,
            alpn_protocols: vec!["h2".to_string(), "http/1.1".to_string()],
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_redis_url")]
    pub url: String,
    #[serde(default = "default_redis_pool_size")]
    pub pool_size: u32,
    #[serde(default = "default_redis_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_redis_prefix")]
    pub key_prefix: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            enable: false,
            url: default_redis_url(),
            pool_size: default_redis_pool_size(),
            timeout_ms: default_redis_timeout(),
            key_prefix: default_redis_prefix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    #[serde(default = "default_service_name")]
    pub service_name: String,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enable: false,
            otlp_endpoint: default_otlp_endpoint(),
            service_name: default_service_name(),
            sample_rate: default_sample_rate(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub upstreams: Vec<UpstreamConfig>,
    #[serde(default = "default_lb_algorithm")]
    pub algorithm: String,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            enable: false,
            upstreams: Vec::new(),
            algorithm: default_lb_algorithm(),
            health_check: HealthCheckConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub name: String,
    pub url: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_health_check_path")]
    pub path: String,
    #[serde(default = "default_health_check_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_health_check_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enable: true,
            path: default_health_check_path(),
            interval_seconds: default_health_check_interval(),
            timeout_seconds: default_health_check_timeout(),
            unhealthy_threshold: default_unhealthy_threshold(),
            healthy_threshold: default_healthy_threshold(),
        }
    }
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

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_redis_pool_size() -> u32 {
    10
}

fn default_redis_timeout() -> u64 {
    5000
}

fn default_redis_prefix() -> String {
    "fe_php:".to_string()
}

fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}

fn default_service_name() -> String {
    "fe-php".to_string()
}

fn default_sample_rate() -> f64 {
    1.0
}

fn default_lb_algorithm() -> String {
    "round_robin".to_string()
}

fn default_weight() -> u32 {
    1
}

fn default_health_check_path() -> String {
    "/_health".to_string()
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_health_check_timeout() -> u64 {
    5
}

fn default_unhealthy_threshold() -> u32 {
    3
}

fn default_healthy_threshold() -> u32 {
    2
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

// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_deployment_strategy")]
    pub strategy: String,  // "ab_test" or "canary"
    #[serde(default)]
    pub variants: Vec<VariantConfig>,
    #[serde(default)]
    pub sticky_sessions: bool,
    #[serde(default)]
    pub ab_test: AbTestConfig,
    #[serde(default)]
    pub canary: CanaryConfig,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            enable: false,
            strategy: default_deployment_strategy(),
            variants: Vec::new(),
            sticky_sessions: true,
            ab_test: AbTestConfig::default(),
            canary: CanaryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantConfig {
    pub name: String,
    pub weight: u32,
    pub upstream: String,
    #[serde(default = "default_true")]
    pub metrics_tracking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbTestConfig {
    #[serde(default = "default_true")]
    pub track_conversion: bool,
    #[serde(default = "default_min_requests")]
    pub min_requests_per_variant: u64,
}

impl Default for AbTestConfig {
    fn default() -> Self {
        Self {
            track_conversion: true,
            min_requests_per_variant: default_min_requests(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    #[serde(default = "default_max_error_rate")]
    pub max_error_rate: f64,
    #[serde(default)]
    pub max_response_time_ms: Option<u64>,
    #[serde(default = "default_min_observation_period")]
    pub min_observation_period_secs: u64,
    #[serde(default = "default_min_requests")]
    pub min_requests_before_decision: u64,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            max_error_rate: default_max_error_rate(),
            max_response_time_ms: None,
            min_observation_period_secs: default_min_observation_period(),
            min_requests_before_decision: default_min_requests(),
        }
    }
}

fn default_deployment_strategy() -> String {
    "ab_test".to_string()
}

fn default_min_requests() -> u64 {
    100
}

fn default_max_error_rate() -> f64 {
    0.05  // 5%
}

fn default_min_observation_period() -> u64 {
    60  // 60 seconds
}

impl Config {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        parser::parse_config(path)
    }

    pub fn validate(&self) -> Result<Vec<String>> {
        validator::validate_config(self)
    }
}
