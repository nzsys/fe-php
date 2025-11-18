//! Default values for configuration options

use std::path::PathBuf;

// Server defaults
pub(super) fn default_host() -> String {
    "0.0.0.0".to_string()
}

pub(super) fn default_port() -> u16 {
    8080
}

pub(super) fn default_workers() -> usize {
    num_cpus::get()
}

pub(super) fn default_http_port() -> u16 {
    80
}

// PHP defaults
pub(super) fn default_max_requests() -> usize {
    1000
}

pub(super) fn default_fpm_socket() -> String {
    "127.0.0.1:9000".to_string()
}

// Opcache defaults
pub(super) fn default_true() -> bool {
    true
}

pub(super) fn default_opcache_memory() -> String {
    "256M".to_string()
}

pub(super) fn default_max_files() -> usize {
    10000
}

// Logging defaults
pub(super) fn default_log_level() -> String {
    "info".to_string()
}

pub(super) fn default_log_format() -> String {
    "json".to_string()
}

pub(super) fn default_log_output() -> String {
    "stdout".to_string()
}

// Metrics defaults
pub(super) fn default_metrics_endpoint() -> String {
    "/_metrics".to_string()
}

pub(super) fn default_metrics_port() -> u16 {
    9090
}

// Rate limit defaults
pub(super) fn default_rate_limit() -> u32 {
    100
}

pub(super) fn default_window_seconds() -> u64 {
    60
}

pub(super) fn default_burst() -> u32 {
    10
}

// Admin defaults
pub(super) fn default_admin_host() -> String {
    "127.0.0.1".to_string()
}

pub(super) fn default_admin_socket() -> PathBuf {
    PathBuf::from("/var/run/fe-php.sock")
}

pub(super) fn default_admin_port() -> u16 {
    9000
}

pub(super) fn default_allowed_ips() -> Vec<String> {
    vec!["127.0.0.1".to_string()]
}

// Redis defaults
pub(super) fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

pub(super) fn default_redis_pool_size() -> u32 {
    10
}

pub(super) fn default_redis_timeout() -> u64 {
    5000
}

pub(super) fn default_redis_prefix() -> String {
    "fe_php:".to_string()
}

// Tracing defaults
pub(super) fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}

pub(super) fn default_service_name() -> String {
    "fe-php".to_string()
}

pub(super) fn default_sample_rate() -> f64 {
    1.0
}

// Load balancing defaults
pub(super) fn default_weight() -> u32 {
    1
}

pub(super) fn default_health_check_path() -> String {
    "/_health".to_string()
}

pub(super) fn default_health_check_interval() -> u64 {
    30
}

pub(super) fn default_health_check_timeout() -> u64 {
    5
}

pub(super) fn default_unhealthy_threshold() -> u32 {
    3
}

pub(super) fn default_healthy_threshold() -> u32 {
    2
}

pub(super) fn default_failure_threshold() -> usize {
    5
}

pub(super) fn default_success_threshold() -> usize {
    2
}

pub(super) fn default_timeout_seconds() -> u64 {
    60
}

pub(super) fn default_half_open_max_requests() -> usize {
    3
}

// Deployment defaults
pub(super) fn default_min_requests() -> u64 {
    100
}

pub(super) fn default_max_error_rate() -> f64 {
    0.05  // 5%
}

pub(super) fn default_min_observation_period() -> u64 {
    60  // 60 seconds
}

// Backend defaults
pub(super) fn default_backend_type() -> String {
    "embedded".to_string()
}

pub(super) fn default_priority() -> u32 {
    50
}

pub(super) fn default_index_files() -> Vec<String> {
    vec!["index.html".to_string(), "index.htm".to_string()]
}

// Connection pool defaults
pub(super) fn default_pool_max_size() -> usize {
    20
}

pub(super) fn default_pool_max_idle_time() -> u64 {
    60
}

pub(super) fn default_pool_max_lifetime() -> u64 {
    3600
}

pub(super) fn default_pool_connect_timeout() -> u64 {
    5
}
