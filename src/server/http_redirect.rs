use anyhow::Result;
use hyper::{Request, Response, StatusCode, body::Incoming};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, debug};

/// HTTP to HTTPS redirect server
pub struct HttpRedirectServer {
    http_port: u16,
    https_port: u16,
}

impl HttpRedirectServer {
    pub fn new(http_port: u16, https_port: u16) -> Self {
        Self {
            http_port,
            https_port,
        }
    }

    pub async fn serve(self) -> Result<()> {
        let addr: SocketAddr = ([0, 0, 0, 0], self.http_port).into();
        let listener = TcpListener::bind(addr).await?;

        info!("HTTP redirect server listening on port {} → redirecting to HTTPS port {}",
            self.http_port, self.https_port);

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let https_port = self.https_port;

                    tokio::spawn(async move {
                        let io = hyper_util::rt::TokioIo::new(stream);

                        let service = hyper::service::service_fn(move |req: Request<Incoming>| {
                            async move {
                                handle_redirect(req, https_port, remote_addr).await
                            }
                        });

                        if let Err(err) = hyper::server::conn::http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            debug!("Error serving HTTP redirect connection: {}", err);
                        }
                    });
                }
                Err(e) => {
                    debug!("Failed to accept HTTP connection: {}", e);
                }
            }
        }
    }
}

async fn handle_redirect(
    req: Request<Incoming>,
    https_port: u16,
    remote_addr: SocketAddr,
) -> Result<Response<String>> {
    let host = req.headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    // Remove port from host if present
    let host_without_port = host.split(':').next().unwrap_or(host);

    // Build HTTPS URL
    let https_url = if https_port == 443 {
        format!("https://{}{}", host_without_port, req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/"))
    } else {
        format!("https://{}:{}{}", host_without_port, https_port, req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/"))
    };

    debug!("Redirecting {} from {} to {}", req.uri(), remote_addr, https_url);

    Ok(Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header("Location", https_url)
        .body(String::new())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redirect_url_construction() {
        // Test would need actual request objects
        // Placeholder for future implementation
    }
}
