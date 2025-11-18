use anyhow::Result;
use axum::{
    extract::State,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    pub uptime_seconds: u64,
    pub pid: u32,
    pub started_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentMetrics {
    pub requests_per_second: f64,
    pub active_connections: usize,
    pub total_requests: u64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatus {
    pub name: String,
    pub backend_type: String, // "embedded", "fastcgi", "static"
    pub status: String,       // "healthy", "degraded", "down"
    pub requests: u64,
    pub errors: u64,
    pub avg_response_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub server: ServerInfo,
    pub metrics: CurrentMetrics,
    pub backends: Vec<BackendStatus>,
}

#[derive(Clone)]
pub struct AdminState {
    start_time: u64,
    metrics_collector: Arc<crate::metrics::MetricsCollector>,
}

impl AdminState {
    fn new(metrics_collector: Arc<crate::metrics::MetricsCollector>) -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            start_time,
            metrics_collector,
        }
    }

    fn get_status(&self) -> StatusResponse {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let uptime = now - self.start_time;

        // Get actual metrics from MetricsCollector
        let total_requests = self.metrics_collector.get_total_requests();
        let active_connections = self.metrics_collector.get_active_connections();

        // Get backend status from actual metrics
        let backends = vec![
            BackendStatus {
                name: "Embedded (libphp)".to_string(),
                backend_type: "embedded".to_string(),
                status: "healthy".to_string(),
                requests: self.metrics_collector.get_backend_requests("embedded"),
                errors: self.metrics_collector.get_backend_errors("embedded"),
                avg_response_ms: self.metrics_collector.get_backend_avg_response_ms("embedded"),
            },
            BackendStatus {
                name: "FastCGI (PHP-FPM)".to_string(),
                backend_type: "fastcgi".to_string(),
                status: "healthy".to_string(),
                requests: self.metrics_collector.get_backend_requests("fastcgi"),
                errors: self.metrics_collector.get_backend_errors("fastcgi"),
                avg_response_ms: self.metrics_collector.get_backend_avg_response_ms("fastcgi"),
            },
            BackendStatus {
                name: "Static Files".to_string(),
                backend_type: "static".to_string(),
                status: "healthy".to_string(),
                requests: self.metrics_collector.get_backend_requests("static"),
                errors: self.metrics_collector.get_backend_errors("static"),
                avg_response_ms: self.metrics_collector.get_backend_avg_response_ms("static"),
            },
        ];

        // Calculate error rate
        let total_errors: u64 = backends.iter().map(|b| b.errors).sum();
        let error_rate = if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        // Calculate requests per second (approximate)
        let requests_per_second = if uptime > 0 {
            total_requests as f64 / uptime as f64
        } else {
            0.0
        };

        StatusResponse {
            server: ServerInfo {
                version: crate::VERSION.to_string(),
                uptime_seconds: uptime,
                pid: std::process::id(),
                started_at: self.start_time,
            },
            metrics: CurrentMetrics {
                requests_per_second,
                active_connections: active_connections as usize,
                total_requests,
                error_rate,
            },
            backends,
        }
    }
}

/// JSON API: Get server status
async fn api_status(State(state): State<Arc<AdminState>>) -> impl IntoResponse {
    let status = state.get_status();
    Json(status)
}

/// JSON API: Health check
async fn api_health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "version": crate::VERSION,
    }))
}

/// JSON API: Prometheus metrics
async fn api_metrics(State(state): State<Arc<AdminState>>) -> impl IntoResponse {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = state.metrics_collector.registry().gather();

    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap_or_default();

    String::from_utf8(buffer).unwrap_or_default()
}

pub async fn serve(
    addr: &str,
    metrics_collector: Arc<crate::metrics::MetricsCollector>,
) -> Result<()> {
    let state = Arc::new(AdminState::new(metrics_collector));

    let app = Router::new()
        .route("/api/status", get(api_status))
        .route("/api/health", get(api_health))
        .route("/metrics", get(api_metrics))
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Admin JSON API server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
