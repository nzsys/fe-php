use clap::Args;
use anyhow::Result;
use crate::{Config, Server};
use std::path::PathBuf;
use tracing::info;

#[derive(Args)]
pub struct ServeArgs {
    /// Path to configuration file
    #[arg(short, long, default_value = "fe-php.toml")]
    pub config: PathBuf,
}

pub async fn run(args: ServeArgs) -> Result<()> {
    // Load configuration
    let config = Config::from_file(&args.config)?;

    // Initialize logging
    crate::logging::init_logging(&config.logging.level, &config.logging.format)?;

    info!("Starting fe-php server v{}", crate::VERSION);
    info!("Loading configuration from: {}", args.config.display());

    // Validate configuration
    let warnings = config.validate()?;
    for warning in warnings {
        println!("{}", warning);
    }

    // Setup signal handlers
    crate::utils::setup_signal_handlers().await?;

    // Initialize metrics
    crate::metrics::init_metrics();

    // Start metrics exporter if enabled
    if config.metrics.enable {
        let metrics_port = config.metrics.port;
        let metrics_endpoint = config.metrics.endpoint.clone();
        tokio::spawn(async move {
            if let Err(e) = start_metrics_server(metrics_port, &metrics_endpoint).await {
                tracing::error!("Metrics server error: {}", e);
            }
        });
        info!("Metrics endpoint available at http://localhost:{}{}", config.metrics.port, config.metrics.endpoint);
    }

    // Start admin interface if enabled
    if config.admin.enable {
        let admin_port = config.admin.http_port;
        tokio::spawn(async move {
            if let Err(e) = start_admin_server(admin_port).await {
                tracing::error!("Admin server error: {}", e);
            }
        });
        info!("Admin interface available at http://localhost:{}", config.admin.http_port);
    }

    // Create and start server
    let server = Server::new(config).await?;
    info!("Server starting...");

    server.serve().await?;

    Ok(())
}

async fn start_metrics_server(port: u16, endpoint: &str) -> Result<()> {
    use hyper::service::service_fn;
    use hyper::{Request, Response, body::{Incoming, Bytes}};
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;
    use http_body_util::Full;

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    let endpoint_path = endpoint.to_string();

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let endpoint_path = endpoint_path.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req: Request<Incoming>| {
                let endpoint_path = endpoint_path.clone();
                async move {
                    if req.uri().path() == endpoint_path {
                        match crate::metrics::export_metrics() {
                            Ok(metrics) => Ok::<_, hyper::Error>(Response::new(Full::new(Bytes::from(metrics)))),
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

async fn start_admin_server(port: u16) -> Result<()> {
    use hyper::service::service_fn;
    use hyper::{Request, Response, body::{Incoming, Bytes}};
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;
    use http_body_util::Full;

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::spawn(async move {
            let service = service_fn(|req: Request<Incoming>| async move {
                let path = req.uri().path();

                let response_body = match path {
                    "/" => {
                        let html = r#"<!DOCTYPE html>
<html><head><title>fe-php Admin</title></head>
<body>
<h1>fe-php Administration</h1>
<ul>
<li><a href="/status">Server Status</a></li>
<li><a href="/config">Configuration</a></li>
</ul>
</body></html>"#;
                        html.to_string()
                    }
                    "/status" => {
                        format!(r#"{{"status":"running","version":"{}","uptime_seconds":{}}}"#,
                            crate::VERSION,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        )
                    }
                    "/config" => {
                        r#"{"message":"Configuration display not yet implemented"}"#.to_string()
                    }
                    _ => "Not Found".to_string(),
                };

                Ok::<_, hyper::Error>(Response::new(Full::new(Bytes::from(response_body))))
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                tracing::debug!("Admin server connection error: {}", e);
            }
        });
    }
}
