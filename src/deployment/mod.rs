pub mod traffic_splitter;
pub mod ab_test;
pub mod canary;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub use traffic_splitter::TrafficSplitter;
pub use ab_test::AbTestManager;
pub use canary::CanaryDeploymentManager;

use crate::config::DeploymentConfig;

/// Unified deployment manager that handles both A/B testing and canary deployments
pub struct DeploymentManager {
    traffic_splitter: Arc<TrafficSplitter>,
    ab_test: Option<Arc<RwLock<AbTestManager>>>,
    canary: Option<Arc<RwLock<CanaryDeploymentManager>>>,
}

impl DeploymentManager {
    /// Create a new deployment manager from configuration
    pub fn new(config: &DeploymentConfig) -> Result<Self> {
        let traffic_splitter = Arc::new(TrafficSplitter::new(
            config.variants.clone(),
            config.sticky_sessions,
        )?);

        let ab_test = if config.strategy == "ab_test" {
            let manager = AbTestManager::new(
                config.variants.clone(),
                config.ab_test.clone(),
            )?;
            info!("A/B testing enabled with {} variants", config.variants.len());
            Some(Arc::new(RwLock::new(manager)))
        } else {
            None
        };

        let canary = if config.strategy == "canary" {
            let manager = CanaryDeploymentManager::new(
                config.variants.clone(),
                config.canary.clone(),
            )?;
            info!("Canary deployment enabled");
            Some(Arc::new(RwLock::new(manager)))
        } else {
            None
        };

        Ok(Self {
            traffic_splitter,
            ab_test,
            canary,
        })
    }

    /// Get the traffic splitter
    pub fn traffic_splitter(&self) -> Arc<TrafficSplitter> {
        self.traffic_splitter.clone()
    }

    /// Record a request result for metrics
    pub async fn record_request(
        &self,
        variant_name: &str,
        success: bool,
        response_time_ms: u64,
    ) {
        if let Some(ref ab_test) = self.ab_test {
            ab_test.write().await.record_request(variant_name, success, response_time_ms);
        }

        if let Some(ref canary) = self.canary {
            canary.write().await.record_request(variant_name, success, response_time_ms).await;
        }
    }

    /// Get deployment statistics
    pub async fn get_stats(&self) -> DeploymentStats {
        let ab_stats = if let Some(ref ab_test) = self.ab_test {
            Some(ab_test.read().await.get_stats())
        } else {
            None
        };

        let canary_stats = if let Some(ref canary) = self.canary {
            Some(canary.read().await.get_stats())
        } else {
            None
        };

        DeploymentStats {
            ab_test: ab_stats,
            canary: canary_stats,
        }
    }

    /// Start background tasks (auto-promotion, rollback monitoring)
    pub async fn start_background_tasks(self: Arc<Self>) {
        if let Some(ref canary) = self.canary {
            let canary_clone = canary.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

                    let mut canary = canary_clone.write().await;
                    if let Err(e) = canary.check_and_update().await {
                        warn!("Canary check failed: {}", e);
                    }
                }
            });
        }

        debug!("Deployment background tasks started");
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentStats {
    pub ab_test: Option<ab_test::AbTestStats>,
    pub canary: Option<canary::CanaryDeploymentStats>,
}
