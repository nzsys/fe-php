use super::{Backend, BackendError, BackendType, HealthStatus};
use crate::php::{PhpRequest, PhpResponse};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct StaticBackend {
    root: PathBuf,
    index_files: Vec<String>,
}

impl StaticBackend {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            index_files: vec!["index.html".to_string(), "index.htm".to_string()],
        }
    }

    pub fn with_index_files(mut self, index_files: Vec<String>) -> Self {
        self.index_files = index_files;
        self
    }

    fn sanitize_path(&self, uri: &str) -> Result<PathBuf, BackendError> {
        let path = uri.split('?').next().unwrap_or(uri);

        let path = path.trim_start_matches('/');
        let path = urlencoding::decode(path)
            .map_err(|e| BackendError::Other(anyhow::anyhow!("Invalid URL encoding: {}", e)))?;

        let full_path = self.root.join(path.as_ref());

        let canonical = full_path.canonicalize()
            .map_err(|_| BackendError::NotFound(path.to_string()))?;

        if !canonical.starts_with(&self.root) {
            return Err(BackendError::Other(anyhow::anyhow!(
                "Path traversal attempt detected: '{}' is outside root '{}'",
                canonical.display(),
                self.root.display()
            )));
        }

        Ok(canonical)
    }

    fn find_index_file(&self, dir_path: &Path) -> Result<PathBuf, BackendError> {
        for index in &self.index_files {
            let index_path = dir_path.join(index);
            if index_path.exists() && index_path.is_file() {
                return Ok(index_path);
            }
        }

        Err(BackendError::NotFound(format!(
            "No index file found in directory: {}",
            dir_path.display()
        )))
    }

    fn guess_mime_type(&self, path: &Path) -> &'static str {
        match path.extension().and_then(|s| s.to_str()) {
            Some("html") | Some("htm") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("js") => "application/javascript; charset=utf-8",
            Some("json") => "application/json; charset=utf-8",
            Some("xml") => "application/xml; charset=utf-8",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("webp") => "image/webp",
            Some("ico") => "image/x-icon",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            Some("otf") => "font/otf",
            Some("pdf") => "application/pdf",
            Some("txt") => "text/plain; charset=utf-8",
            Some("mp4") => "video/mp4",
            Some("webm") => "video/webm",
            Some("mp3") => "audio/mpeg",
            Some("wav") => "audio/wav",
            Some("zip") => "application/zip",
            Some("gz") => "application/gzip",
            _ => "application/octet-stream",
        }
    }

    fn get_cache_control(&self, path: &Path) -> String {
        match path.extension().and_then(|s| s.to_str()) {
            Some("woff") | Some("woff2") | Some("ttf") | Some("otf") => {
                "public, max-age=31536000, immutable".to_string()
            }
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") | Some("ico") => {
                "public, max-age=86400".to_string()
            }
            Some("css") | Some("js") => {
                "public, max-age=3600".to_string()
            }
            Some("html") | Some("htm") => {
                "no-cache".to_string()
            }
            _ => "public, max-age=600".to_string(),
        }
    }
}

impl Backend for StaticBackend {
    fn execute(&self, request: PhpRequest) -> Result<PhpResponse, BackendError> {
        let start = Instant::now();

        if request.method != "GET" && request.method != "HEAD" {
            return Ok(PhpResponse {
                status_code: 405,
                headers: {
                    let mut h = HashMap::new();
                    h.insert("Allow".to_string(), "GET, HEAD".to_string());
                    h.insert("Content-Type".to_string(), "text/plain".to_string());
                    h
                },
                body: b"Method Not Allowed".to_vec(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                memory_peak_mb: 0.0,
            });
        }

        let mut file_path = self.sanitize_path(&request.uri)?;

        if file_path.is_dir() {
            file_path = self.find_index_file(&file_path)?;
        }

        if !file_path.exists() || !file_path.is_file() {
            return Err(BackendError::NotFound(request.uri.clone()));
        }

        let metadata = std::fs::metadata(&file_path)
            .map_err(|e| BackendError::IoError(e))?;

        let file_size = metadata.len();

        let mime_type = self.guess_mime_type(&file_path);

        let cache_control = self.get_cache_control(&file_path);

        if request.method == "HEAD" {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), mime_type.to_string());
            headers.insert("Content-Length".to_string(), file_size.to_string());
            headers.insert("Cache-Control".to_string(), cache_control);

            return Ok(PhpResponse {
                status_code: 200,
                headers,
                body: Vec::new(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                memory_peak_mb: 0.0,
            });
        }

        let content = std::fs::read(&file_path)
            .map_err(|e| BackendError::IoError(e))?;

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), mime_type.to_string());
        headers.insert("Content-Length".to_string(), content.len().to_string());
        headers.insert("Cache-Control".to_string(), cache_control);

        let etag = format!("\"{:x}-{:x}\"",
            metadata.len(),
            metadata.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        headers.insert("ETag".to_string(), etag);

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(PhpResponse {
            status_code: 200,
            headers,
            body: content,
            execution_time_ms,
            memory_peak_mb: 0.0,
        })
    }

    fn health_check(&self) -> Result<HealthStatus> {
        if self.root.exists() && self.root.is_dir() {
            Ok(HealthStatus::healthy(format!(
                "Static backend is healthy (root: {})",
                self.root.display()
            )))
        } else {
            Ok(HealthStatus::unhealthy(format!(
                "Static backend root directory not found or not readable: {}",
                self.root.display()
            )))
        }
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Static
    }
}
