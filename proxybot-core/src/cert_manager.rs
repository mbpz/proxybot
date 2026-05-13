//! Certificate management for MITM proxy — no Tauri/GUI dependencies.
//!
//! Handles:
//! - Root CA generation and persistence (rcgen)
//! - Per-connection leaf certificate generation
//! - CA metadata (fingerprint, serial, expiry)
//! - PEM export
//!
//! # Integration
//!
//! This module is pure logic — it takes paths as parameters.
//! The Tauri layer (`src-tauri/src/cert.rs`) wraps it with
//! `AppConfig` path resolution and `#[tauri::command]` annotations.

use crate::config::{ca_cert_path, ca_dir};
use crate::types::CaMetadata;
use rcgen::{
    BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose, SanType,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Certificate manager handling CA and per-host leaf certificates.
pub struct CertManager {
    /// Serialized PEM of the CA certificate (for export/download)
    ca_cert_pem: Mutex<String>,
    /// Serialized PEM of the CA private key (for signing leaf certs)
    ca_key_pem: Mutex<String>,
    /// Cached leaf certificates: host → (cert_pem, key_pem)
    host_certs: Mutex<HashMap<String, (String, String)>>,
    /// Directory where CA files are stored
    ca_dir: PathBuf,
}

impl CertManager {
    /// Create a new CertManager, loading or generating the root CA.
    ///
    /// CA files are stored in `ca_dir`. If an existing CA is found,
    /// it is loaded; otherwise a new one is generated.
    pub fn new(ca_dir: Option<PathBuf>) -> Result<Self, String> {
        let dir = ca_dir.unwrap_or_else(ca_dir);
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create ca dir: {}", e))?;

        let (cert_pem, key_pem) = Self::load_or_generate_ca(&dir)?;

        Ok(Self {
            ca_cert_pem: Mutex::new(cert_pem),
            ca_key_pem: Mutex::new(key_pem),
            host_certs: Mutex::new(HashMap::new()),
            ca_dir: dir,
        })
    }

    /// Load existing CA or generate a new one.
    fn load_or_generate_ca(ca_dir: &Path) -> Result<(String, String), String> {
        let ca_pem_path = ca_dir.join("ca.pem");
        let meta_path = ca_dir.join("ca.meta.json");

        if ca_pem_path.exists() && meta_path.exists() {
            let cert_pem = fs::read_to_string(&ca_pem_path)
                .map_err(|e| format!("Failed to read CA PEM: {}", e))?;
            let key_path = ca_dir.join("ca.key");
            let key_pem = fs::read_to_string(&key_path)
                .map_err(|e| format!("Failed to read CA key: {}", e))?;

            log::info!("Loaded existing CA certificate from {:?}", ca_pem_path);
            return Ok((cert_pem, key_pem));
        }

        Self::generate_and_save_ca(ca_dir)
    }

    /// Generate a new root CA and persist to disk.
    fn generate_and_save_ca(ca_dir: &Path) -> Result<(String, String), String> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(DnType::CommonName, "ProxyBot MITM CA");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "ProxyBot");
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];

        let not_after = UNIX_EPOCH
            .checked_add(Duration::from_secs(365 * 24 * 60 * 60 * 10))
            .expect("date arithmetic overflow");
        params.not_after = not_after.into();

        let key_pair =
            KeyPair::generate().map_err(|e| format!("Failed to generate key: {}", e))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| format!("Failed to sign CA: {}", e))?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let ca_pem_path = ca_dir.join("ca.pem");
        let key_path = ca_dir.join("ca.key");

        fs::write(&ca_pem_path, &cert_pem)
            .map_err(|e| format!("Failed to write CA PEM: {}", e))?;
        fs::write(&key_path, &key_pem)
            .map_err(|e| format!("Failed to write CA key: {}", e))?;

        // Write metadata
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let serial = format!("{:016x}", now);
        let meta = CaMetadata {
            created_at: now,
            serial,
        };
        let meta_path = ca_dir.join("ca.meta.json");
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialize CA metadata: {}", e))?;
        fs::write(&meta_path, meta_json)
            .map_err(|e| format!("Failed to write CA metadata: {}", e))?;

        log::info!("Generated new CA certificate at {:?}", ca_pem_path);
        Ok((cert_pem, key_pem))
    }

    /// Get the CA certificate as PEM string.
    pub fn get_ca_cert_pem(&self) -> String {
        self.ca_cert_pem.lock().unwrap().clone()
    }

    /// Get the CA private key as PEM string (for signing leaf certs).
    pub fn get_ca_key_pem(&self) -> String {
        self.ca_key_pem.lock().unwrap().clone()
    }

    /// Get CA metadata (created_at, serial).
    pub fn get_ca_metadata(&self) -> Option<CaMetadata> {
        let meta_path = self.ca_dir.join("ca.meta.json");
        let json = fs::read_to_string(&meta_path).ok()?;
        serde_json::from_str(&json).ok()
    }

    /// Get SHA1 fingerprint of the CA certificate (hex string with colons).
    pub fn get_ca_fingerprint(&self) -> String {
        let cert_pem = self.ca_cert_pem.lock().unwrap();
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(cert_pem.as_bytes());
        let result = hasher.finalize();
        result
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":")
    }

    /// Get CA expiry info: (date_string, days_until_expiry).
    pub fn get_ca_expiry(&self) -> (String, i64) {
        if let Some(meta) = self.get_ca_metadata() {
            let created_at_secs = meta.created_at as i64;
            let expiry_secs = created_at_secs + (365 * 10 * 24 * 60 * 60) as i64;
            let expiry_date = format_expiry_date(expiry_secs as u64);

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let days = (expiry_secs - now) / 86400;
            return (expiry_date, days);
        }
        ("Unknown".to_string(), -1)
    }

    /// Export CA PEM to a file and return its path.
    pub fn export_ca_pem(&self, dest: Option<PathBuf>) -> Result<String, String> {
        let cert_pem = self.ca_cert_pem.lock().unwrap();
        let dest = dest.unwrap_or_else(ca_cert_path);
        fs::write(&dest, cert_pem.as_bytes())
            .map_err(|e| format!("Failed to write CA: {}", e))?;
        log::info!("Exported CA certificate to {:?}", dest);
        dest.to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Invalid path".to_string())
    }

    /// Regenerate CA certificate. Clears cached host certs.
    pub fn regenerate_ca(&self) -> Result<(), String> {
        let (cert_pem, key_pem) = Self::generate_and_save_ca(&self.ca_dir)?;
        *self.ca_cert_pem.lock().map_err(|e| e.to_string())? = cert_pem;
        *self.ca_key_pem.lock().map_err(|e| e.to_string())? = key_pem;
        *self.host_certs.lock().map_err(|e| e.to_string())? = HashMap::new();
        Ok(())
    }

    /// Generate a per-host leaf certificate signed by the root CA.
    ///
    /// Returns (cert_pem, key_pem). Results are cached by hostname.
    pub fn generate_host_cert(&self, host: &str) -> Result<(String, String), String> {
        let mut host_certs = self.host_certs.lock().map_err(|e| e.to_string())?;

        if let Some(cert) = host_certs.get(host) {
            return Ok(cert.clone());
        }

        let key_pair =
            KeyPair::generate().map_err(|e| format!("Failed to generate host key: {}", e))?;

        let mut params = CertificateParams::default();
        params.is_ca = IsCa::NoCa;
        params.distinguished_name.push(DnType::CommonName, host);
        params.subject_alt_names = vec![SanType::DnsName(
            host.try_into()
                .map_err(|e: rcgen::Error| format!("Invalid hostname: {}", e))?,
        )];

        let not_after = UNIX_EPOCH
            .checked_add(Duration::from_secs(86400))
            .expect("date arithmetic overflow");
        params.not_after = not_after.into();

        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];

        let ca_key_pem = self.ca_key_pem.lock().map_err(|e| e.to_string())?;
        let ca_key_pair = KeyPair::from_pem(&ca_key_pem)
            .map_err(|e| format!("Failed to parse CA key: {}", e))?;

        let issuer = Issuer::new(params.clone(), ca_key_pair);
        let cert = params
            .signed_by(&key_pair, &issuer)
            .map_err(|e| format!("Failed to sign host cert: {}", e))?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let result = (cert_pem.clone(), key_pem.clone());
        host_certs.insert(host.to_string(), (cert_pem, key_pem));

        Ok(result)
    }
}

impl Default for CertManager {
    fn default() -> Self {
        Self::new(None).expect("Failed to initialize CertManager")
    }
}

/// Format expiry timestamp as a human-readable date string.
fn format_expiry_date(secs: u64) -> String {
    let total_days = secs / 86400;
    let year = 1970 + (total_days / 365) as i64;
    let remaining_days = (total_days % 365) as i64;

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_months = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut remaining = remaining_days;
    let mut month = 1;
    for days in days_in_months.iter() {
        if remaining < *days as i64 {
            break;
        }
        remaining -= *days as i64;
        month += 1;
    }
    let day = remaining + 1;

    let secs_in_day = secs % 86400;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:00 UTC",
        year, month, day, hours, minutes
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_cert_manager() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(Some(dir.path().to_path_buf())).unwrap();
        let pem = mgr.get_ca_cert_pem();
        assert!(pem.contains("BEGIN CERTIFICATE"));
        assert!(pem.contains("END CERTIFICATE"));
    }

    #[test]
    fn test_ca_metadata() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(Some(dir.path().to_path_buf())).unwrap();
        let meta = mgr.get_ca_metadata().unwrap();
        assert!(meta.created_at > 0);
        assert!(!meta.serial.is_empty());
    }

    #[test]
    fn test_ca_fingerprint() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(Some(dir.path().to_path_buf())).unwrap();
        let fp = mgr.get_ca_fingerprint();
        // SHA1 fingerprint is 20 bytes = 59 chars with colons
        assert!(fp.contains(':'));
    }

    #[test]
    fn test_ca_expiry() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(Some(dir.path().to_path_buf())).unwrap();
        let (date, days) = mgr.get_ca_expiry();
        assert!(!date.is_empty());
        assert!(days > 0); // Should be close to 3650 (10 years)
    }

    #[test]
    fn test_export_ca_pem() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(Some(dir.path().to_path_buf())).unwrap();
        let export_path = dir.path().join("exported_ca.crt");
        let result = mgr.export_ca_pem(Some(export_path.clone())).unwrap();
        assert!(export_path.exists());
        let content = fs::read_to_string(&export_path).unwrap();
        assert!(content.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn test_generate_host_cert() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(Some(dir.path().to_path_buf())).unwrap();
        let (cert_pem, key_pem) = mgr.generate_host_cert("example.com").unwrap();
        assert!(cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_host_cert_caching() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(Some(dir.path().to_path_buf())).unwrap();
        let (cert1, _) = mgr.generate_host_cert("example.com").unwrap();
        let (cert2, _) = mgr.generate_host_cert("example.com").unwrap();
        // Same host should return cached cert
        assert_eq!(cert1, cert2);
    }

    #[test]
    fn test_regenerate_ca() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(Some(dir.path().to_path_buf())).unwrap();
        let old_pem = mgr.get_ca_cert_pem();
        mgr.regenerate_ca().unwrap();
        let new_pem = mgr.get_ca_cert_pem();
        assert_ne!(old_pem, new_pem);
    }

    #[test]
    fn test_default_constructor() {
        // Default uses ca_dir() which may fail in CI without HOME.
        // Skip if HOME is not set.
        if std::env::var("HOME").is_err() {
            return;
        }
        let mgr = CertManager::default();
        let pem = mgr.get_ca_cert_pem();
        assert!(pem.contains("BEGIN CERTIFICATE"));
    }
}
