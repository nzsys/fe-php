use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, Context};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};

/// Load TLS certificates and private key from files
pub fn load_tls_config(cert_path: &Path, key_path: &Path) -> Result<Arc<ServerConfig>> {
    // Load certificates
    let cert_file = File::open(cert_path)
        .with_context(|| format!("Failed to open certificate file: {}", cert_path.display()))?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<Certificate> = certs(&mut cert_reader)
        .with_context(|| "Failed to parse certificates")?
        .into_iter()
        .map(Certificate)
        .collect();

    if certs.is_empty() {
        anyhow::bail!("No certificates found in {}", cert_path.display());
    }

    // Load private key
    let key_file = File::open(key_path)
        .with_context(|| format!("Failed to open private key file: {}", key_path.display()))?;
    let mut key_reader = BufReader::new(key_file);
    let mut keys = pkcs8_private_keys(&mut key_reader)
        .with_context(|| "Failed to parse private key")?;

    if keys.is_empty() {
        anyhow::bail!("No private key found in {}", key_path.display());
    }

    let key = PrivateKey(keys.remove(0));

    // Build TLS config
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .with_context(|| "Failed to build TLS configuration")?;

    Ok(Arc::new(config))
}

/// Check if TLS configuration is valid
pub fn validate_tls_config(cert_path: &Path, key_path: &Path) -> Result<()> {
    if !cert_path.exists() {
        anyhow::bail!("Certificate file not found: {}", cert_path.display());
    }

    if !key_path.exists() {
        anyhow::bail!("Private key file not found: {}", key_path.display());
    }

    // Try to load the configuration to validate it
    load_tls_config(cert_path, key_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_missing_cert() {
        let result = validate_tls_config(
            Path::new("/nonexistent/cert.pem"),
            Path::new("/nonexistent/key.pem"),
        );
        assert!(result.is_err());
    }
}
