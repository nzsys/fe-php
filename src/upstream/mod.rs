pub mod balancer;
pub mod circuit_breaker;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{UpstreamConfig, UpstreamServer};
use balancer::{LoadBalancer, RoundRobinBalancer, WeightedBalancer};
use circuit_breaker::CircuitBreaker;

/// Upstream server with health status
#[derive(Debug, Clone)]
pub struct HealthyServer {
    pub server: UpstreamServer,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub healthy: bool,
}

impl HealthyServer {
    pub fn new(server: UpstreamServer, config: &crate::config::CircuitBreakerConfig) -> Self {
        Self {
            server,
            circuit_breaker: Arc::new(CircuitBreaker::new(
                config.failure_threshold,
                config.success_threshold,
                config.timeout_seconds,
            )),
            healthy: true,
        }
    }

    pub async fn is_available(&self) -> bool {
        self.healthy && self.circuit_breaker.is_available().await
    }

    pub async fn record_success(&self) {
        self.circuit_breaker.record_success().await;
    }

    pub async fn record_failure(&self) {
        self.circuit_breaker.record_failure().await;
    }
}

/// Upstream manager
pub struct UpstreamManager {
    servers: Arc<RwLock<Vec<HealthyServer>>>,
    balancer: Box<dyn LoadBalancer>,
}

impl UpstreamManager {
    pub fn new(config: &UpstreamConfig) -> Result<Self> {
        let servers: Vec<HealthyServer> = config
            .servers
            .iter()
            .map(|s| HealthyServer::new(s.clone(), &config.circuit_breaker))
            .collect();

        let balancer: Box<dyn LoadBalancer> = match config.load_balancing_strategy.as_str() {
            "round_robin" => Box::new(RoundRobinBalancer::new()),
            "weighted" => Box::new(WeightedBalancer::new()),
            _ => Box::new(RoundRobinBalancer::new()),
        };

        Ok(Self {
            servers: Arc::new(RwLock::new(servers)),
            balancer,
        })
    }

    /// Get next available server
    pub async fn next_server(&self) -> Option<HealthyServer> {
        let servers = self.servers.read().await;

        // Get available servers
        let mut available = Vec::new();
        for server in servers.iter() {
            if server.is_available().await {
                available.push(server.clone());
            }
        }

        if available.is_empty() {
            return None;
        }

        self.balancer.select(&available)
    }

    /// Mark a server as healthy/unhealthy
    pub async fn set_server_health(&self, host: &str, port: u16, healthy: bool) {
        let mut servers = self.servers.write().await;
        for server in servers.iter_mut() {
            if server.server.host == host && server.server.port == port {
                server.healthy = healthy;
                break;
            }
        }
    }

    /// Get all servers with their health status
    pub async fn get_servers(&self) -> Vec<HealthyServer> {
        self.servers.read().await.clone()
    }

    /// Get count of healthy servers
    pub async fn healthy_count(&self) -> usize {
        let servers = self.servers.read().await;
        let mut count = 0;
        for server in servers.iter() {
            if server.is_available().await {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CircuitBreakerConfig, UpstreamConfig};

    #[tokio::test]
    async fn test_upstream_manager() {
        let config = UpstreamConfig {
            enable: true,
            servers: vec![
                UpstreamServer {
                    host: "localhost".to_string(),
                    port: 8081,
                    weight: 1,
                },
                UpstreamServer {
                    host: "localhost".to_string(),
                    port: 8082,
                    weight: 1,
                },
            ],
            load_balancing_strategy: "round_robin".to_string(),
            circuit_breaker: CircuitBreakerConfig {
                enable: true,
                failure_threshold: 5,
                success_threshold: 2,
                timeout_seconds: 60,
            },
        };

        let manager = UpstreamManager::new(&config).unwrap();

        // Test getting servers
        let servers = manager.get_servers().await;
        assert_eq!(servers.len(), 2);

        // Test health count
        let count = manager.healthy_count().await;
        assert_eq!(count, 2);

        // Test marking server as unhealthy
        manager.set_server_health("localhost", 8081, false).await;
        let count = manager.healthy_count().await;
        assert_eq!(count, 1);
    }
}
