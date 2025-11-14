use hyper::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, RANGE};
use hyper::{Request, Response, StatusCode};
use std::cmp;

/// HTTP Range request parser and handler
pub struct RangeHandler;

/// Parsed range specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64, // inclusive
}

impl ByteRange {
    /// Get the length of this range
    pub fn len(&self) -> u64 {
        self.end - self.start + 1
    }

    /// Check if range is empty
    pub fn is_empty(&self) -> bool {
        self.start > self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeSpec {
    /// Single range (e.g., "bytes=0-1023")
    Single(ByteRange),
    /// Multiple ranges (not commonly supported, we'll reject these)
    Multiple(Vec<ByteRange>),
    /// Suffix range (e.g., "bytes=-500" for last 500 bytes)
    Suffix(u64),
}

impl RangeHandler {
    /// Parse Range header from request
    pub fn parse_range<T>(request: &Request<T>, total_size: u64) -> Option<RangeSpec> {
        let range_header = request.headers().get(RANGE)?.to_str().ok()?;

        // Must start with "bytes="
        if !range_header.starts_with("bytes=") {
            return None;
        }

        let range_str = &range_header[6..]; // Skip "bytes="

        // Handle multiple ranges (comma-separated)
        let parts: Vec<&str> = range_str.split(',').collect();
        if parts.len() > 1 {
            // Multiple ranges - parse all
            let ranges: Vec<ByteRange> = parts
                .iter()
                .filter_map(|p| Self::parse_single_range(p.trim(), total_size))
                .collect();
            return Some(RangeSpec::Multiple(ranges));
        }

        // Single range
        let range_str = parts[0].trim();

        // Handle suffix range (e.g., "-500")
        if range_str.starts_with('-') {
            if let Ok(suffix) = range_str[1..].parse::<u64>() {
                return Some(RangeSpec::Suffix(suffix));
            }
            return None;
        }

        // Normal range (e.g., "0-1023")
        Self::parse_single_range(range_str, total_size).map(RangeSpec::Single)
    }

    fn parse_single_range(range_str: &str, total_size: u64) -> Option<ByteRange> {
        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() != 2 {
            return None;
        }

        let start = parts[0].parse::<u64>().ok()?;
        let end = if parts[1].is_empty() {
            // Open-ended range (e.g., "500-")
            total_size - 1
        } else {
            parts[1].parse::<u64>().ok()?
        };

        // Validate range
        if start > end || start >= total_size {
            return None;
        }

        Some(ByteRange {
            start,
            end: cmp::min(end, total_size - 1),
        })
    }

    /// Convert RangeSpec to a concrete ByteRange
    pub fn resolve_range(spec: &RangeSpec, total_size: u64) -> Option<ByteRange> {
        match spec {
            RangeSpec::Single(range) => Some(*range),
            RangeSpec::Suffix(suffix) => {
                let start = total_size.saturating_sub(*suffix);
                Some(ByteRange {
                    start,
                    end: total_size - 1,
                })
            }
            RangeSpec::Multiple(_) => None, // We don't support multipart ranges
        }
    }

    /// Create a 206 Partial Content response
    pub fn create_partial_response(
        range: ByteRange,
        total_size: u64,
        content_type: &str,
        data: Vec<u8>,
    ) -> Response<Vec<u8>> {
        let response = Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header("Content-Type", content_type)
            .header(CONTENT_LENGTH, range.len().to_string())
            .header(
                CONTENT_RANGE,
                format!("bytes {}-{}/{}", range.start, range.end, total_size),
            )
            .header(ACCEPT_RANGES, "bytes")
            .body(data)
            .unwrap();

        response
    }

    /// Create a 416 Range Not Satisfiable response
    pub fn create_range_not_satisfiable(total_size: u64) -> Response<Vec<u8>> {
        Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(CONTENT_RANGE, format!("bytes */{}", total_size))
            .body(Vec::new())
            .unwrap()
    }

    /// Handle range request for file data
    pub fn handle_range_request<T>(
        request: &Request<T>,
        file_data: &[u8],
        content_type: &str,
    ) -> Response<Vec<u8>> {
        let total_size = file_data.len() as u64;

        // Parse range header
        let range_spec = match Self::parse_range(request, total_size) {
            Some(spec) => spec,
            None => {
                // No valid range, return full content
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", content_type)
                    .header(CONTENT_LENGTH, total_size.to_string())
                    .header(ACCEPT_RANGES, "bytes")
                    .body(file_data.to_vec())
                    .unwrap();
            }
        };

        // We don't support multipart ranges
        if matches!(range_spec, RangeSpec::Multiple(_)) {
            return Self::create_range_not_satisfiable(total_size);
        }

        // Resolve range
        let range = match Self::resolve_range(&range_spec, total_size) {
            Some(r) => r,
            None => return Self::create_range_not_satisfiable(total_size),
        };

        // Validate range
        if range.is_empty() || range.end >= total_size {
            return Self::create_range_not_satisfiable(total_size);
        }

        // Extract range data
        let range_data = file_data[range.start as usize..=range.end as usize].to_vec();

        Self::create_partial_response(range, total_size, content_type, range_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_range() {
        let req = Request::builder()
            .header("Range", "bytes=0-999")
            .body(())
            .unwrap();

        let spec = RangeHandler::parse_range(&req, 10000).unwrap();
        assert_eq!(
            spec,
            RangeSpec::Single(ByteRange { start: 0, end: 999 })
        );
    }

    #[test]
    fn test_parse_open_ended_range() {
        let req = Request::builder()
            .header("Range", "bytes=500-")
            .body(())
            .unwrap();

        let spec = RangeHandler::parse_range(&req, 1000).unwrap();
        assert_eq!(
            spec,
            RangeSpec::Single(ByteRange { start: 500, end: 999 })
        );
    }

    #[test]
    fn test_parse_suffix_range() {
        let req = Request::builder()
            .header("Range", "bytes=-500")
            .body(())
            .unwrap();

        let spec = RangeHandler::parse_range(&req, 1000).unwrap();
        assert_eq!(spec, RangeSpec::Suffix(500));

        let range = RangeHandler::resolve_range(&spec, 1000).unwrap();
        assert_eq!(range, ByteRange { start: 500, end: 999 });
    }

    #[test]
    fn test_handle_range_request() {
        let data = b"Hello, World! This is a test.";
        let req = Request::builder()
            .header("Range", "bytes=0-4")
            .body(())
            .unwrap();

        let response = RangeHandler::handle_range_request(&req, data, "text/plain");
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);

        let body = response.body();
        assert_eq!(body, b"Hello");
    }

    #[test]
    fn test_invalid_range() {
        let data = b"Hello, World!";
        let req = Request::builder()
            .header("Range", "bytes=100-200")
            .body(())
            .unwrap();

        let response = RangeHandler::handle_range_request(&req, data, "text/plain");
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    }

    #[test]
    fn test_byte_range_len() {
        let range = ByteRange { start: 0, end: 999 };
        assert_eq!(range.len(), 1000);

        let range = ByteRange { start: 500, end: 500 };
        assert_eq!(range.len(), 1);
    }
}
