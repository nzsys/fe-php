use crate::logging::structured::RequestLog;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStats {
    pub path: String,
    pub count: usize,
    pub avg_duration_ms: f64,
    pub error_count: usize,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousActivity {
    pub ip_address: String,
    pub event_type: String,
    pub count: usize,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAnalysisResult {
    pub total_requests: usize,
    pub error_count: usize,
    pub top_endpoints: Vec<EndpointStats>,
    pub slow_requests: Vec<RequestLog>,
    pub suspicious_activity: Vec<SuspiciousActivity>,
}

pub struct LogAnalyzer {
    logs: Vec<RequestLog>,
}

impl LogAnalyzer {
    pub fn new() -> Self {
        Self {
            logs: Vec::new(),
        }
    }

    /// Load logs from file
    pub fn load_from_file(&mut self, _path: &Path) -> Result<()> {
        // TODO: 実際のログファイル読み込みを実装
        // JSONLフォーマットのログを1行ずつパース
        Ok(())
    }

    /// Add log entry
    pub fn add_log(&mut self, log: RequestLog) {
        self.logs.push(log);

        // メモリ節約のため、最新1000件のみ保持
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
    }

    /// Get recent logs
    pub fn get_recent_logs(&self, limit: usize) -> Vec<RequestLog> {
        let start = if self.logs.len() > limit {
            self.logs.len() - limit
        } else {
            0
        };
        self.logs[start..].to_vec()
    }

    /// Analyze logs
    pub fn analyze(&self) -> LogAnalysisResult {
        let total_requests = self.logs.len();
        let error_count = self.logs.iter()
            .filter(|log| log.status >= 400)
            .count();

        // エンドポイント統計
        let top_endpoints = self.analyze_endpoints();

        // スローリクエスト（100ms以上）
        let mut slow_requests: Vec<_> = self.logs.iter()
            .filter(|log| log.duration_ms > 100)
            .cloned()
            .collect();
        slow_requests.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
        slow_requests.truncate(10);

        // 不審なアクティビティ
        let suspicious_activity = self.detect_suspicious_activity();

        LogAnalysisResult {
            total_requests,
            error_count,
            top_endpoints,
            slow_requests,
            suspicious_activity,
        }
    }

    /// Analyze endpoint statistics
    fn analyze_endpoints(&self) -> Vec<EndpointStats> {
        let mut endpoint_map: HashMap<String, (usize, u64, usize)> = HashMap::new();

        for log in &self.logs {
            let entry = endpoint_map.entry(log.uri.clone()).or_insert((0, 0, 0));
            entry.0 += 1; // count
            entry.1 += log.duration_ms; // total duration
            if log.status >= 400 {
                entry.2 += 1; // error count
            }
        }

        let mut stats: Vec<_> = endpoint_map.iter()
            .map(|(path, (count, total_duration, error_count))| {
                EndpointStats {
                    path: path.clone(),
                    count: *count,
                    avg_duration_ms: *total_duration as f64 / *count as f64,
                    error_count: *error_count,
                    error_rate: *error_count as f64 / *count as f64,
                }
            })
            .collect();

        stats.sort_by(|a, b| b.count.cmp(&a.count));
        stats.truncate(10);
        stats
    }

    /// Detect suspicious activity
    fn detect_suspicious_activity(&self) -> Vec<SuspiciousActivity> {
        let mut ip_404_map: HashMap<String, usize> = HashMap::new();
        let mut ip_5xx_map: HashMap<String, usize> = HashMap::new();

        for log in &self.logs {
            if log.status == 404 {
                *ip_404_map.entry(log.remote_addr.clone()).or_insert(0) += 1;
            } else if log.status >= 500 {
                *ip_5xx_map.entry(log.remote_addr.clone()).or_insert(0) += 1;
            }
        }

        let mut suspicious = Vec::new();

        // 大量の404エラー（スキャン活動の可能性）
        for (ip, count) in ip_404_map {
            if count > 10 {
                suspicious.push(SuspiciousActivity {
                    ip_address: ip,
                    event_type: "scan".to_string(),
                    count,
                    description: format!("{} 404 errors (possible scanning)", count),
                });
            }
        }

        // 大量の5xxエラー（問題のある挙動）
        for (ip, count) in ip_5xx_map {
            if count > 5 {
                suspicious.push(SuspiciousActivity {
                    ip_address: ip,
                    event_type: "errors".to_string(),
                    count,
                    description: format!("{} server errors", count),
                });
            }
        }

        suspicious
    }
}

impl Default for LogAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
