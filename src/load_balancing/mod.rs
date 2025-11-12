use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Simple circuit breaker state
#[derive(Debug)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Simple circuit breaker implementation
struct SimpleCircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicUsize>,
    success_count: Arc<AtomicUsize>,
    failure_threshold: u32,
    success_threshold: u32,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    timeout: Duration,
}

impl SimpleCircuitBreaker {
    fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            success_count: Arc::new(AtomicUsize::new(0)),
            failure_threshold,
            success_threshold,
            last_failure_time: Arc::new(RwLock::new(None)),
            timeout,
        }
    }

    async fn is_open(&self) -> bool {
        matches!(*self.state.read().await, CircuitState::Open)
    }

    async fn record_success(&self) {
        let mut state = self.state.write().await;
        match *state {
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.success_threshold as usize {
                    *state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
            _ => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
        }
    }

    async fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= self.failure_threshold as usize {
            let mut state = self.state.write().await;
            *state = CircuitState::Open;
            *self.last_failure_time.write().await = Some(Instant::now());
        }
    }

    async fn try_reset(&self) {
        let mut state = self.state.write().await;
        if matches!(*state, CircuitState::Open) {
            let last_failure = self.last_failure_time.read().await;
            if let Some(time) = *last_failure {
                if time.elapsed() >= self.timeout {
                    *state = CircuitState::HalfOpen;
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
        }
    }
}

/// Load balancing manager with health checks and circuit breakers
pub struct LoadBalancingManager {
    upstreams: Arc<RwLock<Vec<UpstreamServer>>>,
    algorithm: LoadBalancingAlgorithm,
    round_robin_counter: Arc<AtomicUsize>,
}

impl LoadBalancingManager {
    /// Create a new load balancing manager
    pub fn new(
        upstreams: Vec<crate::config::UpstreamConfig>,
        algorithm: &str,
        circuit_breaker_config: &crate::config::CircuitBreakerConfig,
    ) -> Result<Self> {
        let algorithm = match algorithm {
            "round_robin" => LoadBalancingAlgorithm::RoundRobin,
            "weighted_round_robin" => LoadBalancingAlgorithm::WeightedRoundRobin,
            "least_connections" => LoadBalancingAlgorithm::LeastConnections,
            "random" => LoadBalancingAlgorithm::Random,
            _ => {
                warn!("Unknown load balancing algorithm '{}', using round_robin", algorithm);
                LoadBalancingAlgorithm::RoundRobin
            }
        };

        let upstream_servers = upstreams
            .into_iter()
            .map(|config| {
                UpstreamServer::new(
                    config.name,
                    config.url,
                    config.weight,
                    config.enabled,
                    circuit_breaker_config,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        debug!(
            "Initialized load balancer with {} upstreams using {:?} algorithm",
            upstream_servers.len(),
            algorithm
        );

        Ok(Self {
            upstreams: Arc::new(RwLock::new(upstream_servers)),
            algorithm,
            round_robin_counter: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Select the next upstream server based on the configured algorithm
    pub async fn select_upstream(&self) -> Result<UpstreamServer> {
        let upstreams = self.upstreams.read().await;

        // Filter healthy and enabled upstreams
        let available: Vec<&UpstreamServer> = upstreams
            .iter()
            .filter(|u| u.enabled && u.is_healthy())
            .collect();

        if available.is_empty() {
            anyhow::bail!("No healthy upstreams available");
        }

        let selected = match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                let index = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % available.len();
                available[index]
            }
            LoadBalancingAlgorithm::WeightedRoundRobin => {
                self.select_weighted(&available)
            }
            LoadBalancingAlgorithm::LeastConnections => {
                available
                    .iter()
                    .min_by_key(|u| u.active_connections.load(Ordering::Relaxed))
                    .unwrap()
            }
            LoadBalancingAlgorithm::Random => {
                let index = rand::random::<usize>() % available.len();
                available[index]
            }
        };

        Ok(selected.clone())
    }

    /// Select upstream using weighted round-robin
    fn select_weighted<'a>(&self, available: &[&'a UpstreamServer]) -> &'a UpstreamServer {
        let total_weight: u32 = available.iter().map(|u| u.weight).sum();
        let mut target = (self.round_robin_counter.fetch_add(1, Ordering::Relaxed) as u32) % total_weight;

        for upstream in available {
            if target < upstream.weight {
                return upstream;
            }
            target -= upstream.weight;
        }

        available[0]
    }

    /// Mark an upstream as healthy or unhealthy
    pub async fn update_health(&self, name: &str, healthy: bool) {
        let mut upstreams = self.upstreams.write().await;
        if let Some(upstream) = upstreams.iter_mut().find(|u| u.name == name) {
            upstream.set_healthy(healthy);
            debug!("Updated health for upstream '{}': healthy={}", name, healthy);
        }
    }

    /// Get status of all upstreams
    pub async fn get_upstreams_status(&self) -> Vec<UpstreamStatus> {
        let upstreams = self.upstreams.read().await;
        upstreams
            .iter()
            .map(|u| UpstreamStatus {
                name: u.name.clone(),
                url: u.url.clone(),
                enabled: u.enabled,
                healthy: u.is_healthy(),
                active_connections: u.active_connections.load(Ordering::Relaxed),
                total_requests: u.total_requests.load(Ordering::Relaxed),
                failed_requests: u.failed_requests.load(Ordering::Relaxed),
            })
            .collect()
    }

    /// Start health check background task
    pub async fn start_health_checks(
        &self,
        health_check_config: crate::config::HealthCheckConfig,
    ) {
        if !health_check_config.enable {
            return;
        }

        let upstreams = self.upstreams.clone();
        let interval = Duration::from_secs(health_check_config.interval_seconds);
        let timeout = Duration::from_secs(health_check_config.timeout_seconds);
        let path = health_check_config.path.clone();
        let unhealthy_threshold = health_check_config.unhealthy_threshold;
        let healthy_threshold = health_check_config.healthy_threshold;

        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("Failed to create HTTP client for health checks");

            loop {
                tokio::time::sleep(interval).await;

                let upstreams_read = upstreams.read().await;
                for upstream in upstreams_read.iter() {
                    if !upstream.enabled {
                        continue;
                    }

                    let url = format!("{}{}", upstream.url, path);
                    let result = client.get(&url).send().await;

                    let success = match result {
                        Ok(response) => response.status().is_success(),
                        Err(e) => {
                            debug!("Health check failed for {}: {}", upstream.name, e);
                            false
                        }
                    };

                    // Update health status based on thresholds
                    if success {
                        upstream.consecutive_successes.fetch_add(1, Ordering::Relaxed);
                        upstream.consecutive_failures.store(0, Ordering::Relaxed);

                        if upstream.consecutive_successes.load(Ordering::Relaxed) >= healthy_threshold as usize {
                            upstream.set_healthy(true);
                        }
                    } else {
                        upstream.consecutive_failures.fetch_add(1, Ordering::Relaxed);
                        upstream.consecutive_successes.store(0, Ordering::Relaxed);

                        if upstream.consecutive_failures.load(Ordering::Relaxed) >= unhealthy_threshold as usize {
                            upstream.set_healthy(false);
                        }
                    }
                }
            }
        });

        debug!("Started health check background task");
    }
}

#[derive(Debug, Clone)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    Random,
}

/// Represents an upstream server
#[derive(Clone)]
pub struct UpstreamServer {
    pub name: String,
    pub url: String,
    pub weight: u32,
    pub enabled: bool,
    healthy: Arc<AtomicBool>,
    circuit_breaker: Arc<SimpleCircuitBreaker>,
    active_connections: Arc<AtomicUsize>,
    total_requests: Arc<AtomicUsize>,
    failed_requests: Arc<AtomicUsize>,
    consecutive_successes: Arc<AtomicUsize>,
    consecutive_failures: Arc<AtomicUsize>,
}

impl UpstreamServer {
    pub fn new(
        name: String,
        url: String,
        weight: u32,
        enabled: bool,
        cb_config: &crate::config::CircuitBreakerConfig,
    ) -> Result<Self> {
        let circuit_breaker = SimpleCircuitBreaker::new(
            cb_config.failure_threshold,
            cb_config.success_threshold,
            Duration::from_secs(cb_config.timeout_seconds),
        );

        Ok(Self {
            name,
            url,
            weight,
            enabled,
            healthy: Arc::new(AtomicBool::new(true)),
            circuit_breaker: Arc::new(circuit_breaker),
            active_connections: Arc::new(AtomicUsize::new(0)),
            total_requests: Arc::new(AtomicUsize::new(0)),
            failed_requests: Arc::new(AtomicUsize::new(0)),
            consecutive_successes: Arc::new(AtomicUsize::new(0)),
            consecutive_failures: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    pub fn set_healthy(&self, healthy: bool) {
        self.healthy.store(healthy, Ordering::Relaxed);
    }

    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_request(&self, success: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Execute a request through the circuit breaker
    pub async fn call_with_circuit_breaker<F, Fut, T>(
        &self,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Try to reset if timeout has elapsed
        self.circuit_breaker.try_reset().await;

        // Check if circuit is open
        if self.circuit_breaker.is_open().await {
            self.record_request(false);
            anyhow::bail!("Circuit breaker is open for upstream '{}'", self.name);
        }

        // Execute the function
        match f().await {
            Ok(result) => {
                self.circuit_breaker.record_success().await;
                self.record_request(true);
                Ok(result)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                self.record_request(false);
                Err(e)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpstreamStatus {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub healthy: bool,
    pub active_connections: usize,
    pub total_requests: usize,
    pub failed_requests: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_round_robin_selection() {
        // This test would create test upstreams and verify round-robin behavior
    }
}
