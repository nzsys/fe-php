use super::*;

pub struct MetricsCollector;

impl MetricsCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn record_request(&self, method: &str, status: u16, duration_secs: f64) {
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[method, &status.to_string()])
            .inc();

        HTTP_RESPONSE_TIME
            .with_label_values(&[method, &status.to_string()])
            .observe(duration_secs);
    }

    pub fn inc_active_connections(&self) {
        ACTIVE_CONNECTIONS.with_label_values(&["http"]).inc();
    }

    pub fn dec_active_connections(&self) {
        ACTIVE_CONNECTIONS.with_label_values(&["http"]).dec();
    }

    pub fn set_php_workers(&self, status: &str, count: i64) {
        PHP_WORKERS.with_label_values(&[status]).set(count);
    }

    pub fn set_php_memory(&self, worker_id: usize, bytes: i64) {
        PHP_MEMORY_BYTES
            .with_label_values(&[&worker_id.to_string()])
            .set(bytes);
    }

    pub fn inc_php_requests_handled(&self, worker_id: usize) {
        PHP_REQUESTS_HANDLED
            .with_label_values(&[&worker_id.to_string()])
            .inc();
    }

    pub fn set_opcache_hit_rate(&self, rate: i64) {
        OPCACHE_HIT_RATE.with_label_values(&[]).set(rate);
    }

    pub fn set_opcache_memory_usage(&self, bytes: i64) {
        OPCACHE_MEMORY_USAGE.with_label_values(&[]).set(bytes);
    }

    pub fn set_opcache_cached_scripts(&self, count: i64) {
        OPCACHE_CACHED_SCRIPTS.with_label_values(&[]).set(count);
    }

    pub fn inc_waf_blocked(&self, rule_id: &str) {
        WAF_REQUESTS_BLOCKED.with_label_values(&[rule_id]).inc();
    }

    pub fn inc_rate_limit_triggered(&self) {
        WAF_RATE_LIMIT_TRIGGERED.with_label_values(&[]).inc();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
