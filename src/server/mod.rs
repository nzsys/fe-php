pub mod router;
pub mod middleware;
pub mod multiprocess;

use crate::config::Config;
use crate::php::{WorkerPool, WorkerPoolConfig, PhpConfig};
use crate::metrics::MetricsCollector;
use crate::tls::TlsManager;
use crate::geoip::GeoIpManager;
use crate::redis_session::RedisSessionManager;
use crate::tracing_telemetry::TracingManager;
use crate::load_balancing::LoadBalancingManager;
use crate::deployment::DeploymentManager;
use anyhow::{Context, Result};
use hyper::server::conn::{http1, http2};
use hyper::service::service_fn;
use hyper::{Request, Response, body::Incoming};
use hyper_util::rt::TokioIo;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::{info, error, warn, debug};

#[derive(Clone)]
pub struct Server {
    config: Arc<Config>,
    worker_pool: Arc<WorkerPool>,
    metrics: Arc<MetricsCollector>,
    tls_manager: Option<Arc<TlsManager>>,
    geoip_manager: Option<Arc<GeoIpManager>>,
    redis_manager: Option<Arc<tokio::sync::RwLock<RedisSessionManager>>>,
    load_balancer: Option<Arc<LoadBalancingManager>>,
    deployment_manager: Option<Arc<DeploymentManager>>,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self> {
        // Use server.workers as the authoritative worker count
        // This fixes the confusion between server.workers and php.worker_pool_size
        let actual_worker_count = config.server.workers;

        info!("Configuring {} PHP worker(s)", actual_worker_count);

        let php_config = PhpConfig {
            libphp_path: config.php.libphp_path.clone(),
            document_root: config.php.document_root.clone(),
            worker_pool_size: actual_worker_count,  // Use server.workers
            worker_max_requests: config.php.worker_max_requests,
            use_fpm: config.php.use_fpm,
            fpm_socket: config.php.fpm_socket.clone(),
        };

        let pool_config = WorkerPoolConfig {
            pool_size: actual_worker_count,  // Use server.workers
            max_requests: config.php.worker_max_requests,
        };

        let worker_pool = WorkerPool::new(php_config, pool_config)?;
        let metrics = MetricsCollector::new();

        // Initialize TLS if enabled
        let tls_manager = if config.tls.enable {
            let cert_path = config.tls.cert_path.as_ref()
                .context("TLS enabled but cert_path not specified")?;
            let key_path = config.tls.key_path.as_ref()
                .context("TLS enabled but key_path not specified")?;

            let tls = TlsManager::new(cert_path, key_path)
                .context("Failed to initialize TLS")?;
            info!("TLS/SSL termination enabled");
            Some(Arc::new(tls))
        } else {
            None
        };

        // Initialize GeoIP if enabled
        let geoip_manager = if config.geoip.enable {
            let db_path = config.geoip.database_path.as_ref()
                .context("GeoIP enabled but database_path not specified")?;

            let geoip = GeoIpManager::new(
                db_path,
                config.geoip.allowed_countries.clone(),
                config.geoip.blocked_countries.clone(),
            ).context("Failed to initialize GeoIP")?;
            info!("GeoIP filtering enabled");
            Some(Arc::new(geoip))
        } else {
            None
        };

        // Initialize Redis if enabled
        let redis_manager = if config.redis.enable {
            let redis = RedisSessionManager::new(
                &config.redis.url,
                config.redis.key_prefix.clone(),
                config.redis.timeout_ms,
            ).await.context("Failed to initialize Redis")?;
            info!("Redis session storage enabled");
            Some(Arc::new(tokio::sync::RwLock::new(redis)))
        } else {
            None
        };

        // Initialize distributed tracing if enabled
        if config.tracing.enable {
            let _tracing = TracingManager::new(
                &config.tracing.otlp_endpoint,
                &config.tracing.service_name,
                config.tracing.sample_rate,
            ).context("Failed to initialize distributed tracing")?;
            info!("Distributed tracing (OpenTelemetry) enabled");
        }

        // Initialize load balancing if enabled
        let load_balancer = if config.load_balancing.enable {
            let lb = LoadBalancingManager::new(
                config.load_balancing.upstreams.clone(),
                config.load_balancing.algorithm,
                &config.load_balancing.circuit_breaker,
            ).context("Failed to initialize load balancing")?;

            // Start health checks
            lb.start_health_checks(config.load_balancing.health_check.clone()).await;

            info!("Load balancing enabled with {} upstreams", config.load_balancing.upstreams.len());
            Some(Arc::new(lb))
        } else {
            None
        };

        // Initialize deployment (A/B testing or canary) if enabled
        let deployment_manager = if config.deployment.enable {
            let dm = DeploymentManager::new(&config.deployment)
                .context("Failed to initialize deployment manager")?;

            info!(
                "Deployment strategy '{}' enabled with {} variants",
                config.deployment.strategy,
                config.deployment.variants.len()
            );

            Some(Arc::new(dm))
        } else {
            None
        };

        Ok(Self {
            config: Arc::new(config),
            worker_pool: Arc::new(worker_pool),
            metrics: Arc::new(metrics),
            tls_manager,
            geoip_manager,
            redis_manager,
            load_balancer,
            deployment_manager,
        })
    }

    pub async fn serve(self) -> Result<()> {
        let addr_str = format!("{}:{}", self.config.server.host, self.config.server.port);

        // Resolve hostname to socket address (supports both IP addresses and hostnames like "localhost")
        let addr: SocketAddr = addr_str.to_socket_addrs()
            .with_context(|| format!("Failed to resolve address: '{}' (host: '{}', port: {})",
                addr_str, self.config.server.host, self.config.server.port))?
            .next()
            .with_context(|| format!("No addresses resolved for: '{}'", addr_str))?;

        let listener = TcpListener::bind(addr).await
            .with_context(|| format!("Failed to bind to address: {}", addr))?;

        let protocol = if self.tls_manager.is_some() { "https" } else { "http" };
        info!("Server listening on {}://{}", protocol, addr);

        if self.config.server.enable_http2 {
            info!("HTTP/2 support enabled");
        }

        let server = Arc::new(self);

        // Create TLS acceptor if TLS is enabled
        let tls_acceptor = server.tls_manager.as_ref().map(|tls| {
            TlsAcceptor::from(tls.server_config())
        });

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let server = Arc::clone(&server);
                    let tls_acceptor = tls_acceptor.clone();

                    tokio::spawn(async move {
                        // Check GeoIP filtering
                        if let Some(ref geoip) = server.geoip_manager {
                            match geoip.is_allowed(remote_addr.ip()) {
                                Ok(false) => {
                                    debug!("Blocked connection from {} due to GeoIP rules", remote_addr);
                                    return;
                                }
                                Err(e) => {
                                    warn!("GeoIP check error for {}: {}", remote_addr, e);
                                    // Continue on error to avoid blocking legitimate traffic
                                }
                                Ok(true) => {}
                            }
                        }

                        // Handle TLS handshake if enabled
                        if let Some(acceptor) = tls_acceptor {
                            match acceptor.accept(stream).await {
                                Ok(tls_stream) => {
                                    let io = TokioIo::new(tls_stream);
                                    server.serve_connection(io, remote_addr).await;
                                }
                                Err(e) => {
                                    error!("TLS handshake failed for {}: {}", remote_addr, e);
                                }
                            }
                        } else {
                            let io = TokioIo::new(stream);
                            server.serve_connection(io, remote_addr).await;
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn serve_connection<I>(&self, io: I, remote_addr: SocketAddr)
    where
        I: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    {
        let server = Arc::new(self.clone());

        let service = service_fn(move |req: Request<Incoming>| {
            let server = Arc::clone(&server);
            async move {
                server.handle_request(req, remote_addr).await
            }
        });

        // Use HTTP/2 if enabled, otherwise HTTP/1.1
        if self.config.server.enable_http2 {
            if let Err(err) = http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                error!("Error serving HTTP/2 connection: {}", err);
            }
        } else {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                error!("Error serving HTTP/1.1 connection: {}", err);
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
