use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{info, debug};

use crate::config::{VariantConfig, AbTestConfig};

/// A/B test manager for tracking and analyzing variant performance
pub struct AbTestManager {
    variants: Vec<VariantConfig>,
    config: AbTestConfig,
    stats: HashMap<String, Arc<VariantStats>>,
}

impl AbTestManager {
    /// Create a new A/B test manager
    pub fn new(variants: Vec<VariantConfig>, config: AbTestConfig) -> Result<Self> {
        let stats: HashMap<String, Arc<VariantStats>> = variants
            .iter()
            .map(|v| (v.name.clone(), Arc::new(VariantStats::new(v.name.clone()))))
            .collect();

        info!("A/B test initialized with {} variants", variants.len());

        Ok(Self {
            variants,
            config,
            stats,
        })
    }

    /// Record a request result
    pub fn record_request(&mut self, variant_name: &str, success: bool, response_time_ms: u64) {
        if let Some(stats) = self.stats.get(variant_name) {
            stats.total_requests.fetch_add(1, Ordering::Relaxed);

            if success {
                stats.successful_requests.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.failed_requests.fetch_add(1, Ordering::Relaxed);
            }

            // Update response time (simple moving average)
            let current_avg = stats.avg_response_time_ms.load(Ordering::Relaxed);
            let total = stats.total_requests.load(Ordering::Relaxed);

            if total > 0 {
                let new_avg = ((current_avg as u128 * (total - 1) as u128) + response_time_ms as u128) / total as u128;
                stats.avg_response_time_ms.store(new_avg as u64, Ordering::Relaxed);
            }

            if self.config.track_conversion {
                // Conversion tracking would go here
                // For now, we consider successful requests as conversions
                if success {
                    stats.conversions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// Get statistics for all variants
    pub fn get_stats(&self) -> AbTestStats {
        let variant_stats: Vec<_> = self.stats
            .values()
            .map(|s| s.snapshot())
            .collect();

        AbTestStats {
            variants: variant_stats,
            winner: self.determine_winner(),
        }
    }

    /// Determine the winning variant based on configured criteria
    fn determine_winner(&self) -> Option<String> {
        if !self.has_sufficient_data() {
            return None;
        }

        // Simple winner determination based on success rate and response time
        let mut best_variant: Option<(&String, f64)> = None;

        for (name, stats) in &self.stats {
            let snapshot = stats.snapshot();
            if snapshot.total_requests < self.config.min_requests_per_variant {
                continue;
            }

            // Calculate score: success_rate * 100 - (response_time_ms / 10)
            // This favors both high success rates and low response times
            let score = snapshot.success_rate * 100.0 - (snapshot.avg_response_time_ms as f64 / 10.0);

            if let Some((_, best_score)) = best_variant {
                if score > best_score {
                    best_variant = Some((name, score));
                }
            } else {
                best_variant = Some((name, score));
            }
        }

        best_variant.map(|(name, _)| name.clone())
    }

    /// Check if we have sufficient data for analysis
    fn has_sufficient_data(&self) -> bool {
        self.stats.values().all(|s| {
            s.total_requests.load(Ordering::Relaxed) >= self.config.min_requests_per_variant
        })
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        for stats in self.stats.values() {
            stats.reset();
        }
        info!("A/B test statistics reset");
    }
}

/// Statistics for a single variant
#[derive(Debug)]
struct VariantStats {
    name: String,
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    avg_response_time_ms: AtomicU64,
    conversions: AtomicU64,
}

impl VariantStats {
    fn new(name: String) -> Self {
        Self {
            name,
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            avg_response_time_ms: AtomicU64::new(0),
            conversions: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> VariantStatsSnapshot {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);

        VariantStatsSnapshot {
            name: self.name.clone(),
            total_requests: total,
            successful_requests: successful,
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            success_rate: if total > 0 {
                successful as f64 / total as f64
            } else {
                0.0
            },
            avg_response_time_ms: self.avg_response_time_ms.load(Ordering::Relaxed),
            conversions: self.conversions.load(Ordering::Relaxed),
            conversion_rate: if total > 0 {
                self.conversions.load(Ordering::Relaxed) as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.avg_response_time_ms.store(0, Ordering::Relaxed);
        self.conversions.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct VariantStatsSnapshot {
    pub name: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub avg_response_time_ms: u64,
    pub conversions: u64,
    pub conversion_rate: f64,
}

#[derive(Debug, Clone)]
pub struct AbTestStats {
    pub variants: Vec<VariantStatsSnapshot>,
    pub winner: Option<String>,
}
