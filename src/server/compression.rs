use brotli::enc::BrotliEncoderParams;
use flate2::write::GzEncoder;
use flate2::Compression;
use hyper::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{Request, Response};
use std::io::Write;

/// Supported compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    Gzip,
    Brotli,
    None,
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub enable_gzip: bool,
    pub enable_brotli: bool,
    pub min_size: usize,
    pub gzip_level: u32,
    pub brotli_quality: u32,
    pub compressible_types: Vec<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enable_gzip: true,
            enable_brotli: true,
            min_size: 1024, // Don't compress files smaller than 1KB
            gzip_level: 6,  // Default gzip compression level
            brotli_quality: 6, // Default brotli quality level
            compressible_types: vec![
                "text/html".to_string(),
                "text/css".to_string(),
                "text/javascript".to_string(),
                "text/xml".to_string(),
                "text/plain".to_string(),
                "application/javascript".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
                "application/xhtml+xml".to_string(),
                "image/svg+xml".to_string(),
            ],
        }
    }
}

impl CompressionConfig {
    /// Check if content type should be compressed
    pub fn should_compress(&self, content_type: &str, size: usize) -> bool {
        if size < self.min_size {
            return false;
        }

        // Extract base content type (remove charset, etc.)
        let base_type = content_type.split(';').next().unwrap_or(content_type).trim();

        self.compressible_types.iter().any(|t| t == base_type)
    }

    /// Determine best compression algorithm based on Accept-Encoding header
    pub fn select_algorithm<T>(&self, request: &Request<T>) -> CompressionAlgorithm {
        let accept_encoding = request
            .headers()
            .get(ACCEPT_ENCODING)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Brotli is preferred if available (better compression ratio)
        if self.enable_brotli && accept_encoding.contains("br") {
            return CompressionAlgorithm::Brotli;
        }

        // Fallback to gzip
        if self.enable_gzip && accept_encoding.contains("gzip") {
            return CompressionAlgorithm::Gzip;
        }

        CompressionAlgorithm::None
    }

    /// Compress data with the specified algorithm
    pub fn compress(&self, data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>, std::io::Error> {
        match algorithm {
            CompressionAlgorithm::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.gzip_level));
                encoder.write_all(data)?;
                encoder.finish()
            }
            CompressionAlgorithm::Brotli => {
                let mut output = Vec::new();
                let params = BrotliEncoderParams {
                    quality: self.brotli_quality as i32,
                    ..Default::default()
                };
                brotli::BrotliCompress(
                    &mut std::io::Cursor::new(data),
                    &mut output,
                    &params,
                )?;
                Ok(output)
            }
            CompressionAlgorithm::None => Ok(data.to_vec()),
        }
    }

    /// Apply compression to response if applicable
    pub fn compress_response<T, B>(
        &self,
        request: &Request<T>,
        response: Response<B>,
        body: Vec<u8>,
    ) -> Result<Response<Vec<u8>>, std::io::Error>
    where
        B: Send,
    {
        // Get content type
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Check if should compress
        if !self.should_compress(content_type, body.len()) {
            let (parts, _) = response.into_parts();
            return Ok(Response::from_parts(parts, body));
        }

        // Select algorithm
        let algorithm = self.select_algorithm(request);
        if algorithm == CompressionAlgorithm::None {
            let (parts, _) = response.into_parts();
            return Ok(Response::from_parts(parts, body));
        }

        // Compress
        let compressed = self.compress(&body, algorithm)?;

        // Check if compression was beneficial
        if compressed.len() >= body.len() {
            // Compression increased size, don't use it
            let (parts, _) = response.into_parts();
            return Ok(Response::from_parts(parts, body));
        }

        // Update headers
        let (mut parts, _) = response.into_parts();
        parts.headers.insert(
            CONTENT_ENCODING,
            match algorithm {
                CompressionAlgorithm::Gzip => "gzip".parse().unwrap(),
                CompressionAlgorithm::Brotli => "br".parse().unwrap(),
                CompressionAlgorithm::None => unreachable!(),
            },
        );
        parts.headers.insert(
            CONTENT_LENGTH,
            compressed.len().to_string().parse().unwrap(),
        );

        Ok(Response::from_parts(parts, compressed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_compress() {
        let config = CompressionConfig::default();

        // Should compress HTML
        assert!(config.should_compress("text/html", 2048));

        // Should not compress small files
        assert!(!config.should_compress("text/html", 512));

        // Should not compress images
        assert!(!config.should_compress("image/jpeg", 2048));
    }

    #[test]
    fn test_select_algorithm() {
        let config = CompressionConfig::default();
        let request = Request::builder()
            .header("Accept-Encoding", "gzip, br")
            .body(())
            .unwrap();

        // Brotli should be preferred
        assert_eq!(config.select_algorithm(&request), CompressionAlgorithm::Brotli);
    }

    #[test]
    fn test_compression() {
        let config = CompressionConfig::default();
        let data = b"Hello, World! This is a test string that should compress well.";

        // Test gzip
        let gzipped = config.compress(data, CompressionAlgorithm::Gzip).unwrap();
        assert!(gzipped.len() < data.len());

        // Test brotli
        let brotlied = config.compress(data, CompressionAlgorithm::Brotli).unwrap();
        assert!(brotlied.len() < data.len());
    }
}
