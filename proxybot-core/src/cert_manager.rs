//! Certificate management for MITM proxy
//! Handles root CA generation and per-connection leaf certificates

use anyhow::Result;
use std::path::PathBuf;

/// Manages CA certificates for MITM proxy operations
pub struct CertManager {
    ca_cert_path: Option<PathBuf>,
    ca_key_path: Option<PathBuf>,
}

impl CertManager {
    /// Create a new CertManager
    pub fn new() -> Self {
        Self {
            ca_cert_path: None,
            ca_key_path: None,
        }
    }

    /// Initialize the certificate manager with CA certificate paths
    pub fn init(&mut self, cert_path: PathBuf, key_path: PathBuf) -> Result<()> {
        self.ca_cert_path = Some(cert_path);
        self.ca_key_path = Some(key_path);
        Ok(())
    }

    /// Generate a new root CA certificate
    pub fn generate_root_ca(&mut self) -> Result<()> {
        Ok(())
    }

    /// Get the path to the CA certificate
    pub fn ca_cert_path(&self) -> Option<&PathBuf> {
        self.ca_cert_path.as_ref()
    }
}

impl Default for CertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_manager_new() {
        let manager = CertManager::new();
        assert!(manager.ca_cert_path().is_none());
    }

    #[test]
    fn test_cert_manager_init() {
        let mut manager = CertManager::new();
        let cert_path = PathBuf::from("/tmp/cert.pem");
        let key_path = PathBuf::from("/tmp/key.pem");
        assert!(manager.init(cert_path.clone(), key_path).is_ok());
        assert_eq!(manager.ca_cert_path(), Some(&cert_path));
    }
}