use anyhow::Result;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
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
}

impl AdminState {
    fn new() -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self { start_time }
    }

    fn get_status(&self) -> StatusResponse {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let uptime = now - self.start_time;

        StatusResponse {
            server: ServerInfo {
                version: crate::VERSION.to_string(),
                uptime_seconds: uptime,
                pid: std::process::id(),
                started_at: self.start_time,
            },
            metrics: CurrentMetrics {
                requests_per_second: 0.0,
                active_connections: 0,
                total_requests: 0,
                error_rate: 0.0,
            },
            backends: vec![],
        }
    }
}

async fn dashboard(State(state): State<Arc<AdminState>>) -> Html<String> {
    let status = state.get_status();
    let html = render_dashboard(&status);
    Html(html)
}

async fn api_status(State(state): State<Arc<AdminState>>) -> impl IntoResponse {
    let status = state.get_status();
    Json(status)
}

fn render_dashboard(status: &StatusResponse) -> String {
    let uptime_str = format_uptime(status.server.uptime_seconds);

    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>fe-php Admin Console</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background: #f5f5f5;
            color: #333;
            line-height: 1.6;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }}
        header {{
            background: #2c3e50;
            color: white;
            padding: 20px 0;
            margin-bottom: 30px;
        }}
        header h1 {{
            font-size: 28px;
            font-weight: 600;
        }}
        header .version {{
            font-size: 14px;
            opacity: 0.8;
            margin-top: 5px;
        }}
        .nav {{
            background: white;
            padding: 15px;
            border-radius: 8px;
            margin-bottom: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .nav a {{
            text-decoration: none;
            color: #3498db;
            margin-right: 20px;
            font-weight: 500;
        }}
        .nav a:hover {{
            color: #2980b9;
        }}
        .nav a.active {{
            color: #2c3e50;
            font-weight: 600;
        }}
        .card {{
            background: white;
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .card h2 {{
            font-size: 20px;
            margin-bottom: 15px;
            color: #2c3e50;
            border-bottom: 2px solid #3498db;
            padding-bottom: 10px;
        }}
        .metrics-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
        }}
        .metric {{
            background: #f8f9fa;
            padding: 15px;
            border-radius: 6px;
            border-left: 4px solid #3498db;
        }}
        .metric-label {{
            font-size: 14px;
            color: #7f8c8d;
            margin-bottom: 5px;
        }}
        .metric-value {{
            font-size: 24px;
            font-weight: 600;
            color: #2c3e50;
        }}
        .metric-unit {{
            font-size: 14px;
            color: #95a5a6;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
        }}
        th, td {{
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #ecf0f1;
        }}
        th {{
            background: #f8f9fa;
            font-weight: 600;
            color: #2c3e50;
        }}
        .status-badge {{
            display: inline-block;
            padding: 4px 12px;
            border-radius: 12px;
            font-size: 12px;
            font-weight: 600;
        }}
        .status-healthy {{
            background: #d4edda;
            color: #155724;
        }}
        .status-degraded {{
            background: #fff3cd;
            color: #856404;
        }}
        .status-down {{
            background: #f8d7da;
            color: #721c24;
        }}
        .empty-state {{
            text-align: center;
            padding: 40px;
            color: #95a5a6;
        }}
        footer {{
            text-align: center;
            padding: 20px;
            color: #95a5a6;
            font-size: 14px;
        }}
    </style>
</head>
<body>
    <header>
        <div class="container">
            <h1>fe-php Admin Console</h1>
            <div class="version">Version {version} | PID: {pid}</div>
        </div>
    </header>

    <div class="container">
        <div class="nav">
            <a href="/" class="active">Dashboard</a>
            <a href="/metrics">Metrics</a>
            <a href="/logs">Logs</a>
            <a href="/waf">WAF</a>
            <a href="/backends">Backends</a>
            <a href="/system">System</a>
            <a href="/api/status" style="float: right;">JSON API</a>
        </div>

        <div class="card">
            <h2>サーバー状態</h2>
            <div class="metrics-grid">
                <div class="metric">
                    <div class="metric-label">Uptime</div>
                    <div class="metric-value">{uptime}</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Started At</div>
                    <div class="metric-value">{started_at}</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Version</div>
                    <div class="metric-value">{version}</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Process ID</div>
                    <div class="metric-value">{pid}</div>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>リアルタイムメトリクス</h2>
            <div class="metrics-grid">
                <div class="metric">
                    <div class="metric-label">Requests/sec</div>
                    <div class="metric-value">{rps} <span class="metric-unit">req/s</span></div>
                </div>
                <div class="metric">
                    <div class="metric-label">Active Connections</div>
                    <div class="metric-value">{active_conn}</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Total Requests</div>
                    <div class="metric-value">{total_req}</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Error Rate</div>
                    <div class="metric-value">{error_rate} <span class="metric-unit">%</span></div>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>バックエンド状態</h2>
            {backends_table}
        </div>
    </div>

    <footer>
        fe-php Admin Console - Read Only Interface
    </footer>
</body>
</html>"#,
        version = status.server.version,
        pid = status.server.pid,
        uptime = uptime_str,
        started_at = format_timestamp(status.server.started_at),
        rps = format!("{:.2}", status.metrics.requests_per_second),
        active_conn = status.metrics.active_connections,
        total_req = status.metrics.total_requests,
        error_rate = format!("{:.2}", status.metrics.error_rate),
        backends_table = render_backends_table(&status.backends),
    )
}

fn render_backends_table(backends: &[BackendStatus]) -> String {
    if backends.is_empty() {
        return r#"<div class="empty-state">バックエンド情報は起動後に表示されます</div>"#.to_string();
    }

    let mut rows = String::new();
    for backend in backends {
        let status_class = match backend.status.as_str() {
            "healthy" => "status-healthy",
            "degraded" => "status-degraded",
            _ => "status-down",
        };
        rows.push_str(&format!(
            r#"<tr>
                <td>{}</td>
                <td>{}</td>
                <td><span class="status-badge {}">{}</span></td>
                <td>{}</td>
                <td>{}</td>
                <td>{:.2} ms</td>
            </tr>"#,
            backend.name,
            backend.backend_type,
            status_class,
            backend.status,
            backend.requests,
            backend.errors,
            backend.avg_response_ms,
        ));
    }

    format!(
        r#"<table>
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Type</th>
                    <th>Status</th>
                    <th>Requests</th>
                    <th>Errors</th>
                    <th>Avg Response</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>"#,
        rows
    )
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

fn format_timestamp(timestamp: u64) -> String {
    use chrono::{DateTime, Utc, TimeZone};
    let dt: DateTime<Utc> = Utc.timestamp_opt(timestamp as i64, 0).unwrap();
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

// Main server function
pub async fn start_admin_server(host: String, port: u16) -> Result<()> {
    let state = Arc::new(AdminState::new());

    let app = Router::new()
        .route("/", get(dashboard))
        .route("/api/status", get(api_status))
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("Admin server starting on {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
