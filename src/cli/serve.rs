use clap::Args;
use anyhow::Result;
use crate::{Config, Server};
use crate::server::config_reload::ConfigReloadManager;
use crate::admin::api::AdminCommand;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error};

#[derive(Args)]
pub struct ServeArgs {
    #[arg(short, long, default_value = "fe-php.toml")]
    pub config: PathBuf,
}

pub async fn run(args: ServeArgs) -> Result<()> {
    let config = Config::from_file(&args.config)?;

    crate::logging::init_logging(&config.logging.level, &config.logging.format)?;

    info!("Starting fe-php server v{}", crate::VERSION);
    info!("Loading configuration from: {}", args.config.display());

    let warnings = config.validate()?;
    for warning in warnings {
        println!("{}", warning);
    }

    crate::utils::setup_signal_handlers().await?;

    crate::metrics::init_metrics();

    // Create config reload manager
    let config_reload_manager = Arc::new(ConfigReloadManager::new(
        args.config.clone(),
        config.clone(),
    ));

    // Create server first to get metrics collector and ip blocker
    let mut server = Server::new(config.clone()).await?;
    let metrics_collector = server.metrics_collector();
    let ip_blocker = server.ip_blocker();

    // Create admin command channel
    let (admin_tx, mut admin_rx) = mpsc::unbounded_channel::<AdminCommand>();

    // Spawn admin command handler
    let reload_manager = config_reload_manager.clone();
    let ip_blocker_clone = ip_blocker.clone();
    tokio::spawn(async move {
        while let Some(command) = admin_rx.recv().await {
            match command {
                AdminCommand::ReloadConfig => {
                    info!("Received config reload request");
                    if let Err(e) = reload_manager.reload() {
                        error!("Failed to reload configuration: {}", e);
                    } else {
                        info!("Configuration reloaded successfully");
                    }
                }
                AdminCommand::RestartWorkers => {
                    info!("Received worker restart request (not yet implemented)");
                    // TODO: Implement worker restart
                }
                AdminCommand::BlockIp(ip) => {
                    info!("Received request to block IP: {}", ip);
                    match ip_blocker_clone.block(&ip) {
                        Ok(()) => {
                            info!("IP {} successfully blocked", ip);
                        }
                        Err(e) => {
                            error!("Failed to block IP {}: {}", ip, e);
                        }
                    }
                }
                AdminCommand::UnblockIp(ip) => {
                    info!("Received request to unblock IP: {}", ip);
                    match ip_blocker_clone.unblock(&ip) {
                        Ok(()) => {
                            info!("IP {} successfully unblocked", ip);
                        }
                        Err(e) => {
                            error!("Failed to unblock IP {}: {}", ip, e);
                        }
                    }
                }
            }
        }
    });

    if config.metrics.enable {
        let metrics_port = config.metrics.port;
        let metrics_endpoint = config.metrics.endpoint.clone();
        let metrics_for_server = metrics_collector.clone();
        tokio::spawn(async move {
            if let Err(e) = start_metrics_server(metrics_port, &metrics_endpoint, metrics_for_server).await {
                tracing::error!("Metrics server error: {}", e);
            }
        });
        info!("Metrics endpoint available at http://localhost:{}{}", config.metrics.port, config.metrics.endpoint);
    }

    if config.admin.enable {
        // Create AdminApi with command channel
        let worker_pool_size = config.server.workers;
        let admin_api = Arc::new(crate::admin::AdminApi::with_command_channel(
            metrics_collector.clone(),
            admin_tx.clone(),
            ip_blocker.clone(),
            worker_pool_size,
        ));

        // Start HTTP JSON API (optional, for external tools)
        let admin_host = config.admin.host.clone();
        let admin_port = config.admin.http_port;
        let metrics_for_admin = metrics_collector.clone();
        tokio::spawn(async move {
            let addr = format!("{}:{}", admin_host, admin_port);
            if let Err(e) = crate::admin::serve_json_api(&addr, metrics_for_admin).await {
                error!("Admin JSON API server error: {}", e);
            }
        });
        info!("Admin HTTP JSON API available at http://{}:{}", config.admin.host, config.admin.http_port);

        // Start Unix Socket server (recommended for TUI)
        let socket_path = config.admin.unix_socket.clone();
        let socket_server = crate::admin::UnixSocketServer::new(socket_path.clone(), admin_api.clone());
        tokio::spawn(async move {
            if let Err(e) = socket_server.serve().await {
                error!("Unix socket server error: {}", e);
            }
        });
        info!("Admin Unix Socket available at {:?}", config.admin.unix_socket);
        info!("Hot configuration reload enabled (send reload command via admin API)");

        // Set AdminApi on server for logging
        server.set_admin_api(admin_api.clone());
    }

    info!("Server starting...");

    server.serve().await?;

    Ok(())
}

async fn start_metrics_server(port: u16, endpoint: &str, metrics_collector: Arc<crate::metrics::MetricsCollector>) -> Result<()> {
    use hyper::service::service_fn;
    use hyper::{Request, Response, body::{Incoming, Bytes}};
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;
    use http_body_util::Full;
    use prometheus::Encoder;

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    let endpoint_path = endpoint.to_string();

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let endpoint_path = endpoint_path.clone();
        let metrics_collector = Arc::clone(&metrics_collector);

        tokio::spawn(async move {
            let service = service_fn(move |req: Request<Incoming>| {
                let endpoint_path = endpoint_path.clone();
                let metrics_collector = Arc::clone(&metrics_collector);
                async move {
                    if req.uri().path() == endpoint_path {
                        // Use MetricsCollector's registry instead of global registry
                        let encoder = prometheus::TextEncoder::new();
                        let metric_families = metrics_collector.registry().gather();
                        let mut buffer = Vec::new();

                        match encoder.encode(&metric_families, &mut buffer) {
                            Ok(_) => Ok::<_, hyper::Error>(Response::new(Full::new(Bytes::from(buffer)))),
                            Err(_) => Ok(Response::builder()
                                .status(500)
                                .body(Full::new(Bytes::from("Error exporting metrics")))
                                .unwrap()),
                        }
                    } else {
                        Ok(Response::builder()
                            .status(404)
                            .body(Full::new(Bytes::from("Not Found")))
                            .unwrap())
                    }
                }
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                tracing::debug!("Metrics server connection error: {}", e);
            }
        });
    }
}
