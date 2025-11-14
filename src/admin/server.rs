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

        // 実際のメトリクスCollectorから取得
        let total_requests = self.metrics_collector.get_total_requests();
        let active_connections = self.metrics_collector.get_active_connections();

        // バックエンド状態を実際のメトリクスから取得
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

async fn dashboard(State(state): State<Arc<AdminState>>) -> Html<String> {
    let status = state.get_status();
    let html = render_dashboard(&status);
    Html(html)
}

async fn api_status(State(state): State<Arc<AdminState>>) -> impl IntoResponse {
    let status = state.get_status();
    Json(status)
}

async fn metrics_page() -> Html<String> {
    Html(render_metrics_page())
}

async fn logs_page() -> Html<String> {
    Html(render_logs_page())
}

async fn waf_page() -> Html<String> {
    Html(render_waf_page())
}

async fn backends_page(State(state): State<Arc<AdminState>>) -> Html<String> {
    let status = state.get_status();
    Html(render_backends_page(&status.backends))
}

async fn system_page() -> Html<String> {
    Html(render_system_page())
}

// API endpoints
async fn api_metrics(State(state): State<Arc<AdminState>>) -> impl IntoResponse {
    let metrics = &state.metrics_collector;

    Json(serde_json::json!({
        "http_requests_total": metrics.get_total_requests(),
        "active_connections": metrics.get_active_connections(),
        "backend_requests": {
            "embedded": metrics.get_backend_requests("embedded"),
            "fastcgi": metrics.get_backend_requests("fastcgi"),
            "static": metrics.get_backend_requests("static")
        }
    }))
}

async fn api_logs() -> impl IntoResponse {
    // ログのサンプル
    use chrono::Utc;
    let now = Utc::now();
    Json(serde_json::json!({
        "logs": [
            {
                "timestamp": now.to_rfc3339(),
                "level": "INFO",
                "message": "Request handled successfully",
                "target": "fe_php::server"
            },
            {
                "timestamp": now.to_rfc3339(),
                "level": "DEBUG",
                "message": "Worker pool executing request",
                "target": "fe_php::php::worker"
            }
        ]
    }))
}

async fn api_waf(State(state): State<Arc<AdminState>>) -> impl IntoResponse {
    let metrics = &state.metrics_collector;

    // 実際のWAFメトリクスを取得
    let total_blocked = metrics.get_waf_blocked_total();
    let rate_limited = metrics.get_rate_limit_triggered();

    // Note: 現在はルール別の統計とブロックIPリストは未実装
    // WAFエンジンに追加の統計収集機能が必要
    Json(serde_json::json!({
        "total_blocked": total_blocked,
        "rate_limit_triggered": rate_limited,
        "rules_triggered": {},
        "blocked_ips": []
    }))
}

async fn api_system() -> impl IntoResponse {
    use sysinfo::{System, Pid};

    let mut sys = System::new_all();
    sys.refresh_all();

    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();

    // Calculate overall CPU usage
    let cpu_usage: f32 = if sys.cpus().is_empty() {
        0.0
    } else {
        sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32
    };

    // Get per-core CPU usage
    let cpu_cores: Vec<f32> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();

    // Get current process info
    let current_pid = Pid::from_u32(std::process::id());
    let (process_cpu, process_memory) = sys.process(current_pid)
        .map(|proc| (proc.cpu_usage(), proc.memory()))
        .unwrap_or((0.0, 0));

    Json(serde_json::json!({
        "cpu_usage": cpu_usage,
        "cpu_count": sys.cpus().len(),
        "cpu_cores": cpu_cores,
        "total_memory": total_memory,
        "used_memory": used_memory,
        "os_name": System::name().unwrap_or_else(|| "Unknown".to_string()),
        "os_version": System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        "kernel_version": System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        "hostname": System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        "process_count": sys.processes().len(),
        "uptime": System::uptime(),
        "process_cpu": process_cpu,
        "process_memory": process_memory
    }))
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
        fe-php Admin Console - リアルタイム更新 (5秒間隔)
    </footer>

    <script>
        // リアルタイム更新
        let updateInterval = 5000; // 5秒

        async function updateMetrics() {{
            try {{
                const response = await fetch('/api/status');
                const data = await response.json();

                // サーバー状態の更新
                document.querySelector('.metric-value').textContent = formatUptime(data.server.uptime_seconds);

                // メトリクスの更新
                const metricsValues = document.querySelectorAll('.metrics-grid')[1].querySelectorAll('.metric-value');
                if (metricsValues.length >= 4) {{
                    metricsValues[0].innerHTML = data.metrics.requests_per_second.toFixed(2) + ' <span class="metric-unit">req/s</span>';
                    metricsValues[1].textContent = data.metrics.active_connections;
                    metricsValues[2].textContent = data.metrics.total_requests;
                    metricsValues[3].innerHTML = data.metrics.error_rate.toFixed(2) + ' <span class="metric-unit">%</span>';
                }}

                // バックエンド状態の更新
                if (data.backends.length > 0) {{
                    updateBackendsTable(data.backends);
                }}
            }} catch (error) {{
                console.error('Failed to update metrics:', error);
            }}
        }}

        function formatUptime(seconds) {{
            const days = Math.floor(seconds / 86400);
            const hours = Math.floor((seconds % 86400) / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            const secs = seconds % 60;

            if (days > 0) {{
                return `${{days}}d ${{hours}}h ${{mins}}m`;
            }} else if (hours > 0) {{
                return `${{hours}}h ${{mins}}m`;
            }} else if (mins > 0) {{
                return `${{mins}}m ${{secs}}s`;
            }} else {{
                return `${{secs}}s`;
            }}
        }}

        function updateBackendsTable(backends) {{
            const tbody = document.querySelector('table tbody');
            if (!tbody) return;

            tbody.innerHTML = '';
            backends.forEach(backend => {{
                const statusClass = backend.status === 'healthy' ? 'status-healthy' :
                                  backend.status === 'degraded' ? 'status-degraded' : 'status-down';
                const row = `
                    <tr>
                        <td>${{backend.name}}</td>
                        <td>${{backend.backend_type}}</td>
                        <td><span class="status-badge ${{statusClass}}">${{backend.status}}</span></td>
                        <td>${{backend.requests}}</td>
                        <td>${{backend.errors}}</td>
                        <td>${{backend.avg_response_ms.toFixed(2)}} ms</td>
                    </tr>
                `;
                tbody.innerHTML += row;
            }});
        }}

        // 初回更新と定期更新を開始
        updateMetrics();
        setInterval(updateMetrics, updateInterval);
    </script>
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

fn render_placeholder_page(title: &str, message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - fe-php Admin Console</title>
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
        .placeholder {{
            background: white;
            border-radius: 8px;
            padding: 60px 20px;
            text-align: center;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .placeholder h2 {{
            font-size: 24px;
            color: #2c3e50;
            margin-bottom: 20px;
        }}
        .placeholder p {{
            color: #7f8c8d;
            font-size: 16px;
            white-space: pre-line;
            line-height: 1.8;
        }}
        .back-link {{
            display: inline-block;
            margin-top: 30px;
            padding: 10px 20px;
            background: #3498db;
            color: white;
            text-decoration: none;
            border-radius: 4px;
        }}
        .back-link:hover {{
            background: #2980b9;
        }}
    </style>
</head>
<body>
    <header>
        <div class="container">
            <h1>fe-php Admin Console</h1>
        </div>
    </header>

    <div class="container">
        <div class="nav">
            <a href="/">Dashboard</a>
            <a href="/metrics">Metrics</a>
            <a href="/logs">Logs</a>
            <a href="/waf">WAF</a>
            <a href="/backends">Backends</a>
            <a href="/system">System</a>
            <a href="/api/status" style="float: right;">JSON API</a>
        </div>

        <div class="placeholder">
            <h2>{title}</h2>
            <p>{message}</p>
            <a href="/" class="back-link">← ダッシュボードに戻る</a>
        </div>
    </div>
</body>
</html>"#,
        title = title,
        message = message,
    )
}

fn render_metrics_page() -> String {
    include_str!("../../templates/admin_metrics.html").to_string()
}

fn render_logs_page() -> String {
    include_str!("../../templates/admin_logs.html").to_string()
}

fn render_waf_page() -> String {
    include_str!("../../templates/admin_waf.html").to_string()
}

fn render_backends_page(backends: &[BackendStatus]) -> String {
    let backends_json = serde_json::to_string(backends).unwrap_or_else(|_| "[]".to_string());
    include_str!("../../templates/admin_backends.html")
        .replace("{backends_json}", &backends_json)
}

fn render_system_page() -> String {
    include_str!("../../templates/admin_system.html").to_string()
}

// Main server function
pub async fn start_admin_server(
    host: String,
    port: u16,
    metrics_collector: Arc<crate::metrics::MetricsCollector>,
) -> Result<()> {
    let state = Arc::new(AdminState::new(metrics_collector));

    let app = Router::new()
        .route("/", get(dashboard))
        .route("/api/status", get(api_status))
        .route("/api/metrics", get(api_metrics))
        .route("/api/logs", get(api_logs))
        .route("/api/waf", get(api_waf))
        .route("/api/system", get(api_system))
        .route("/metrics", get(metrics_page))
        .route("/logs", get(logs_page))
        .route("/waf", get(waf_page))
        .route("/backends", get(backends_page))
        .route("/system", get(system_page))
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("Admin server starting on {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
