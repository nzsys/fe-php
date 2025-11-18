use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestLog {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub request_id: String,
    pub method: String,
    pub uri: String,
    pub status: u16,
    pub duration_ms: u64,
    pub memory_peak_mb: f64,
    pub opcache_hit: bool,
    pub worker_id: Option<usize>,
    pub remote_addr: String,
    pub user_agent: Option<String>,
    pub waf_triggered: bool,
}

impl RequestLog {
    pub fn new(
        method: String,
        uri: String,
        status: u16,
        duration_ms: u64,
        remote_addr: String,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            level: "info".to_string(),
            request_id: Uuid::new_v4().to_string(),
            method,
            uri,
            status,
            duration_ms,
            memory_peak_mb: 0.0,
            opcache_hit: false,
            worker_id: None,
            remote_addr,
            user_agent: None,
            waf_triggered: false,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorLog {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub request_id: Option<String>,
    pub error: String,
    pub context: Option<String>,
}

impl ErrorLog {
    pub fn new(error: String, context: Option<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level: "error".to_string(),
            request_id: None,
            error,
            context,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}
