use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

/// Circuit breaker state
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,     // Normal operation
    Open,       // Failing, reject requests
    HalfOpen,   // Testing if service recovered
}

/// Circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
}

#[derive(Debug)]
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    last_failure_time: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout_seconds: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitBreakerState {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                failure_threshold,
                success_threshold,
                timeout: Duration::from_secs(timeout_seconds),
                last_failure_time: None,
            })),
        }
    }

    /// Check if circuit breaker allows requests
    pub async fn is_available(&self) -> bool {
        let mut state = self.state.write().await;

        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() >= state.timeout {
                        // Transition to half-open state
                        state.state = CircuitState::HalfOpen;
                        state.success_count = 0;
                        state.failure_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful request
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;

        match state.state {
            CircuitState::Closed => {
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= state.success_threshold {
                    // Transition to closed state
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                    state.last_failure_time = None;
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let mut state = self.state.write().await;

        state.failure_count += 1;
        state.last_failure_time = Some(Instant::now());

        match state.state {
            CircuitState::Closed => {
                if state.failure_count >= state.failure_threshold {
                    // Transition to open state
                    state.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Immediately transition back to open state
                state.state = CircuitState::Open;
                state.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.state.clone()
    }

    /// Get failure count
    pub async fn get_failure_count(&self) -> u32 {
        self.state.read().await.failure_count
    }

    /// Get success count
    pub async fn get_success_count(&self) -> u32 {
        self.state.read().await.success_count
    }

    /// Reset circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure_time = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let cb = CircuitBreaker::new(3, 2, 5);

        // Initially closed
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        assert!(cb.is_available().await);

        // Record failures
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        assert_eq!(cb.get_failure_count().await, 1);

        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        assert_eq!(cb.get_failure_count().await, 2);

        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        assert_eq!(cb.get_failure_count().await, 3);

        // Should not be available when open
        assert!(!cb.is_available().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_open_to_half_open() {
        let cb = CircuitBreaker::new(2, 2, 1);

        // Trigger open state
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should transition to half-open
        assert!(cb.is_available().await);
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed() {
        let cb = CircuitBreaker::new(2, 2, 1);

        // Trigger open state
        cb.record_failure().await;
        cb.record_failure().await;

        // Wait and transition to half-open
        tokio::time::sleep(Duration::from_secs(2)).await;
        cb.is_available().await;

        // Record successful requests
        cb.record_success().await;
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);

        cb.record_success().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_open() {
        let cb = CircuitBreaker::new(2, 2, 1);

        // Trigger open state
        cb.record_failure().await;
        cb.record_failure().await;

        // Wait and transition to half-open
        tokio::time::sleep(Duration::from_secs(2)).await;
        cb.is_available().await;

        // Record a failure in half-open state
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(2, 2, 5);

        // Trigger open state
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);

        // Reset
        cb.reset().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        assert_eq!(cb.get_failure_count().await, 0);
        assert!(cb.is_available().await);
    }
}
