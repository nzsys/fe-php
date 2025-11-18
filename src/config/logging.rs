use serde::{Deserialize, Serialize};
use super::defaults::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default = "default_log_output")]
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_metrics_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}
