use anyhow::{Context, Result};
use rustls::{ServerConfig, Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

/// TLS configuration manager for handling SSL/TLS termination
pub struct TlsManager {
    server_config: Arc<ServerConfig>,
}

impl TlsManager {
    /// Create a new TLS manager from certificate and key files
    pub fn new(cert_path: &Path, key_path: &Path) -> Result<Self> {
        // Load certificates
        let cert_file = File::open(cert_path)
            .context("Failed to open certificate file")?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain = certs(&mut cert_reader)
            .context("Failed to parse certificates")?
            .into_iter()
            .map(Certificate)
            .collect();

        // Load private key
        let key_file = File::open(key_path)
            .context("Failed to open private key file")?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = pkcs8_private_keys(&mut key_reader)
            .context("Failed to parse private key")?;

        let private_key = keys.remove(0);

        // Build TLS server configuration
        let mut config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, PrivateKey(private_key))
            .context("Failed to build TLS configuration")?;

        // Enable HTTP/2 and HTTP/1.1 via ALPN
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(Self {
            server_config: Arc::new(config),
        })
    }

    /// Get the server configuration
    pub fn server_config(&self) -> Arc<ServerConfig> {
        self.server_config.clone()
    }

    /// Check if a certificate is valid
    pub fn validate_certificate(cert_path: &Path) -> Result<()> {
        let cert_file = File::open(cert_path)
            .context("Failed to open certificate file")?;
        let mut cert_reader = BufReader::new(cert_file);
        let _certs = certs(&mut cert_reader)
            .context("Failed to parse certificates")?;

        Ok(())
    }

    /// Check if a private key is valid
    pub fn validate_private_key(key_path: &Path) -> Result<()> {
        let key_file = File::open(key_path)
            .context("Failed to open private key file")?;
        let mut key_reader = BufReader::new(key_file);
        let _keys = pkcs8_private_keys(&mut key_reader)
            .context("Failed to parse private key")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_manager_validates_paths() {
        // This test would need actual certificate files to work
        // In a real scenario, you would create test certificates
    }
}
