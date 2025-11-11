pub mod router;
pub mod middleware;

use crate::config::Config;
use crate::php::{WorkerPool, WorkerPoolConfig, PhpConfig};
use crate::metrics::MetricsCollector;
use anyhow::Result;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, body::Incoming};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

pub struct Server {
    config: Arc<Config>,
    worker_pool: Arc<WorkerPool>,
    metrics: Arc<MetricsCollector>,
}

impl Server {
    pub fn new(config: Config) -> Result<Self> {
        let php_config = PhpConfig {
            libphp_path: config.php.libphp_path.clone(),
            document_root: config.php.document_root.clone(),
            worker_pool_size: config.php.worker_pool_size,
            worker_max_requests: config.php.worker_max_requests,
        };

        let pool_config = WorkerPoolConfig {
            pool_size: config.php.worker_pool_size,
            max_requests: config.php.worker_max_requests,
        };

        let worker_pool = WorkerPool::new(php_config, pool_config)?;
        let metrics = MetricsCollector::new();

        Ok(Self {
            config: Arc::new(config),
            worker_pool: Arc::new(worker_pool),
            metrics: Arc::new(metrics),
        })
    }

    pub async fn serve(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse()?;

        let listener = TcpListener::bind(addr).await?;
        info!("Server listening on http://{}", addr);

        let server = Arc::new(self);

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let server = Arc::clone(&server);

                    tokio::spawn(async move {
                        let io = TokioIo::new(stream);

                        let service = service_fn(move |req: Request<Incoming>| {
                            let server = Arc::clone(&server);
                            async move {
                                server.handle_request(req, remote_addr).await
                            }
                        });

                        if let Err(err) = http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            error!("Error serving connection: {}", err);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_request(
        &self,
        req: Request<Incoming>,
        remote_addr: SocketAddr,
    ) -> Result<Response<String>> {
        router::handle_request(
            req,
            remote_addr,
            Arc::clone(&self.worker_pool),
            Arc::clone(&self.metrics),
            Arc::clone(&self.config),
        )
        .await
    }
}
