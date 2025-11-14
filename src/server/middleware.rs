// Middleware implementation for request processing
// This module can be extended with various middleware functions

use hyper::Request;
use std::future::Future;
use std::pin::Pin;

pub type MiddlewareResult<T> = Pin<Box<dyn Future<Output = Result<T, anyhow::Error>> + Send>>;

pub trait Middleware {
    fn process(&self, req: Request<Vec<u8>>) -> MiddlewareResult<Request<Vec<u8>>>;
}

// Example: Request ID middleware
pub struct RequestIdMiddleware;

impl Middleware for RequestIdMiddleware {
    fn process(&self, mut req: Request<Vec<u8>>) -> MiddlewareResult<Request<Vec<u8>>> {
        Box::pin(async move {
            let request_id = uuid::Uuid::new_v4().to_string();
            req.headers_mut()
                .insert("X-Request-ID", request_id.parse().unwrap());
            Ok(req)
        })
    }
}

// Example: Logging middleware
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn process(&self, req: Request<Vec<u8>>) -> MiddlewareResult<Request<Vec<u8>>> {
        Box::pin(async move {
            tracing::info!(
                "Incoming request: {} {}",
                req.method(),
                req.uri()
            );
            Ok(req)
        })
    }
}
