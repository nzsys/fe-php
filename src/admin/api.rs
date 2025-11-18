use serde::{Deserialize, Serialize};
use crate::metrics::MetricsCollector;
use crate::metrics::collector::BackendStats;
use crate::monitor::analyzer::{LogAnalyzer, LogAnalysisResult};
use crate::server::ip_blocker::IpBlocker;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use parking_lot::RwLock;
use thiserror::Error;

/// Error type for Admin API operations
#[derive(Debug, Error)]
pub enum AdminError {
    /// Command channel is not available
    #[error("Command channel not available: {0}")]
    NoCommandChannel(String),

    /// Failed to send command through channel
    #[error("Failed to send command: {0}")]
    SendError(String),
}

impl From<mpsc::error::SendError<AdminCommand>> for AdminError {
    fn from(err: mpsc::error::SendError<AdminCommand>) -> Self {
        AdminError::SendError(err.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkerStatus {
    pub worker_id: usize,
    pub status: String,
    pub requests_handled: usize,
    pub memory_mb: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerStatus {
    pub uptime_seconds: u64,
    pub active_connections: i64,
    pub total_requests: u64,
    pub workers: Vec<WorkerStatus>,
    pub backends: HashMap<String, BackendStats>,
    #[serde(default)]
    pub recent_logs: Vec<crate::logging::structured::RequestLog>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub version: String,
}

/// Admin command for server operations
#[derive(Debug, Clone)]
pub enum AdminCommand {
    ReloadConfig,
    RestartWorkers,
    BlockIp(String),
    UnblockIp(String),
}

pub struct AdminApi {
    metrics: Arc<MetricsCollector>,
    // Channel for sending admin commands to the server
    command_tx: Option<mpsc::UnboundedSender<AdminCommand>>,
    // Log analyzer for analysis endpoint
    log_analyzer: Arc<RwLock<LogAnalyzer>>,
    // IP blocker for runtime IP blocking
    ip_blocker: Option<Arc<IpBlocker>>,
    // Worker pool size (for worker status reporting)
    worker_pool_size: usize,
}

impl AdminApi {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self {
            metrics,
            command_tx: None,
            log_analyzer: Arc::new(RwLock::new(LogAnalyzer::new())),
            ip_blocker: None,
            worker_pool_size: 0,
        }
    }

    /// Create AdminApi with command channel support
    pub fn with_command_channel(
        metrics: Arc<MetricsCollector>,
        command_tx: mpsc::UnboundedSender<AdminCommand>,
        ip_blocker: Arc<IpBlocker>,
        worker_pool_size: usize,
    ) -> Self {
        Self {
            metrics,
            command_tx: Some(command_tx),
            log_analyzer: Arc::new(RwLock::new(LogAnalyzer::new())),
            ip_blocker: Some(ip_blocker),
            worker_pool_size,
        }
    }

    /// Get current server status
    pub fn get_status(&self) -> ServerStatus {
        let uptime = self.metrics.get_uptime_seconds();
        let active_connections = self.metrics.get_active_connections();
        let total_requests = self.metrics.get_total_requests();
        let backends = self.metrics.get_all_backend_stats();

        // Generate worker status based on pool size
        // Note: Actual per-worker metrics would require WorkerPool integration
        let workers: Vec<WorkerStatus> = (0..self.worker_pool_size)
            .map(|worker_id| WorkerStatus {
                worker_id,
                status: "idle".to_string(), // Simplified status
                requests_handled: 0, // Would need per-worker tracking
                memory_mb: 0.0, // Would need per-worker tracking
            })
            .collect();

        // Get recent logs (last 100 entries)
        let recent_logs = self.get_recent_logs(100);

        ServerStatus {
            uptime_seconds: uptime,
            active_connections,
            total_requests,
            workers,
            backends,
            recent_logs,
        }
    }

    /// Health check endpoint
    pub fn health_check(&self) -> HealthCheckResponse {
        HealthCheckResponse {
            status: "healthy".to_string(),
            uptime_seconds: self.metrics.get_uptime_seconds(),
            version: crate::VERSION.to_string(),
        }
    }

    /// Reload configuration
    ///
    /// # Errors
    /// Returns `AdminError::NoCommandChannel` if the command channel is not available,
    /// or `AdminError::SendError` if sending the command fails.
    pub fn reload_config(&self) -> Result<(), AdminError> {
        let tx = self.command_tx.as_ref().ok_or_else(|| {
            AdminError::NoCommandChannel("Configuration reload not supported".to_string())
        })?;

        tx.send(AdminCommand::ReloadConfig)?;
        Ok(())
    }

    /// Restart workers
    ///
    /// # Errors
    /// Returns `AdminError::NoCommandChannel` if the command channel is not available,
    /// or `AdminError::SendError` if sending the command fails.
    pub fn restart_workers(&self) -> Result<(), AdminError> {
        let tx = self.command_tx.as_ref().ok_or_else(|| {
            AdminError::NoCommandChannel("Worker restart not supported".to_string())
        })?;

        tx.send(AdminCommand::RestartWorkers)?;
        Ok(())
    }

    /// Block IP address
    ///
    /// # Errors
    /// Returns `AdminError::NoCommandChannel` if the command channel is not available,
    /// or `AdminError::SendError` if sending the command fails.
    pub fn block_ip(&self, ip: String) -> Result<(), AdminError> {
        let tx = self.command_tx.as_ref().ok_or_else(|| {
            AdminError::NoCommandChannel("IP blocking not supported".to_string())
        })?;

        tx.send(AdminCommand::BlockIp(ip))?;
        Ok(())
    }

    /// Unblock IP address
    ///
    /// # Errors
    /// Returns `AdminError::NoCommandChannel` if the command channel is not available,
    /// or `AdminError::SendError` if sending the command fails.
    pub fn unblock_ip(&self, ip: String) -> Result<(), AdminError> {
        let tx = self.command_tx.as_ref().ok_or_else(|| {
            AdminError::NoCommandChannel("IP unblocking not supported".to_string())
        })?;

        tx.send(AdminCommand::UnblockIp(ip))?;
        Ok(())
    }

    /// Get metrics in Prometheus format
    pub fn get_metrics_text(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.metrics.registry().gather();

        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap_or_default();

        String::from_utf8(buffer).unwrap_or_default()
    }

    /// Get log analysis result
    pub fn get_log_analysis(&self) -> LogAnalysisResult {
        let analyzer = self.log_analyzer.read();
        analyzer.analyze()
    }

    /// Get recent logs
    pub fn get_recent_logs(&self, limit: usize) -> Vec<crate::logging::structured::RequestLog> {
        let analyzer = self.log_analyzer.read();
        analyzer.get_recent_logs(limit)
    }

    /// Get log analyzer (for adding logs from request handlers)
    pub fn log_analyzer(&self) -> Arc<RwLock<LogAnalyzer>> {
        Arc::clone(&self.log_analyzer)
    }

    /// Get list of blocked IPs
    pub fn get_blocked_ips(&self) -> Vec<String> {
        if let Some(ref blocker) = self.ip_blocker {
            blocker.get_blocked_ips()
        } else {
            vec![]
        }
    }
}

impl Default for AdminApi {
    fn default() -> Self {
        Self::new(Arc::new(MetricsCollector::new()))
    }
}
