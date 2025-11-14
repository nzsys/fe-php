use anyhow::Result;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::debug;

use crate::config::VariantConfig;

pub struct TrafficSplitter {
    variants: Vec<VariantConfig>,
    total_weight: u32,
    round_robin_counter: Arc<AtomicUsize>,
    sticky_sessions: bool,
    // User -> Variant mapping for sticky sessions
    user_assignments: Arc<parking_lot::RwLock<HashMap<String, String>>>,
}

impl TrafficSplitter {
    pub fn new(variants: Vec<VariantConfig>, sticky_sessions: bool) -> Result<Self> {
        let total_weight: u32 = variants.iter().map(|v| v.weight).sum();

        if total_weight == 0 {
            anyhow::bail!("Total weight of variants must be greater than 0");
        }

        Ok(Self {
            variants,
            total_weight,
            round_robin_counter: Arc::new(AtomicUsize::new(0)),
            sticky_sessions,
            user_assignments: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        })
    }

    pub fn select_variant(&self, user_id: Option<&str>, ip_addr: Option<IpAddr>) -> &VariantConfig {
        if self.sticky_sessions {
            let identifier: Option<String> = user_id
                .map(|s| s.to_string())
                .or_else(|| ip_addr.map(|ip| ip.to_string()));

            if let Some(id) = identifier {
                {
                    let assignments = self.user_assignments.read();
                    if let Some(variant_name) = assignments.get(&id) {
                        if let Some(variant) = self.variants.iter().find(|v| &v.name == variant_name) {
                            debug!("Sticky session: {} -> {}", id, variant_name);
                            return variant;
                        }
                    }
                }

                let variant = self.select_by_weight();
                self.user_assignments.write().insert(id, variant.name.clone());
                return variant;
            }
        }

        self.select_by_weight()
    }

    fn select_by_weight(&self) -> &VariantConfig {
        let counter = self.round_robin_counter.fetch_add(1, Ordering::Relaxed);
        let mut target = (counter as u32) % self.total_weight;

        for variant in &self.variants {
            if target < variant.weight {
                return variant;
            }
            target -= variant.weight;
        }

        &self.variants[0]
    }

    pub fn update_weights(&mut self, new_weights: HashMap<String, u32>) {
        for variant in &mut self.variants {
            if let Some(&new_weight) = new_weights.get(&variant.name) {
                variant.weight = new_weight;
            }
        }
        self.total_weight = self.variants.iter().map(|v| v.weight).sum();
    }

    pub fn get_weights(&self) -> HashMap<String, u32> {
        self.variants
            .iter()
            .map(|v| (v.name.clone(), v.weight))
            .collect()
    }

    pub fn clear_sticky_sessions(&self) {
        self.user_assignments.write().clear();
    }

    pub fn sticky_session_count(&self) -> usize {
        self.user_assignments.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_splitter_weighted_distribution() {
        let variants = vec![
            VariantConfig {
                name: "v1".to_string(),
                weight: 70,
                upstream: "http://v1:8080".to_string(),
                metrics_tracking: true,
            },
            VariantConfig {
                name: "v2".to_string(),
                weight: 30,
                upstream: "http://v2:8080".to_string(),
                metrics_tracking: true,
            },
        ];

        let splitter = TrafficSplitter::new(variants, false).unwrap();

        let mut counts = HashMap::new();
        for _ in 0..1000 {
            let variant = splitter.select_variant(None, None);
            *counts.entry(variant.name.clone()).or_insert(0) += 1;
        }

        let v1_count = counts.get("v1").unwrap_or(&0);
        let v2_count = counts.get("v2").unwrap_or(&0);

        assert!(*v1_count > 600 && *v1_count < 800);
        assert!(*v2_count > 200 && *v2_count < 400);
    }
}
