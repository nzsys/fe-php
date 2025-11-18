use crate::admin::api::{AdminApi, ServerStatus};
use crate::metrics::collector::BackendStats;
use crate::tui::client::TuiClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub server_status: ServerStatus,
    pub request_rate: f64,
    pub error_rate: f64,
}

/// Data source for monitoring
pub enum DataSource {
    /// Local: Direct access to AdminApi
    Local(AdminApi),
    /// Remote: Via Unix socket
    Remote(Arc<TuiClient>),
}

pub struct MonitorCollector {
    data_source: DataSource,
    // 過去のスナップショット（直近100個）
    history: Vec<MonitorSnapshot>,
}

impl MonitorCollector {
    /// Create new collector with local data source
    pub fn new(admin_api: AdminApi) -> Self {
        Self {
            data_source: DataSource::Local(admin_api),
            history: Vec::new(),
        }
    }

    /// Create new collector with remote data source
    pub fn new_remote(client: Arc<TuiClient>) -> Self {
        Self {
            data_source: DataSource::Remote(client),
            history: Vec::new(),
        }
    }

    /// Get server status from data source
    async fn get_server_status(&self) -> Result<ServerStatus> {
        match &self.data_source {
            DataSource::Local(api) => Ok(api.get_status()),
            DataSource::Remote(client) => client.get_status().await,
        }
    }

    /// Take a snapshot of current server state
    pub async fn take_snapshot(&mut self) -> Result<MonitorSnapshot> {
        let server_status = self.get_server_status().await?;

        // 前回のスナップショットと比較してレートを計算
        let (request_rate, error_rate) = self.calculate_rates(&server_status);

        let snapshot = MonitorSnapshot {
            timestamp: chrono::Utc::now(),
            server_status,
            request_rate,
            error_rate,
        };

        // 履歴に追加（最大100個）
        self.history.push(snapshot.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        Ok(snapshot)
    }

    /// Calculate request rate and error rate
    fn calculate_rates(&self, current_status: &ServerStatus) -> (f64, f64) {
        if let Some(prev) = self.history.last() {
            let time_diff = (chrono::Utc::now() - prev.timestamp).num_seconds() as f64;
            if time_diff > 0.0 {
                let request_diff = current_status.total_requests.saturating_sub(
                    prev.server_status.total_requests
                ) as f64;

                let request_rate = request_diff / time_diff;

                // エラーレートの計算（全バックエンドのエラー合計）
                let current_errors: u64 = current_status.backends.values()
                    .map(|b| b.errors)
                    .sum();
                let prev_errors: u64 = prev.server_status.backends.values()
                    .map(|b| b.errors)
                    .sum();

                let error_diff = current_errors.saturating_sub(prev_errors) as f64;
                let error_rate = if request_diff > 0.0 {
                    error_diff / request_diff
                } else {
                    0.0
                };

                return (request_rate, error_rate);
            }
        }

        (0.0, 0.0)
    }

    /// Get recent history
    pub fn get_history(&self, count: usize) -> Vec<MonitorSnapshot> {
        let start = if self.history.len() > count {
            self.history.len() - count
        } else {
            0
        };
        self.history[start..].to_vec()
    }

    /// Get current server status
    pub async fn get_current_status(&self) -> Result<ServerStatus> {
        self.get_server_status().await
    }

    /// Get backend statistics
    pub async fn get_backend_stats(&self) -> Result<HashMap<String, BackendStats>> {
        let status = self.get_server_status().await?;
        Ok(status.backends)
    }
}
