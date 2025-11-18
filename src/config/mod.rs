pub mod parser;
pub mod validator;

mod defaults;
mod types;
mod server;
mod php;
mod security;
mod advanced;
mod backend;
mod logging;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Re-export types
pub use types::*;
pub use server::*;
pub use php::*;
pub use security::*;
pub use advanced::*;
pub use backend::*;
pub use logging::*;

/// Main configuration structure for the fe-php server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub php: PhpConfig,
    pub logging: LoggingConfig,
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub waf: WafConfig,
    #[serde(default)]
    pub admin: AdminConfig,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub geoip: GeoIpConfig,
    #[serde(default)]
    pub redis: RedisConfig,
    #[serde(default)]
    pub tracing: TracingConfig,
    #[serde(default)]
    pub load_balancing: LoadBalancingConfig,
    #[serde(default)]
    pub deployment: DeploymentConfig,
    #[serde(default)]
    pub backend: BackendConfig,
}

impl Config {
    /// Load configuration from a file
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        parser::parse_config(path)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<Vec<String>> {
        validator::validate_config(self)
    }
}
