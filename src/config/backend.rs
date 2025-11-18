use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::defaults::*;
use super::types::PathPatternConfig;
use super::advanced::CircuitBreakerConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    #[serde(default)]
    pub enable_hybrid: bool,
    #[serde(default = "default_backend_type")]
    pub default_backend: String,
    #[serde(default)]
    pub routing_rules: Vec<RoutingRule>,
    #[serde(default)]
    pub static_files: StaticFilesConfig,
    #[serde(default)]
    pub connection_pool: ConnectionPoolConfig,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            enable_hybrid: false,
            default_backend: default_backend_type(),
            routing_rules: Vec::new(),
            static_files: StaticFilesConfig::default(),
            connection_pool: ConnectionPoolConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub pattern: PathPatternConfig,
    pub backend: String,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticFilesConfig {
    #[serde(default)]
    pub enable: bool,
    pub root: Option<PathBuf>,
    #[serde(default = "default_index_files")]
    pub index_files: Vec<String>,
}

impl Default for StaticFilesConfig {
    fn default() -> Self {
        Self {
            enable: false,
            root: None,
            index_files: default_index_files(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    #[serde(default = "default_pool_max_size")]
    pub max_size: usize,
    #[serde(default = "default_pool_max_idle_time")]
    pub max_idle_time_secs: u64,
    #[serde(default = "default_pool_max_lifetime")]
    pub max_lifetime_secs: u64,
    #[serde(default = "default_pool_connect_timeout")]
    pub connect_timeout_secs: u64,
    #[serde(default)]
    pub enable_metrics: bool,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_size: default_pool_max_size(),
            max_idle_time_secs: default_pool_max_idle_time(),
            max_lifetime_secs: default_pool_max_lifetime(),
            connect_timeout_secs: default_pool_connect_timeout(),
            enable_metrics: true,
            circuit_breaker: CircuitBreakerConfig::default(),
        }
    }
}
