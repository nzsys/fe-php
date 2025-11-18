use anyhow::Result;
use clap::Args;
use crate::admin::api::AdminApi;
use crate::metrics::MetricsCollector;
use crate::monitor::MonitorCollector;
use crate::tui;
use crate::tui::client::TuiClient;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Args, Debug)]
pub struct MonitorArgs {
    /// Output format (tui, json, text)
    #[arg(short, long, default_value = "tui")]
    format: String,

    /// Refresh interval in seconds (only for TUI mode)
    #[arg(short, long, default_value = "1")]
    refresh: u64,

    /// Unix socket path to connect to running server
    #[arg(short, long)]
    socket: Option<String>,
}

pub async fn run(args: MonitorArgs) -> Result<()> {
    // Create monitor collector based on whether socket path is provided
    let (monitor, client) = if let Some(socket_path) = args.socket {
        // Remote mode: Connect to Unix socket
        let client = Arc::new(TuiClient::new(PathBuf::from(socket_path)));
        let monitor = MonitorCollector::new_remote(client.clone());
        (monitor, Some(client))
    } else {
        // Local mode: Create local metrics collector and admin API
        let metrics = Arc::new(MetricsCollector::new());
        let admin_api = AdminApi::new(metrics.clone());
        let monitor = MonitorCollector::new(admin_api);
        (monitor, None)
    };

    match args.format.as_str() {
        "tui" => {
            // Run TUI
            let app = if let Some(client) = client {
                tui::app::App::with_client(monitor, client)
            } else {
                tui::app::App::new(monitor)
            };
            tui::run_tui(app).await?;
        }
        "json" => {
            // Output JSON
            let mut collector = monitor;
            let snapshot = collector.take_snapshot().await?;
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
        }
        "text" => {
            // Output text
            let mut collector = monitor;
            let snapshot = collector.take_snapshot().await?;
            print_text_status(&snapshot);
        }
        _ => {
            anyhow::bail!("Invalid format: {}. Use 'tui', 'json', or 'text'", args.format);
        }
    }

    Ok(())
}

fn print_text_status(snapshot: &crate::monitor::collector::MonitorSnapshot) {
    println!("=== fe-php Server Status ===");
    println!("Timestamp: {}", snapshot.timestamp);
    println!();

    println!("Server:");
    println!("  Uptime: {}s", snapshot.server_status.uptime_seconds);
    println!("  Active Connections: {}", snapshot.server_status.active_connections);
    println!("  Total Requests: {}", snapshot.server_status.total_requests);
    println!("  Request Rate: {:.2} req/s", snapshot.request_rate);
    println!("  Error Rate: {:.2}%", snapshot.error_rate * 100.0);
    println!();

    println!("Backends:");
    for (name, stats) in &snapshot.server_status.backends {
        let error_rate = if stats.requests > 0 {
            (stats.errors as f64 / stats.requests as f64) * 100.0
        } else {
            0.0
        };

        println!("  {}:", name);
        println!("    Requests: {}", stats.requests);
        println!("    Errors: {} ({:.2}%)", stats.errors, error_rate);
        println!("    Avg Response Time: {:.2}ms", stats.avg_response_ms);
    }
}
