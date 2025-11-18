use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::defaults::*;
use super::types::ListenType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default)]
    pub enable_http2: bool,
    #[serde(default)]
    pub multi_process: bool,
    #[serde(default = "default_workers")]
    pub process_count: usize,
    #[serde(default)]
    pub listen_type: ListenType,
    #[serde(default)]
    pub unix_socket_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub cert_path: Option<PathBuf>,
    #[serde(default)]
    pub key_path: Option<PathBuf>,
    #[serde(default)]
    pub ca_cert_path: Option<PathBuf>,
    #[serde(default)]
    pub alpn_protocols: Vec<String>,
    #[serde(default)]
    pub http_redirect: bool,
    #[serde(default = "default_http_port")]
    pub http_port: u16,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enable: false,
            cert_path: None,
            key_path: None,
            ca_cert_path: None,
            alpn_protocols: vec!["h2".to_string(), "http/1.1".to_string()],
            http_redirect: false,
            http_port: default_http_port(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_admin_host")]
    pub host: String,
    #[serde(default = "default_admin_socket")]
    pub unix_socket: PathBuf,
    #[serde(default = "default_admin_port")]
    pub http_port: u16,
    #[serde(default = "default_allowed_ips")]
    pub allowed_ips: Vec<String>,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enable: false,
            host: default_admin_host(),
            unix_socket: default_admin_socket(),
            http_port: default_admin_port(),
            allowed_ips: default_allowed_ips(),
        }
    }
}
