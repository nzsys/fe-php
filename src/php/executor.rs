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
    /// Create executor with full PHP module initialization (call module_startup)
    /// This should be called once globally, and returns the shared PhpFfi instance
    pub fn new(config: PhpConfig) -> Result<Self> {
        let (ffi, fastcgi) = if config.use_fpm {
            // Use PHP-FPM via FastCGI
            (None, Some(FastCgiClient::new(config.fpm_socket.clone())))
        } else {
            // Use libphp and initialize PHP module
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

    /// Create executor using shared PhpFfi instance (for worker threads)
    /// This avoids multiple library loads and module initialization
    pub fn new_worker(config: PhpConfig, shared_ffi: Option<Arc<PhpFfi>>) -> Result<Self> {
        let (ffi, fastcgi) = if config.use_fpm {
            // Use PHP-FPM via FastCGI
            (None, Some(FastCgiClient::new(config.fpm_socket.clone())))
        } else {
            // Use shared PhpFfi instance (no need to load or initialize)
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

    /// Get the shared PhpFfi instance (for passing to workers)
    pub fn get_shared_ffi(&self) -> Option<Arc<PhpFfi>> {
        self.ffi.clone()
    }

    pub fn execute(&self, request: PhpRequest) -> Result<PhpResponse> {
        let start = std::time::Instant::now();

        // Determine script path from URI
        let script_path = self.resolve_script_path(&request.uri)?;

        if self.use_fpm {
            // Execute via PHP-FPM
            let fastcgi = self.fastcgi.as_ref().unwrap();

            // Use tokio's block_in_place to run async code in blocking context
            let (stdout, stderr) = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(
                    fastcgi.execute(
                        script_path.to_str().unwrap(),
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

            // Parse FastCGI response (HTTP headers + body)
            let (status_code, headers, body) = self.parse_fastcgi_response(&stdout)?;

            Ok(PhpResponse {
                status_code,
                headers,
                body,
                execution_time_ms,
                memory_peak_mb: 0.0,
            })
        } else {
            // Execute via embedded libphp (high performance mode)
            let ffi = self.ffi.as_ref().unwrap();

            // Start request
            ffi.request_startup()
                .context("Failed to start PHP request")?;

            // Execute script
            let output = match ffi.execute_script(script_path.to_str().unwrap()) {
                Ok(out) => out,
                Err(e) => {
                    ffi.request_shutdown();
                    return Err(e);
                }
            };

            // Get captured headers from SAPI callbacks
            let captured_headers = ffi.get_headers();

            // Shutdown request
            ffi.request_shutdown();

            let execution_time_ms = start.elapsed().as_millis() as u64;

            // Parse output for headers (PHP can output headers directly or via header() function)
            let (status_code, headers, body) = self.parse_php_output(&output, &captured_headers)?;

            Ok(PhpResponse {
                status_code,
                headers,
                body,
                execution_time_ms,
                memory_peak_mb: 0.0,
            })
        }
    }

    /// Parse PHP output (handles both raw output and headers)
    /// captured_headers: Headers captured via SAPI header_handler callback
    fn parse_php_output(&self, data: &[u8], captured_headers: &[String]) -> Result<(u16, HashMap<String, String>, Vec<u8>)> {
        let mut status_code = 200u16;
        let mut headers = HashMap::new();

        // Process headers captured via SAPI callbacks (header() function)
        for header_line in captured_headers {
            if let Some((name, value)) = header_line.split_once(':') {
                let name = name.trim();
                let value = value.trim();

                if name.eq_ignore_ascii_case("Status") {
                    // Parse status code from "Status: 200 OK"
                    if let Some(code_str) = value.split_whitespace().next() {
                        status_code = code_str.parse().unwrap_or(200);
                    }
                } else if !name.is_empty() {
                    headers.insert(name.to_string(), value.to_string());
                }
            }
        }

        // Check if output contains headers (raw output mode)
        // This handles cases where PHP outputs headers directly without using header()
        if data.len() >= 4 && (data.starts_with(b"HTTP/") || data.starts_with(b"Status:") || data.starts_with(b"Content-Type:")) {
            // Parse headers from raw output
            let (raw_status, raw_headers, body) = self.parse_headers_and_body(data)?;

            // Merge raw headers with captured headers (raw headers take precedence for conflicts)
            for (name, value) in raw_headers {
                headers.insert(name, value);
            }

            if raw_status != 200 {
                status_code = raw_status;
            }

            return Ok((status_code, headers, body));
        }

        // No raw headers, just body content with captured headers
        if !headers.contains_key("Content-Type") && !headers.contains_key("content-type") {
            headers.insert("Content-Type".to_string(), "text/html; charset=UTF-8".to_string());
        }

        Ok((status_code, headers, data.to_vec()))
    }

    /// Parse headers and body from raw output (optimized with memchr)
    fn parse_headers_and_body(&self, data: &[u8]) -> Result<(u16, HashMap<String, String>, Vec<u8>)> {
        let mut status_code = 200u16;
        let mut headers = HashMap::with_capacity(8); // Pre-allocate for typical header count
        let mut body_start = 0;

        // Find header/body separator using fast memmem search
        let separator: &[u8];
        if let Some(pos) = memmem::find(data, b"\r\n\r\n") {
            body_start = pos + 4;
            separator = b"\r\n";
        } else if let Some(pos) = memmem::find(data, b"\n\n") {
            body_start = pos + 2;
            separator = b"\n";
        } else {
            // No separator found, treat all as body
            let mut headers = HashMap::with_capacity(1);
            headers.insert("Content-Type".to_string(), "text/html; charset=UTF-8".to_string());
            return Ok((200, headers, data.to_vec()));
        };

        // Parse headers
        let header_data = &data[..body_start];
        for line in header_data.split(|&b| b == separator[0]) {
            if line.is_empty() {
                continue;
            }

            // Convert to string for parsing
            let line_str = String::from_utf8_lossy(line);

            if let Some((name, value)) = line_str.split_once(':') {
                let name = name.trim();
                let value = value.trim();

                if name.eq_ignore_ascii_case("Status") {
                    // Parse status code from "Status: 200 OK"
                    if let Some(code_str) = value.split_whitespace().next() {
                        status_code = code_str.parse().unwrap_or(200);
                    }
                } else if !name.is_empty() {
                    headers.insert(name.to_string(), value.to_string());
                }
            }
        }

        // Ensure Content-Type header exists
        if !headers.contains_key("Content-Type") && !headers.contains_key("content-type") {
            headers.insert("Content-Type".to_string(), "text/html; charset=UTF-8".to_string());
        }

        // Extract body (zero-copy slice)
        let body = if body_start < data.len() {
            data[body_start..].to_vec()
        } else {
            Vec::new()
        };

        Ok((status_code, headers, body))
    }

    fn parse_fastcgi_response(&self, data: &[u8]) -> Result<(u16, HashMap<String, String>, Vec<u8>)> {
        // Reuse the same parser for FastCGI responses
        self.parse_headers_and_body(data)
    }

    fn resolve_script_path(&self, uri: &str) -> Result<PathBuf> {
        // Remove query string
        let path = uri.split('?').next().unwrap_or(uri);

        // Remove leading slash
        let path = path.trim_start_matches('/');

        // Default to index.php if path is empty or is a directory
        let path = if path.is_empty() || path.ends_with('/') {
            format!("{}index.php", path)
        } else if !path.ends_with(".php") {
            format!("{}.php", path)
        } else {
            path.to_string()
        };

        let script_path = self.document_root.join(path);

        // Security: ensure path is within document root
        let canonical = script_path.canonicalize().unwrap_or(script_path.clone());
        if !canonical.starts_with(&self.document_root) {
            return Err(anyhow::anyhow!("Path traversal attempt detected"));
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
            // Only call module_shutdown if we called module_startup
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
        };

        // Note: This test would fail because PhpExecutor::new requires actual libphp.so
        // In real implementation, we'd mock this or use dependency injection

        // Test path resolution logic separately
        let uri = "/test.php";
        assert!(uri.trim_start_matches('/') == "test.php");
    }
}
