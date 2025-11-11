pub mod collector;
pub mod exporter;

pub use collector::MetricsCollector;
pub use exporter::export_metrics;

use lazy_static::lazy_static;
use prometheus::{
    IntCounterVec, HistogramVec, IntGaugeVec, Opts, Registry, register_int_counter_vec,
    register_histogram_vec, register_int_gauge_vec,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new("fe_php_requests_total", "Total HTTP requests"),
        &["method", "status"]
    ).unwrap();

    pub static ref HTTP_RESPONSE_TIME: HistogramVec = register_histogram_vec!(
        "fe_php_response_time_seconds",
        "HTTP response time in seconds",
        &["method", "status"]
    ).unwrap();

    pub static ref ACTIVE_CONNECTIONS: IntGaugeVec = register_int_gauge_vec!(
        Opts::new("fe_php_active_connections", "Active connections"),
        &["type"]
    ).unwrap();

    pub static ref PHP_WORKERS: IntGaugeVec = register_int_gauge_vec!(
        Opts::new("fe_php_php_workers", "PHP worker status"),
        &["status"]
    ).unwrap();

    pub static ref PHP_MEMORY_BYTES: IntGaugeVec = register_int_gauge_vec!(
        Opts::new("fe_php_php_memory_bytes", "PHP worker memory usage"),
        &["worker_id"]
    ).unwrap();

    pub static ref PHP_REQUESTS_HANDLED: IntCounterVec = register_int_counter_vec!(
        Opts::new("fe_php_php_requests_handled", "PHP requests handled by worker"),
        &["worker_id"]
    ).unwrap();

    pub static ref OPCACHE_HIT_RATE: IntGaugeVec = register_int_gauge_vec!(
        Opts::new("fe_php_opcache_hit_rate", "OPcache hit rate percentage"),
        &[]
    ).unwrap();

    pub static ref OPCACHE_MEMORY_USAGE: IntGaugeVec = register_int_gauge_vec!(
        Opts::new("fe_php_opcache_memory_usage_bytes", "OPcache memory usage"),
        &[]
    ).unwrap();

    pub static ref OPCACHE_CACHED_SCRIPTS: IntGaugeVec = register_int_gauge_vec!(
        Opts::new("fe_php_opcache_cached_scripts", "OPcache cached scripts"),
        &[]
    ).unwrap();

    pub static ref WAF_REQUESTS_BLOCKED: IntCounterVec = register_int_counter_vec!(
        Opts::new("fe_php_waf_requests_blocked", "WAF blocked requests"),
        &["rule_id"]
    ).unwrap();

    pub static ref WAF_RATE_LIMIT_TRIGGERED: IntCounterVec = register_int_counter_vec!(
        Opts::new("fe_php_waf_rate_limit_triggered", "WAF rate limit triggered"),
        &[]
    ).unwrap();
}

pub fn init_metrics() {
    // Initialize metrics (they're already registered via lazy_static)
}
