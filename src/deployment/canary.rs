use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn, error};

use crate::config::{VariantConfig, CanaryConfig};

pub struct CanaryDeploymentManager {
    variants: Vec<VariantConfig>,
    config: CanaryConfig,
    stats: HashMap<String, Arc<CanaryStats>>,
    current_phase: CanaryPhase,
    phase_start_time: Instant,
}

impl CanaryDeploymentManager {
    pub fn new(variants: Vec<VariantConfig>, config: CanaryConfig) -> Result<Self> {
        if variants.len() != 2 {
            anyhow::bail!("Canary deployment requires exactly 2 variants (stable and canary)");
        }

        let stats: HashMap<String, Arc<CanaryStats>> = variants
            .iter()
            .map(|v| (v.name.clone(), Arc::new(CanaryStats::new())))
            .collect();

        info!("Canary deployment initialized");

        Ok(Self {
            variants,
            config,
            stats,
            current_phase: CanaryPhase::Initial,
            phase_start_time: Instant::now(),
        })
    }

    pub async fn record_request(&mut self, variant_name: &str, success: bool, response_time_ms: u64) {
        if let Some(stats) = self.stats.get(variant_name) {
            stats.record_request(success, response_time_ms);
        }
    }

    pub async fn check_and_update(&mut self) -> Result<()> {
        let elapsed = self.phase_start_time.elapsed();

        if elapsed < Duration::from_secs(self.config.min_observation_period_secs) {
            return Ok(());
        }

        let canary_variant = self.variants.iter()
            .find(|v| v.name.contains("canary") || v.name.contains("new"))
            .ok_or_else(|| anyhow::anyhow!("Canary variant not found"))?;

        let canary_stats = self.stats.get(&canary_variant.name)
            .ok_or_else(|| anyhow::anyhow!("Canary stats not found"))?;

        let snapshot = canary_stats.snapshot();

        if self.should_rollback(&snapshot) {
            self.rollback().await?;
            return Ok(());
        }

        if self.should_promote(&snapshot) {
            self.promote().await?;
        }

        Ok(())
    }

    fn should_rollback(&self, stats: &CanaryStatsSnapshot) -> bool {
        if stats.total_requests < self.config.min_requests_before_decision {
            return false;
        }

        if stats.error_rate > self.config.max_error_rate {
            warn!(
                "Canary error rate {:.2}% exceeds threshold {:.2}%",
                stats.error_rate * 100.0,
                self.config.max_error_rate * 100.0
            );
            return true;
        }

        if let Some(max_response_time) = self.config.max_response_time_ms {
            if stats.avg_response_time_ms > max_response_time {
                warn!(
                    "Canary response time {}ms exceeds threshold {}ms",
                    stats.avg_response_time_ms,
                    max_response_time
                );
                return true;
            }
        }

        false
    }

    fn should_promote(&self, stats: &CanaryStatsSnapshot) -> bool {
        if stats.total_requests < self.config.min_requests_before_decision {
            return false;
        }

        stats.error_rate <= self.config.max_error_rate
    }

    async fn rollback(&mut self) -> Result<()> {
        error!("Canary deployment FAILED - Rolling back to stable");

        for variant in &mut self.variants {
            if variant.name.contains("canary") || variant.name.contains("new") {
                variant.weight = 0;
            } else {
                variant.weight = 100;
            }
        }

        self.current_phase = CanaryPhase::RolledBack;
        info!("Rollback completed");

        Ok(())
    }

    async fn promote(&mut self) -> Result<()> {
        match self.current_phase {
            CanaryPhase::Initial => {
                self.set_weights(95, 5);
                self.current_phase = CanaryPhase::Phase5;
                info!("Canary promoted to 5%");
            }
            CanaryPhase::Phase5 => {
                self.set_weights(75, 25);
                self.current_phase = CanaryPhase::Phase25;
                info!("Canary promoted to 25%");
            }
            CanaryPhase::Phase25 => {
                self.set_weights(50, 50);
                self.current_phase = CanaryPhase::Phase50;
                info!("Canary promoted to 50%");
            }
            CanaryPhase::Phase50 => {
                self.set_weights(0, 100);
                self.current_phase = CanaryPhase::Completed;
                info!("Canary deployment COMPLETED - Canary is now stable");
            }
            _ => {}
        }

        self.phase_start_time = Instant::now();
        self.reset_stats();

        Ok(())
    }

    fn set_weights(&mut self, stable_weight: u32, canary_weight: u32) {
        for variant in &mut self.variants {
            if variant.name.contains("canary") || variant.name.contains("new") {
                variant.weight = canary_weight;
            } else {
                variant.weight = stable_weight;
            }
        }
    }

    fn reset_stats(&mut self) {
        for stats in self.stats.values() {
            stats.reset();
        }
    }

    pub fn get_stats(&self) -> CanaryDeploymentStats {
        let variant_stats: Vec<_> = self.variants
            .iter()
            .map(|v| {
                let stats = self.stats.get(&v.name).map(|s| s.snapshot());
                (v.name.clone(), v.weight, stats)
            })
            .collect();

        CanaryDeploymentStats {
            current_phase: format!("{:?}", self.current_phase),
            phase_duration_secs: self.phase_start_time.elapsed().as_secs(),
            variants: variant_stats,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CanaryPhase {
    Initial,
    Phase5,
    Phase25,
    Phase50,
    Completed,
    RolledBack,
}

#[derive(Debug)]
struct CanaryStats {
    total_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_response_time_ms: AtomicU64,
}

impl CanaryStats {
    fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_response_time_ms: AtomicU64::new(0),
        }
    }

    fn record_request(&self, success: bool, response_time_ms: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.total_response_time_ms.fetch_add(response_time_ms, Ordering::Relaxed);
    }

    fn snapshot(&self) -> CanaryStatsSnapshot {
        let total = self.total_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);
        let total_time = self.total_response_time_ms.load(Ordering::Relaxed);

        CanaryStatsSnapshot {
            total_requests: total,
            failed_requests: failed,
            error_rate: if total > 0 { failed as f64 / total as f64 } else { 0.0 },
            avg_response_time_ms: if total > 0 { total_time / total } else { 0 },
        }
    }

    fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.total_response_time_ms.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct CanaryStatsSnapshot {
    pub total_requests: u64,
    pub failed_requests: u64,
    pub error_rate: f64,
    pub avg_response_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct CanaryDeploymentStats {
    pub current_phase: String,
    pub phase_duration_secs: u64,
    pub variants: Vec<(String, u32, Option<CanaryStatsSnapshot>)>,
}
