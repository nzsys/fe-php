use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::defaults::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpConfig {
    pub libphp_path: PathBuf,
    pub document_root: PathBuf,
    #[serde(default = "default_workers")]
    pub worker_pool_size: usize,
    #[serde(default = "default_max_requests")]
    pub worker_max_requests: usize,
    #[serde(default)]
    pub opcache: OpcacheConfig,
    #[serde(default)]
    pub use_fpm: bool,
    #[serde(default = "default_fpm_socket")]
    pub fpm_socket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcacheConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_opcache_memory")]
    pub memory_size: String,
    #[serde(default = "default_max_files")]
    pub max_files: usize,
    #[serde(default)]
    pub validate_timestamps: bool,
}

impl Default for OpcacheConfig {
    fn default() -> Self {
        Self {
            enable: default_true(),
            memory_size: default_opcache_memory(),
            max_files: default_max_files(),
            validate_timestamps: false,
        }
    }
}
