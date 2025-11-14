use super::ffi::PhpFfi;
use super::fastcgi::FastCgiClient;
use super::PhpConfig;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use memchr::memmem;

#[derive(Debug, Clone)]
pub struct PhpRequest {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub query_string: String,
    pub remote_addr: String,
}

#[derive(Debug)]
pub struct PhpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub execution_time_ms: u64,
    pub memory_peak_mb: f64,
}

pub struct PhpExecutor {
    ffi: Option<Arc<PhpFfi>>,
    fastcgi: Option<FastCgiClient>,
    document_root: PathBuf,
    use_fpm: bool,
    skip_module_lifecycle: bool,  // Skip module_startup/shutdown (already done globally)
}

impl PhpExecutor {
    pub fn new(config: PhpConfig) -> Result<Self> {
        let (ffi, fastcgi) = if config.use_fpm {
            (None, Some(FastCgiClient::new(config.fpm_socket.clone())))
        } else {
            let ffi = PhpFfi::load(&config.libphp_path)?;
            ffi.module_startup()
                .context("PHP module startup failed - check PHP installation and configuration")?;
            (Some(Arc::new(ffi)), None)
        };

        Ok(Self {
            ffi,
            fastcgi,
            document_root: config.document_root,
            use_fpm: config.use_fpm,
            skip_module_lifecycle: false,
        })
    }

    pub fn new_worker(config: PhpConfig, shared_ffi: Option<Arc<PhpFfi>>) -> Result<Self> {
        let (ffi, fastcgi) = if config.use_fpm {
            (None, Some(FastCgiClient::new(config.fpm_socket.clone())))
        } else {
            (shared_ffi, None)
        };

        Ok(Self {
            ffi,
            fastcgi,
            document_root: config.document_root,
            use_fpm: config.use_fpm,
            skip_module_lifecycle: true,
        })
    }

    pub fn get_shared_ffi(&self) -> Option<Arc<PhpFfi>> {
        self.ffi.clone()
    }

    pub fn thread_init(&self) {
        if let Some(ffi) = &self.ffi {
            ffi.thread_init();
        }
    }

    pub fn thread_cleanup(&self) {
        if let Some(ffi) = &self.ffi {
            ffi.thread_cleanup();
        }
    }

    pub fn execute(&self, request: PhpRequest) -> Result<PhpResponse> {
        let start = std::time::Instant::now();

        let script_path = self.resolve_script_path(&request.uri)?;

        if self.use_fpm {
            let fastcgi = self.fastcgi.as_ref()
                .ok_or_else(|| anyhow::anyhow!("FastCGI client not initialized"))?;

            let (stdout, _stderr) = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(
                    fastcgi.execute(
                        script_path.to_str()
                            .ok_or_else(|| anyhow::anyhow!("Script path contains invalid UTF-8"))?,
                        &request.method,
                        &request.uri,
                        &request.query_string,
                        &request.headers,
                        &request.body,
                        &request.remote_addr,
                    )
                )
            })?;

            let execution_time_ms = start.elapsed().as_millis() as u64;

            let (status_code, headers, body) = self.parse_fastcgi_response(&stdout)?;

            Ok(PhpResponse {
                status_code,
                headers,
                body,
                execution_time_ms,
                memory_peak_mb: 0.0,
            })
        } else {
            let ffi = self.ffi.as_ref()
                .ok_or_else(|| anyhow::anyhow!("PHP FFI not initialized"))?;

            ffi.request_startup()
                .context("Failed to start PHP request")?;

            let script_path_str = script_path.to_str()
                .ok_or_else(|| anyhow::anyhow!("Script path contains invalid UTF-8"))?;
            let output = match ffi.execute_script(script_path_str) {
                Ok(out) => out,
                Err(e) => {
                    ffi.request_shutdown();
                    return Err(e);
                }
            };

            ffi.request_shutdown();

            let execution_time_ms = start.elapsed().as_millis() as u64;

            let (status_code, headers, body) = self.parse_php_output(&output)?;

            Ok(PhpResponse {
                status_code,
                headers,
                body,
                execution_time_ms,
                memory_peak_mb: 0.0,
            })
        }
    }

    fn parse_php_output(&self, data: &[u8]) -> Result<(u16, HashMap<String, String>, Vec<u8>)> {
        if data.len() < 4 || !data.starts_with(b"HTTP/") && !data.starts_with(b"Status:") && !data.starts_with(b"Content-Type:") {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "text/html; charset=UTF-8".to_string());
            return Ok((200, headers, data.to_vec()));
        }

        self.parse_headers_and_body(data)
    }

    fn parse_headers_and_body(&self, data: &[u8]) -> Result<(u16, HashMap<String, String>, Vec<u8>)> {
        let mut status_code = 200u16;
        let mut headers = HashMap::with_capacity(8); // Pre-allocate for typical header count

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
                        status_code = code_str.parse().unwrap_or_else(|e| {
                            tracing::warn!("Failed to parse status code '{}': {}, defaulting to 200", code_str, e);
                            200
                        });
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

    fn parse_fastcgi_response(&self, data: &[u8]) -> Result<(u16, HashMap<String, String>, Vec<u8>)> {
        self.parse_headers_and_body(data)
    }

    fn resolve_script_path(&self, uri: &str) -> Result<PathBuf> {
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
            .with_context(|| format!(
                "Failed to canonicalize script path '{}' - file may not exist or insufficient permissions",
                script_path.display()
            ))?;

        if !canonical.starts_with(&self.document_root) {
            return Err(anyhow::anyhow!(
                "Path traversal attempt detected: '{}' is outside document root '{}'",
                canonical.display(),
                self.document_root.display()
            ));
        }

        if !canonical.exists() {
            return Err(anyhow::anyhow!("Script not found: {}", canonical.display()));
        }

        Ok(canonical)
    }
}

impl Drop for PhpExecutor {
    fn drop(&mut self) {
        if let Some(ffi) = &self.ffi {
            if !self.skip_module_lifecycle {
                let _ = ffi.module_shutdown();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_script_path() {
        let config = PhpConfig {
            libphp_path: PathBuf::from("/usr/local/lib/libphp.so"),
            document_root: PathBuf::from("/var/www/html"),
            worker_pool_size: 4,
            worker_max_requests: 1000,
            use_fpm: false,
            fpm_socket: String::from("127.0.0.1:9000"),
        };

        let uri = "/test.php";
        assert!(uri.trim_start_matches('/') == "test.php");
    }
}
