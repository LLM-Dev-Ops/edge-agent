//! TLS configuration using Rustls

use anyhow::{Context, Result};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tracing::info;

/// Load TLS configuration from certificate and key files
pub fn load_tls_config(cert_path: &str, key_path: &str) -> Result<Arc<ServerConfig>> {
    info!(
        cert_path = %cert_path,
        key_path = %key_path,
        "Loading TLS configuration"
    );

    // Load certificates
    let cert_file = File::open(cert_path)
        .with_context(|| format!("Failed to open certificate file: {}", cert_path))?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_chain = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse certificate chain")?;

    if cert_chain.is_empty() {
        anyhow::bail!("No certificates found in {}", cert_path);
    }

    // Load private key
    let key_file = File::open(key_path)
        .with_context(|| format!("Failed to open private key file: {}", key_path))?;
    let mut key_reader = BufReader::new(key_file);
    let mut keys = pkcs8_private_keys(&mut key_reader)
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse private key")?;

    if keys.is_empty() {
        anyhow::bail!("No private keys found in {}", key_path);
    }

    let key = keys.remove(0);

    // Build TLS config
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key.into())
        .context("Failed to build TLS configuration")?;

    info!("TLS configuration loaded successfully");
    Ok(Arc::new(config))
}

/// Create a TLS acceptor from configuration
pub fn create_tls_acceptor(cert_path: &str, key_path: &str) -> Result<TlsAcceptor> {
    let config = load_tls_config(cert_path, key_path)?;
    Ok(TlsAcceptor::from(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_missing_file() {
        let result = load_tls_config("nonexistent.crt", "nonexistent.key");
        assert!(result.is_err());
    }
}
