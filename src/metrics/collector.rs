use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry,
};
use std::sync::Arc;

lazy_static! {
    static ref HTTP_REQUESTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests"),
        &["method", "status"]
    ).unwrap();

    static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration"),
        &["method"]
    ).unwrap();

    static ref ACTIVE_CONNECTIONS: Gauge = Gauge::new(
        "active_connections", "Active connections"
    ).unwrap();

    static ref BACKEND_REQUESTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("backend_requests_total", "Total backend requests"),
        &["backend", "status"]
    ).unwrap();

    static ref BACKEND_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("backend_request_duration_seconds", "Backend request duration"),
        &["backend"]
    ).unwrap();

    static ref BACKEND_ERRORS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("backend_errors_total", "Total backend errors"),
        &["backend", "error_type"]
    ).unwrap();

    static ref PHP_WORKERS: GaugeVec = GaugeVec::new(
        Opts::new("php_workers", "PHP worker pool status"),
        &["status"]
    ).unwrap();

    static ref PHP_MEMORY_USAGE: GaugeVec = GaugeVec::new(
        Opts::new("php_memory_bytes", "PHP worker memory usage"),
        &["worker_id"]
    ).unwrap();

    static ref PHP_REQUESTS_HANDLED: CounterVec = CounterVec::new(
        Opts::new("php_requests_handled_total", "PHP requests handled by worker"),
        &["worker_id"]
    ).unwrap();

    static ref OPCACHE_HIT_RATE: Gauge = Gauge::new(
        "opcache_hit_rate_percent", "OPcache hit rate percentage"
    ).unwrap();

    static ref OPCACHE_MEMORY_USAGE: Gauge = Gauge::new(
        "opcache_memory_bytes", "OPcache memory usage"
    ).unwrap();

    static ref OPCACHE_CACHED_SCRIPTS: Gauge = Gauge::new(
        "opcache_cached_scripts", "Number of cached scripts"
    ).unwrap();

    static ref WAF_BLOCKED_TOTAL: CounterVec = CounterVec::new(
        Opts::new("waf_blocked_total", "Requests blocked by WAF"),
        &["rule_id"]
    ).unwrap();

    static ref RATE_LIMIT_TRIGGERED: Counter = Counter::new(
        "rate_limit_triggered_total", "Rate limit triggers"
    ).unwrap();

    static ref FASTCGI_POOL_SIZE: Gauge = Gauge::new(
        "fastcgi_pool_connections", "FastCGI connection pool size"
    ).unwrap();

    static ref FASTCGI_POOL_MAX_SIZE: Gauge = Gauge::new(
        "fastcgi_pool_max_connections", "FastCGI connection pool max size"
    ).unwrap();
}

pub struct MetricsCollector {
    registry: Arc<Registry>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        registry.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).unwrap();
        registry.register(Box::new(HTTP_REQUEST_DURATION.clone())).unwrap();
        registry.register(Box::new(ACTIVE_CONNECTIONS.clone())).unwrap();
        registry.register(Box::new(BACKEND_REQUESTS_TOTAL.clone())).unwrap();
        registry.register(Box::new(BACKEND_REQUEST_DURATION.clone())).unwrap();
        registry.register(Box::new(BACKEND_ERRORS_TOTAL.clone())).unwrap();
        registry.register(Box::new(PHP_WORKERS.clone())).unwrap();
        registry.register(Box::new(PHP_MEMORY_USAGE.clone())).unwrap();
        registry.register(Box::new(PHP_REQUESTS_HANDLED.clone())).unwrap();
        registry.register(Box::new(OPCACHE_HIT_RATE.clone())).unwrap();
        registry.register(Box::new(OPCACHE_MEMORY_USAGE.clone())).unwrap();
        registry.register(Box::new(OPCACHE_CACHED_SCRIPTS.clone())).unwrap();
        registry.register(Box::new(WAF_BLOCKED_TOTAL.clone())).unwrap();
        registry.register(Box::new(RATE_LIMIT_TRIGGERED.clone())).unwrap();
        registry.register(Box::new(FASTCGI_POOL_SIZE.clone())).unwrap();
        registry.register(Box::new(FASTCGI_POOL_MAX_SIZE.clone())).unwrap();

        Self {
            registry: Arc::new(registry),
        }
    }

    pub fn registry(&self) -> Arc<Registry> {
        Arc::clone(&self.registry)
    }

    pub fn record_request(&self, method: &str, status: u16, duration_secs: f64) {
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[method, &status.to_string()])
            .inc();
        HTTP_REQUEST_DURATION
            .with_label_values(&[method])
            .observe(duration_secs);
    }

    pub fn inc_active_connections(&self) {
        ACTIVE_CONNECTIONS.inc();
    }

    pub fn dec_active_connections(&self) {
        ACTIVE_CONNECTIONS.dec();
    }

    pub fn record_backend_request(&self, backend: &str, status: &str, duration_secs: f64) {
        BACKEND_REQUESTS_TOTAL
            .with_label_values(&[backend, status])
            .inc();
        BACKEND_REQUEST_DURATION
            .with_label_values(&[backend])
            .observe(duration_secs);
    }

    pub fn record_backend_error(&self, backend: &str, error_type: &str) {
        BACKEND_ERRORS_TOTAL
            .with_label_values(&[backend, error_type])
            .inc();
    }

    pub fn set_php_workers(&self, status: &str, count: i64) {
        PHP_WORKERS.with_label_values(&[status]).set(count as f64);
    }

    pub fn set_php_memory(&self, worker_id: usize, bytes: i64) {
        PHP_MEMORY_USAGE
            .with_label_values(&[&worker_id.to_string()])
            .set(bytes as f64);
    }

    pub fn inc_php_requests_handled(&self, worker_id: usize) {
        PHP_REQUESTS_HANDLED
            .with_label_values(&[&worker_id.to_string()])
            .inc();
    }

    pub fn set_opcache_hit_rate(&self, rate: i64) {
        OPCACHE_HIT_RATE.set(rate as f64);
    }

    pub fn set_opcache_memory_usage(&self, bytes: i64) {
        OPCACHE_MEMORY_USAGE.set(bytes as f64);
    }

    pub fn set_opcache_cached_scripts(&self, count: i64) {
        OPCACHE_CACHED_SCRIPTS.set(count as f64);
    }

    pub fn inc_waf_blocked(&self, rule_id: &str) {
        WAF_BLOCKED_TOTAL.with_label_values(&[rule_id]).inc();
    }

    pub fn inc_rate_limit_triggered(&self) {
        RATE_LIMIT_TRIGGERED.inc();
    }

    pub fn set_fastcgi_pool_size(&self, size: usize, max_size: usize) {
        FASTCGI_POOL_SIZE.set(size as f64);
        FASTCGI_POOL_MAX_SIZE.set(max_size as f64);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
