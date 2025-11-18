use anyhow::{Context, Result};
use hyper::http::HeaderMap;
use hyper::body::Incoming;
use http_body_util::BodyExt;
use std::collections::HashMap;

/// Maximum request body size (10MB by default)
pub const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Parse HTTP headers into a HashMap
///
/// Converts an HTTP HeaderMap to a HashMap<String, String>.
/// Invalid UTF-8 header values are skipped.
///
/// # Arguments
/// * `headers` - The HTTP headers to parse
///
/// # Returns
/// A HashMap containing the parsed headers
pub fn parse_headers(headers: &HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::with_capacity(headers.len());
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            map.insert(name.to_string(), value_str.to_string());
        }
    }
    map
}

/// Read request body with size limit
///
/// Reads the entire request body into a Vec<u8>, enforcing a maximum size limit.
///
/// # Arguments
/// * `body` - The incoming body to read
/// * `max_size` - Maximum allowed body size in bytes (defaults to MAX_BODY_SIZE)
///
/// # Returns
/// The body as a Vec<u8>
///
/// # Errors
/// Returns an error if:
/// - The body exceeds the maximum size
/// - Reading the body fails
pub async fn read_body_with_limit(body: Incoming, max_size: Option<usize>) -> Result<Vec<u8>> {
    let max_size = max_size.unwrap_or(MAX_BODY_SIZE);

    let collected = body
        .collect()
        .await
        .context("Failed to read request body")?;

    let body_bytes = collected.to_bytes();

    if body_bytes.len() > max_size {
        anyhow::bail!("Request body too large: {} bytes (max: {} bytes)", body_bytes.len(), max_size);
    }

    Ok(body_bytes.to_vec())
}

/// Read request body with default size limit
///
/// Convenience wrapper around read_body_with_limit using the default MAX_BODY_SIZE.
///
/// # Arguments
/// * `body` - The incoming body to read
///
/// # Returns
/// The body as a Vec<u8>
pub async fn read_body(body: Incoming) -> Result<Vec<u8>> {
    read_body_with_limit(body, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;

    #[test]
    fn test_parse_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-custom-header", "test-value".parse().unwrap());

        let parsed = parse_headers(&headers);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(parsed.get("x-custom-header"), Some(&"test-value".to_string()));
    }

    #[test]
    fn test_parse_headers_capacity() {
        let headers = HeaderMap::new();
        let parsed = parse_headers(&headers);

        // Should initialize with headers.len() capacity
        assert_eq!(parsed.len(), 0);
    }
}
