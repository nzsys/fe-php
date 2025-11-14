use super::{Backend, BackendError, BackendType, HealthStatus};
use crate::php::fastcgi::FastCgiClient;
use crate::php::{PhpRequest, PhpResponse};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub struct FastCGIBackend {
    client: Arc<FastCgiClient>,
    document_root: PathBuf,
}

impl FastCGIBackend {
    pub fn new(fpm_socket: String, document_root: PathBuf) -> Self {
        Self {
            client: Arc::new(FastCgiClient::new(fpm_socket)),
            document_root,
        }
    }

    fn resolve_script_path(&self, uri: &str) -> Result<PathBuf, BackendError> {
        let path = uri.split('?').next().unwrap_or(uri);

        let path = path.trim_start_matches('/');

        let path = if path.is_empty() || path.ends_with('/') {
            format!("{}index.php", path)
        } else if !path.ends_with(".php") {
            format!("{}.php", path)
        } else {
            path.to_string()
        };

        let script_path = self.document_root.join(path);

        let canonical = script_path.canonicalize()
            .map_err(|_| BackendError::NotFound(script_path.display().to_string()))?;

        if !canonical.starts_with(&self.document_root) {
            return Err(BackendError::Other(anyhow::anyhow!(
                "Path traversal attempt detected: '{}' is outside document root '{}'",
                canonical.display(),
                self.document_root.display()
            )));
        }

        if !canonical.exists() {
            return Err(BackendError::NotFound(canonical.display().to_string()));
        }

        Ok(canonical)
    }

    fn parse_fastcgi_response(&self, data: &[u8]) -> Result<(u16, HashMap<String, String>, Vec<u8>), BackendError> {
        use memchr::memmem;

        let mut status_code = 200u16;
        let mut headers = HashMap::with_capacity(8);

        let (separator, body_start) = if let Some(pos) = memmem::find(data, b"\r\n\r\n") {
            (b"\r\n" as &[u8], pos + 4)
        } else if let Some(pos) = memmem::find(data, b"\n\n") {
            (b"\n" as &[u8], pos + 2)
        } else {
            let mut headers = HashMap::with_capacity(1);
            headers.insert("Content-Type".to_string(), "text/html; charset=UTF-8".to_string());
            return Ok((200, headers, data.to_vec()));
        };

        let header_data = &data[..body_start];
        for line in header_data.split(|&b| b == separator[0]) {
            if line.is_empty() {
                continue;
            }

            let line_str = String::from_utf8_lossy(line);

            if let Some((name, value)) = line_str.split_once(':') {
                let name = name.trim();
                let value = value.trim();

                if name.eq_ignore_ascii_case("Status") {
                    if let Some(code_str) = value.split_whitespace().next() {
                        status_code = code_str.parse().unwrap_or(200);
                    }
                } else if !name.is_empty() {
                    headers.insert(name.to_string(), value.to_string());
                }
            }
        }

        if !headers.contains_key("Content-Type") && !headers.contains_key("content-type") {
            headers.insert("Content-Type".to_string(), "text/html; charset=UTF-8".to_string());
        }

        let body = if body_start < data.len() {
            data[body_start..].to_vec()
        } else {
            Vec::new()
        };

        Ok((status_code, headers, body))
    }
}

impl Backend for FastCGIBackend {
    fn execute(&self, request: PhpRequest) -> Result<PhpResponse, BackendError> {
        let start = Instant::now();

        let script_path = self.resolve_script_path(&request.uri)?;

        let (stdout, _stderr) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                self.client.execute(
                    script_path.to_str()
                        .ok_or_else(|| BackendError::Other(anyhow::anyhow!("Script path contains invalid UTF-8")))?,
                    &request.method,
                    &request.uri,
                    &request.query_string,
                    &request.headers,
                    &request.body,
                    &request.remote_addr,
                )
            )
        }).map_err(|e| BackendError::ConnectionFailed(e.to_string()))?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        let (status_code, headers, body) = self.parse_fastcgi_response(&stdout)?;

        Ok(PhpResponse {
            status_code,
            headers,
            body,
            execution_time_ms,
            memory_peak_mb: 0.0,
        })
    }

    fn health_check(&self) -> Result<HealthStatus> {
        let start = Instant::now();

        let check_request = PhpRequest {
            method: "GET".to_string(),
            uri: "/_health.php".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            query_string: String::new(),
            remote_addr: "127.0.0.1".to_string(),
        };

        match self.execute(check_request) {
            Ok(_) => {
                let latency = start.elapsed();
                Ok(HealthStatus::healthy("FastCGI backend is healthy")
                    .with_latency(latency))
            }
            Err(BackendError::NotFound(_)) => {
                let latency = start.elapsed();
                Ok(HealthStatus::healthy("FastCGI backend is reachable (no health check script)")
                    .with_latency(latency))
            }
            Err(e) => {
                Ok(HealthStatus::unhealthy(format!("FastCGI backend error: {}", e)))
            }
        }
    }

    fn backend_type(&self) -> BackendType {
        BackendType::FastCGI
    }
}
