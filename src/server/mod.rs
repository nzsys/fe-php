pub mod router;
pub mod middleware;
pub mod multiprocess;
pub mod shutdown;
pub mod http_redirect;
pub mod ip_filter;
pub mod cors;
pub mod compression;
pub mod range;
pub mod config_reload;

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
    backend_router: Option<Arc<crate::backend::router::BackendRouter>>,
    metrics: Arc<MetricsCollector>,
    tls_manager: Option<Arc<TlsManager>>,
    geoip_manager: Option<Arc<GeoIpManager>>,
    _redis_manager: Option<Arc<tokio::sync::RwLock<RedisSessionManager>>>,
    _load_balancer: Option<Arc<LoadBalancingManager>>,
    _deployment_manager: Option<Arc<DeploymentManager>>,
    waf_engine: Option<Arc<crate::waf::WafEngine>>,
    shutdown_coordinator: Arc<shutdown::ShutdownCoordinator>,
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

        let worker_pool = Arc::new(WorkerPool::new(php_config.clone(), pool_config)?);
        let metrics = Arc::new(MetricsCollector::new());
        let shutdown_coordinator = Arc::new(shutdown::ShutdownCoordinator::new(30));

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

        // Initialize WAF if enabled
        let waf_engine = if config.waf.enable {
            let rules = crate::waf::rules::default_rules();

            let waf = crate::waf::WafEngine::new(
                rules,
                config.waf.mode.to_string(),
                Arc::clone(&metrics),
            );

            info!("WAF enabled in '{}' mode with {} rules", config.waf.mode, waf.rules_count());
            Some(Arc::new(waf))
        } else {
            None
        };

        // Initialize hybrid backend system if enabled
        let backend_router = if config.backend.enable_hybrid {
            use crate::backend::{Backend, BackendType, embedded::EmbeddedBackend, fastcgi::FastCGIBackend, static_files::StaticBackend};
            use std::collections::HashMap;

            info!("Hybrid backend system enabled");

            let mut backends: HashMap<BackendType, Arc<dyn Backend>> = HashMap::new();

            // Add embedded backend using existing WorkerPool
            // This avoids creating a new PhpExecutor and TSRM issues
            if worker_pool.executor().is_some() {
                backends.insert(
                    BackendType::Embedded,
                    Arc::new(EmbeddedBackend::new(Arc::clone(&worker_pool))),
                );
                info!("Registered embedded backend (libphp)");
            }

            // Add FastCGI backend if FPM is configured
            if config.php.use_fpm || !config.php.fpm_socket.is_empty() {
                backends.insert(
                    BackendType::FastCGI,
                    Arc::new(FastCGIBackend::new(
                        config.php.fpm_socket.clone(),
                        config.php.document_root.clone(),
                    )),
                );
                info!("Registered FastCGI backend (PHP-FPM at {})", config.php.fpm_socket);
            }

            // Add static file backend if enabled
            if config.backend.static_files.enable {
                if let Some(ref static_root) = config.backend.static_files.root {
                    let static_backend = StaticBackend::new(static_root.clone())
                        .with_index_files(config.backend.static_files.index_files.clone());
                    backends.insert(BackendType::Static, Arc::new(static_backend));
                    info!("Registered static file backend (root: {})", static_root.display());
                } else {
                    warn!("Static file backend enabled but no root directory specified");
                }
            }

            // Parse default backend type
            let default_backend = config.backend.default_backend.parse::<BackendType>()
                .with_context(|| format!("Invalid default backend type: {}", config.backend.default_backend))?;

            // Ensure default backend is registered
            if !backends.contains_key(&default_backend) {
                return Err(anyhow::anyhow!(
                    "Default backend '{}' is not registered. Available backends: {:?}",
                    default_backend,
                    backends.keys().collect::<Vec<_>>()
                ));
            }

            // Create backend router
            let router = crate::backend::router::BackendRouter::new(
                backends,
                config.backend.routing_rules.clone(),
                default_backend,
            )?;

            info!(
                "Backend router initialized with {} rules, default backend: {}",
                config.backend.routing_rules.len(),
                default_backend
            );

            Some(Arc::new(router))
        } else {
            None
        };

        Ok(Self {
            config: Arc::new(config),
            worker_pool,
            backend_router,
            metrics,
            tls_manager,
            geoip_manager,
            _redis_manager: redis_manager,
            _load_balancer: load_balancer,
            _deployment_manager: deployment_manager,
            waf_engine,
            shutdown_coordinator,
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

        // Spawn signal handler for graceful shutdown
        let shutdown_handle = tokio::spawn(shutdown::setup_signal_handler(
            Arc::clone(&server.shutdown_coordinator)
        ));

        // Spawn HTTP redirect server if TLS is enabled with http_redirect
        if server.config.tls.enable && server.config.tls.http_redirect {
            let http_redirect_server = http_redirect::HttpRedirectServer::new(
                server.config.tls.http_port,
                server.config.server.port,
            );

            tokio::spawn(async move {
                if let Err(e) = http_redirect_server.serve().await {
                    warn!("HTTP redirect server error: {}", e);
                }
            });
        }

        // Create TLS acceptor if TLS is enabled
        let tls_acceptor = server.tls_manager.as_ref().map(|tls| {
            TlsAcceptor::from(tls.server_config())
        });

        // Get shutdown receiver
        let mut shutdown_rx = server.shutdown_coordinator.subscribe();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, remote_addr)) => {
                            // Check if shutdown has been initiated
                            if server.shutdown_coordinator.is_shutting_down() {
                                debug!("Rejecting new connection during shutdown from {}", remote_addr);
                                continue;
                            }

                            let server = Arc::clone(&server);
                            let tls_acceptor = tls_acceptor.clone();

                                    // Track connection
                            server.shutdown_coordinator.inc_connections();

                            tokio::spawn(async move {
                                // Check GeoIP filtering
                                if let Some(ref geoip) = server.geoip_manager {
                                    match geoip.is_allowed(remote_addr.ip()) {
                                        Ok(false) => {
                                            debug!("Blocked connection from {} due to GeoIP rules", remote_addr);
                                            server.shutdown_coordinator.dec_connections();
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

                                // Decrement connection counter when done
                                server.shutdown_coordinator.dec_connections();
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }

                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received, stopping listener");
                    break;
                }
            }
        }

        // Wait for signal handler to complete
        let _ = shutdown_handle.await;

        Ok(())
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
        // Check WAF if enabled
        if let Some(ref waf) = self.waf_engine {
            
            use http_body_util::BodyExt;

            // Decompose request first
            let (parts, body) = req.into_parts();

            // Extract information from parts
            let method = parts.method.as_str();
            let uri = parts.uri.to_string();
            let query_string = parts.uri.query().unwrap_or("");

            // Convert headers to HashMap
            let mut headers_map = std::collections::HashMap::new();
            for (key, value) in parts.headers.iter() {
                if let Ok(value_str) = value.to_str() {
                    headers_map.insert(key.to_string(), value_str.to_string());
                }
            }

            // Collect body (for POST requests)
            let body_bytes = body.collect().await
                .map(|collected| collected.to_bytes())
                .unwrap_or_default();

            // Check request against WAF rules
            match waf.check_request(method, &uri, query_string, &headers_map, &body_bytes) {
                crate::waf::WafResult::Block(rule) => {
                    warn!("WAF blocked request from {}: rule {} - {}", remote_addr, rule.id, rule.description);
                    return Ok(Response::builder()
                        .status(403)
                        .body("Forbidden: Request blocked by WAF".to_string())
                        .unwrap());
                }
                crate::waf::WafResult::Allow => {
                    // Reconstruct request from parts and body
                    let req = Request::from_parts(parts, http_body_util::Full::new(body_bytes));

                    // Use hybrid backend router if enabled
                    if let Some(ref backend_router) = self.backend_router {
                        return self.handle_with_backend_router(req, remote_addr, backend_router).await;
                    }

                    return router::handle_request(
                        req,
                        remote_addr,
                        Arc::clone(&self.worker_pool),
                        Arc::clone(&self.metrics),
                        Arc::clone(&self.config),
                    )
                    .await;
                }
            }
        }

        // Use hybrid backend router if enabled
        if let Some(ref backend_router) = self.backend_router {
            return self.handle_with_backend_router(req, remote_addr, backend_router).await;
        }

        router::handle_request(
            req,
            remote_addr,
            Arc::clone(&self.worker_pool),
            Arc::clone(&self.metrics),
            Arc::clone(&self.config),
        )
        .await
    }

    async fn handle_with_backend_router<B>(
        &self,
        req: Request<B>,
        remote_addr: SocketAddr,
        backend_router: &crate::backend::router::BackendRouter,
    ) -> Result<Response<String>>
    where
        B: hyper::body::Body + Send + 'static,
        B::Data: Send,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::fmt::Display,
    {
        use http_body_util::BodyExt;
        use std::collections::HashMap;

        let start = std::time::Instant::now();
        let method = req.method().to_string();
        let uri = req.uri().to_string();

        self.metrics.inc_active_connections();

        // Handle metrics endpoint
        if self.config.metrics.enable && uri == self.config.metrics.endpoint {
            self.metrics.dec_active_connections();
            let metrics_output = crate::metrics::export_metrics()?;
            return Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/plain; version=0.0.4")
                .body(metrics_output)?);
        }

        // Handle health check (enhanced with backend status)
        if uri == "/_health" {
            self.metrics.dec_active_connections();
            return self.handle_health_check(backend_router).await;
        }

        // Convert Hyper request to PhpRequest
        let (parts, body) = req.into_parts();

        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes().to_vec(),
            Err(e) => {
                error!("Failed to read request body: {}", e);
                vec![]
            }
        };

        let mut headers = HashMap::new();
        for (name, value) in parts.headers.iter() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        let query_string = parts.uri.query().unwrap_or("").to_string();

        let php_request = crate::php::PhpRequest {
            method: method.clone(),
            uri: uri.clone(),
            headers,
            body: body_bytes,
            query_string,
            remote_addr: remote_addr.to_string(),
        };

        // Route to appropriate backend
        let path = parts.uri.path();
        let backend = backend_router.route(path);

        debug!("Routing {} to {} backend", path, backend.backend_type());

        // Execute on selected backend
        let php_response = match backend.execute(php_request) {
            Ok(response) => response,
            Err(e) => {
                error!("Backend execution failed: {}", e);
                self.metrics.dec_active_connections();

                let duration = start.elapsed().as_secs_f64();
                self.metrics.record_request(&method, 500, duration);

                return Ok(Response::builder()
                    .status(500)
                    .body(format!("Internal Server Error: {}", e))?);
            }
        };

        let duration = start.elapsed().as_secs_f64();
        self.metrics.record_request(&method, php_response.status_code, duration);
        self.metrics.dec_active_connections();

        info!(
            method = %method,
            uri = %uri,
            status = php_response.status_code,
            duration_ms = php_response.execution_time_ms,
            backend = %backend.backend_type(),
            "Request completed"
        );

        // Build response
        let mut response = Response::builder().status(php_response.status_code);

        for (name, value) in php_response.headers.iter() {
            response = response.header(name, value);
        }

        Ok(response.body(String::from_utf8_lossy(&php_response.body).to_string())?)
    }

    async fn handle_health_check(
        &self,
        backend_router: &crate::backend::router::BackendRouter,
    ) -> Result<Response<String>> {
        use serde_json::json;

        let mut backend_statuses = serde_json::Map::new();
        let mut all_healthy = true;

        for (backend_type, backend) in backend_router.backends() {
            match backend.health_check() {
                Ok(status) => {
                    backend_statuses.insert(
                        backend_type.to_string(),
                        json!({
                            "healthy": status.healthy,
                            "message": status.message,
                            "latency_ms": status.latency.map(|d| d.as_millis()),
                        }),
                    );
                    if !status.healthy {
                        all_healthy = false;
                    }
                }
                Err(e) => {
                    backend_statuses.insert(
                        backend_type.to_string(),
                        json!({
                            "healthy": false,
                            "message": format!("Health check error: {}", e),
                        }),
                    );
                    all_healthy = false;
                }
            }
        }

        let response_body = json!({
            "status": if all_healthy { "healthy" } else { "degraded" },
            "backends": backend_statuses,
        });

        let status_code = if all_healthy { 200 } else { 503 };

        Ok(Response::builder()
            .status(status_code)
            .header("Content-Type", "application/json")
            .body(response_body.to_string())?)
    }
}
