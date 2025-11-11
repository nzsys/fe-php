use crate::config::Config;
use crate::php::{WorkerPool, PhpRequest};
use crate::metrics::MetricsCollector;
use anyhow::Result;
use hyper::{Request, Response, body::Incoming, StatusCode};
use http_body_util::BodyExt;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, error};

pub async fn handle_request(
    req: Request<Incoming>,
    remote_addr: SocketAddr,
    worker_pool: Arc<WorkerPool>,
    metrics: Arc<MetricsCollector>,
    config: Arc<Config>,
) -> Result<Response<String>> {
    let start = std::time::Instant::now();
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    metrics.inc_active_connections();

    // Handle metrics endpoint
    if config.metrics.enable && uri == config.metrics.endpoint {
        metrics.dec_active_connections();
        return handle_metrics().await;
    }

    // Handle health check
    if uri == "/_health" {
        metrics.dec_active_connections();
        return Ok(Response::new("OK".to_string()));
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

    let php_request = PhpRequest {
        method: method.clone(),
        uri: uri.clone(),
        headers,
        body: body_bytes,
        query_string,
        remote_addr: remote_addr.to_string(),
    };

    // Execute PHP
    let php_response = match worker_pool.execute(php_request).await {
        Ok(response) => response,
        Err(e) => {
            error!("PHP execution failed: {}", e);
            metrics.dec_active_connections();

            let duration = start.elapsed().as_secs_f64();
            metrics.record_request(&method, 500, duration);

            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Internal Server Error: {}", e))?);
        }
    };

    let duration = start.elapsed().as_secs_f64();
    metrics.record_request(&method, php_response.status_code, duration);
    metrics.dec_active_connections();

    info!(
        method = %method,
        uri = %uri,
        status = php_response.status_code,
        duration_ms = php_response.execution_time_ms,
        "Request completed"
    );

    // Build response
    let mut response = Response::builder().status(php_response.status_code);

    for (name, value) in php_response.headers.iter() {
        response = response.header(name, value);
    }

    Ok(response.body(String::from_utf8_lossy(&php_response.body).to_string())?)
}

async fn handle_metrics() -> Result<Response<String>> {
    let metrics_output = crate::metrics::export_metrics()?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics_output)?)
}
