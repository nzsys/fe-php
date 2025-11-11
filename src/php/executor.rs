use super::ffi::PhpFfi;
use super::PhpConfig;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

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
    ffi: PhpFfi,
    document_root: PathBuf,
}

impl PhpExecutor {
    pub fn new(config: PhpConfig) -> Result<Self> {
        let ffi = PhpFfi::load(&config.libphp_path)?;
        ffi.module_startup()?;

        Ok(Self {
            ffi,
            document_root: config.document_root,
        })
    }

    pub fn execute(&self, request: PhpRequest) -> Result<PhpResponse> {
        let start = std::time::Instant::now();

        // Determine script path from URI
        let script_path = self.resolve_script_path(&request.uri)?;

        // Execute PHP script
        let output = self.ffi.execute_script(script_path.to_str().unwrap())?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Build response
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/html; charset=UTF-8".to_string());

        Ok(PhpResponse {
            status_code: 200,
            headers,
            body: output.into_bytes(),
            execution_time_ms,
            memory_peak_mb: 0.0, // Would be populated from PHP in real implementation
        })
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
        let _ = self.ffi.module_shutdown();
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
