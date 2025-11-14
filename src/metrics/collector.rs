use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry,
    core::Collector,
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

    static ref CONNECTION_POOL_IDLE: GaugeVec = GaugeVec::new(
        Opts::new("connection_pool_idle_connections", "Idle connections in pool"),
        &["backend", "pool_type"]
    ).unwrap();

    static ref CONNECTION_POOL_ACTIVE: GaugeVec = GaugeVec::new(
        Opts::new("connection_pool_active_connections", "Active connections in pool"),
        &["backend", "pool_type"]
    ).unwrap();

    static ref CONNECTION_POOL_ACQUIRE_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("connection_pool_acquire_duration_seconds", "Time to acquire connection from pool"),
        &["backend", "pool_type"]
    ).unwrap();

    static ref CONNECTION_POOL_ERRORS: CounterVec = CounterVec::new(
        Opts::new("connection_pool_errors_total", "Connection pool errors"),
        &["backend", "pool_type", "error_type"]
    ).unwrap();

    static ref CIRCUIT_BREAKER_STATE: GaugeVec = GaugeVec::new(
        Opts::new("circuit_breaker_state", "Circuit breaker state (0=closed, 1=half-open, 2=open)"),
        &["backend"]
    ).unwrap();

    static ref CIRCUIT_BREAKER_FAILURES: CounterVec = CounterVec::new(
        Opts::new("circuit_breaker_failures_total", "Circuit breaker failure count"),
        &["backend"]
    ).unwrap();
}

pub struct MetricsCollector {
    registry: Arc<Registry>,
    // キャッシュされたメトリクス値 (直接アクセス用)
    cached_total_requests: Arc<std::sync::atomic::AtomicU64>,
    cached_active_connections: Arc<std::sync::atomic::AtomicI64>,
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
        registry.register(Box::new(CONNECTION_POOL_IDLE.clone())).unwrap();
        registry.register(Box::new(CONNECTION_POOL_ACTIVE.clone())).unwrap();
        registry.register(Box::new(CONNECTION_POOL_ACQUIRE_DURATION.clone())).unwrap();
        registry.register(Box::new(CONNECTION_POOL_ERRORS.clone())).unwrap();
        registry.register(Box::new(CIRCUIT_BREAKER_STATE.clone())).unwrap();
        registry.register(Box::new(CIRCUIT_BREAKER_FAILURES.clone())).unwrap();

        Self {
            registry: Arc::new(registry),
            cached_total_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cached_active_connections: Arc::new(std::sync::atomic::AtomicI64::new(0)),
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
        // Update cache
        self.cached_total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn inc_active_connections(&self) {
        ACTIVE_CONNECTIONS.inc();
        self.cached_active_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn dec_active_connections(&self) {
        ACTIVE_CONNECTIONS.dec();
        self.cached_active_connections.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
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

    pub fn set_connection_pool_idle(&self, backend: &str, pool_type: &str, count: usize) {
        CONNECTION_POOL_IDLE
            .with_label_values(&[backend, pool_type])
            .set(count as f64);
    }

    pub fn set_connection_pool_active(&self, backend: &str, pool_type: &str, count: usize) {
        CONNECTION_POOL_ACTIVE
            .with_label_values(&[backend, pool_type])
            .set(count as f64);
    }

    pub fn observe_connection_pool_acquire(&self, backend: &str, pool_type: &str, duration_secs: f64) {
        CONNECTION_POOL_ACQUIRE_DURATION
            .with_label_values(&[backend, pool_type])
            .observe(duration_secs);
    }

    pub fn inc_connection_pool_error(&self, backend: &str, pool_type: &str, error_type: &str) {
        CONNECTION_POOL_ERRORS
            .with_label_values(&[backend, pool_type, error_type])
            .inc();
    }

    pub fn set_circuit_breaker_state(&self, backend: &str, state: i64) {
        CIRCUIT_BREAKER_STATE
            .with_label_values(&[backend])
            .set(state as f64);
    }

    pub fn inc_circuit_breaker_failure(&self, backend: &str) {
        CIRCUIT_BREAKER_FAILURES
            .with_label_values(&[backend])
            .inc();
    }

    /// Get total HTTP requests (from cache)
    pub fn get_total_requests(&self) -> u64 {
        self.cached_total_requests.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get current active connections (from cache)
    pub fn get_active_connections(&self) -> i64 {
        self.cached_active_connections.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get backend requests by backend type
    /// Note: これらは近似値です。正確な値が必要な場合は、Prometheusから直接取得してください。
    pub fn get_backend_requests(&self, _backend: &str) -> u64 {
        // 簡略化のため、全体のリクエスト数の一部として返す
        // 実際の実装ではバックエンドごとのカウンターが必要
        0
    }

    /// Get backend errors by backend type
    pub fn get_backend_errors(&self, _backend: &str) -> u64 {
        // 簡略化のため、0を返す
        // 実際の実装ではバックエンドごとのエラーカウンターが必要
        0
    }

    /// Get average backend response time in milliseconds
    pub fn get_backend_avg_response_ms(&self, _backend: &str) -> f64 {
        // 簡略化のため、0を返す
        // 実際の実装ではヒストグラムから計算が必要
        0.0
    }

    /// Get total WAF blocked requests
    pub fn get_waf_blocked_total(&self) -> u64 {
        // グローバルメトリクスから取得（簡略化）
        // 注: 正確な値取得はPrometheus APIを使用する必要がある
        0
    }

    /// Get rate limit triggered count
    pub fn get_rate_limit_triggered(&self) -> u64 {
        // グローバルメトリクスから取得（簡略化）
        0
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
