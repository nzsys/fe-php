use super::HealthyServer;
use std::sync::atomic::{AtomicUsize, Ordering};
use rand::Rng;

/// Load balancing strategy trait
pub trait LoadBalancer: Send + Sync {
    fn select(&self, servers: &[HealthyServer]) -> Option<HealthyServer>;
}

/// Round-robin load balancer
pub struct RoundRobinBalancer {
    counter: AtomicUsize,
}

impl RoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl LoadBalancer for RoundRobinBalancer {
    fn select(&self, servers: &[HealthyServer]) -> Option<HealthyServer> {
        if servers.is_empty() {
            return None;
        }

        let index = self.counter.fetch_add(1, Ordering::Relaxed) % servers.len();
        Some(servers[index].clone())
    }
}

impl Default for RoundRobinBalancer {
    fn default() -> Self {
        Self::new()
    }
}

/// Weighted load balancer
pub struct WeightedBalancer;

impl WeightedBalancer {
    pub fn new() -> Self {
        Self
    }
}

impl LoadBalancer for WeightedBalancer {
    fn select(&self, servers: &[HealthyServer]) -> Option<HealthyServer> {
        if servers.is_empty() {
            return None;
        }

        // Calculate total weight
        let total_weight: u32 = servers.iter().map(|s| s.server.weight).sum();

        if total_weight == 0 {
            // If all weights are 0, fall back to random selection
            let index = rand::thread_rng().gen_range(0..servers.len());
            return Some(servers[index].clone());
        }

        // Select a random number between 0 and total_weight
        let mut random = rand::thread_rng().gen_range(0..total_weight);

        // Find the server based on weight
        for server in servers {
            if random < server.server.weight {
                return Some(server.clone());
            }
            random -= server.server.weight;
        }

        // Fallback to first server (should not happen)
        Some(servers[0].clone())
    }
}

impl Default for WeightedBalancer {
    fn default() -> Self {
        Self::new()
    }
}

/// Least connections load balancer
/// Note: This is a placeholder implementation. In a real system,
/// you would track active connections per server.
pub struct LeastConnectionsBalancer;

impl LeastConnectionsBalancer {
    pub fn new() -> Self {
        Self
    }
}

impl LoadBalancer for LeastConnectionsBalancer {
    fn select(&self, servers: &[HealthyServer]) -> Option<HealthyServer> {
        if servers.is_empty() {
            return None;
        }

        // Placeholder: Just return the first server
        // In a real implementation, track connections per server
        Some(servers[0].clone())
    }
}

impl Default for LeastConnectionsBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CircuitBreakerConfig, UpstreamServer};

    fn create_test_servers() -> Vec<HealthyServer> {
        let circuit_breaker_config = CircuitBreakerConfig {
            enable: true,
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 60,
        };

        vec![
            HealthyServer::new(
                UpstreamServer {
                    host: "server1".to_string(),
                    port: 8080,
                    weight: 1,
                },
                &circuit_breaker_config,
            ),
            HealthyServer::new(
                UpstreamServer {
                    host: "server2".to_string(),
                    port: 8080,
                    weight: 2,
                },
                &circuit_breaker_config,
            ),
            HealthyServer::new(
                UpstreamServer {
                    host: "server3".to_string(),
                    port: 8080,
                    weight: 1,
                },
                &circuit_breaker_config,
            ),
        ]
    }

    #[test]
    fn test_round_robin_balancer() {
        let balancer = RoundRobinBalancer::new();
        let servers = create_test_servers();

        // First selection should be server1
        let selected = balancer.select(&servers).unwrap();
        assert_eq!(selected.server.host, "server1");

        // Second selection should be server2
        let selected = balancer.select(&servers).unwrap();
        assert_eq!(selected.server.host, "server2");

        // Third selection should be server3
        let selected = balancer.select(&servers).unwrap();
        assert_eq!(selected.server.host, "server3");

        // Fourth selection should wrap back to server1
        let selected = balancer.select(&servers).unwrap();
        assert_eq!(selected.server.host, "server1");
    }

    #[test]
    fn test_weighted_balancer() {
        let balancer = WeightedBalancer::new();
        let servers = create_test_servers();

        // Run multiple selections and verify weighted distribution
        let mut counts = std::collections::HashMap::new();
        for _ in 0..1000 {
            let selected = balancer.select(&servers).unwrap();
            *counts.entry(selected.server.host.clone()).or_insert(0) += 1;
        }

        // Server2 should have roughly twice as many selections as server1 and server3
        // (allowing for randomness variance)
        let server1_count = counts.get("server1").unwrap_or(&0);
        let server2_count = counts.get("server2").unwrap_or(&0);
        let server3_count = counts.get("server3").unwrap_or(&0);

        // Server2 should have more selections than server1 and server3
        assert!(server2_count > server1_count);
        assert!(server2_count > server3_count);
    }

    #[test]
    fn test_empty_servers() {
        let balancer = RoundRobinBalancer::new();
        let servers: Vec<HealthyServer> = vec![];

        assert!(balancer.select(&servers).is_none());
    }
}
