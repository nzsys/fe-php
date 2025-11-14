use clap::Args;
use anyhow::Result;
use crate::{Config, Server};
use std::path::PathBuf;
use tracing::info;

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

    // Create server first to get metrics collector
    let server = Server::new(config.clone()).await?;
    let metrics_collector = server.metrics_collector();

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

    if config.admin.enable {
        let admin_host = config.admin.host.clone();
        let admin_port = config.admin.http_port;
        let metrics_for_admin = metrics_collector.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::admin::start_admin_server(admin_host.clone(), admin_port, metrics_for_admin).await {
                tracing::error!("Admin server error: {}", e);
            }
        });
        info!("Admin interface available at http://{}:{}", config.admin.host, config.admin.http_port);
    }

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
