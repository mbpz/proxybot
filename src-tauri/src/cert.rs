use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose, SanType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
pub struct CaMetadata {
    pub created_at: u64,
    pub serial: String,
}

pub struct CertManager {
    /// Serialized PEM of the CA certificate (for export/download)
    ca_cert_pem: Mutex<String>,
    /// Serialized PEM of the CA private key (for signing leaf certs)
    ca_key_pem: Mutex<String>,
    /// Cached leaf certificates: host -> (cert_pem, key_pem)
    #[allow(dead_code)]
    host_certs: Mutex<HashMap<String, (String, String)>>,
}

impl CertManager {
    pub fn new() -> Result<Self, String> {
        let ca_dir = Self::get_ca_dir()?;
        fs::create_dir_all(&ca_dir).map_err(|e| format!("Failed to create ca dir: {}", e))?;

        let (cert_pem, key_pem) = Self::load_or_generate_ca(&ca_dir)?;

        Ok(Self {
            ca_cert_pem: Mutex::new(cert_pem),
            ca_key_pem: Mutex::new(key_pem),
            host_certs: Mutex::new(HashMap::new()),
        })
    }

    fn get_ca_dir() -> Result<PathBuf, String> {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        Ok(PathBuf::from(home).join(".proxybot"))
    }

    fn load_or_generate_ca(ca_dir: &PathBuf) -> Result<(String, String), String> {
        let ca_pem_path = ca_dir.join("ca.pem");
        let meta_path = ca_dir.join("ca.meta.json");

        // Load existing CA if present
        if ca_pem_path.exists() && meta_path.exists() {
            let cert_pem = fs::read_to_string(&ca_pem_path)
                .map_err(|e| format!("Failed to read CA PEM: {}", e))?;
            let key_path = ca_dir.join("ca.key");
            let key_pem = fs::read_to_string(&key_path)
                .map_err(|e| format!("Failed to read CA key: {}", e))?;

            log::info!("Loaded existing CA certificate from {:?}", ca_pem_path);
            return Ok((cert_pem, key_pem));
        }

        // Generate new CA
        Self::generate_and_save_ca(ca_dir)
    }

    fn generate_and_save_ca(ca_dir: &PathBuf) -> Result<(String, String), String> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.distinguished_name.push(DnType::CommonName, "ProxyBot MITM CA");
        params.distinguished_name.push(DnType::OrganizationName, "ProxyBot");
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];

        let not_after = UNIX_EPOCH
            .checked_add(Duration::from_secs(365 * 24 * 60 * 60 * 10))
            .expect("date arithmetic overflow");
        params.not_after = not_after.into();

        let key_pair = KeyPair::generate().map_err(|e| format!("Failed to generate key: {}", e))?;
        let cert = params.self_signed(&key_pair)
            .map_err(|e| format!("Failed to sign CA: {}", e))?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let ca_pem_path = ca_dir.join("ca.pem");
        let key_path = ca_dir.join("ca.key");

        fs::write(&ca_pem_path, &cert_pem).map_err(|e| format!("Failed to write CA PEM: {}", e))?;
        fs::write(&key_path, &key_pem).map_err(|e| format!("Failed to write CA key: {}", e))?;

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

    pub fn get_ca_cert_pem(&self) -> String {
        self.ca_cert_pem.lock().unwrap().clone()
    }

    /// Get CA metadata for display.
    pub fn get_ca_metadata(&self) -> Option<CaMetadata> {
        let ca_dir = Self::get_ca_dir().ok()?;
        let meta_path = ca_dir.join("ca.meta.json");
        let json = fs::read_to_string(&meta_path).ok()?;
        serde_json::from_str(&json).ok()
    }

    /// Regenerate CA certificate. Existing in-memory host certs remain valid.
    pub fn regenerate_ca(&self) -> Result<(), String> {
        let ca_dir = Self::get_ca_dir()?;
        let (cert_pem, key_pem) = Self::generate_and_save_ca(&ca_dir)?;
        *self.ca_cert_pem.lock().map_err(|e| e.to_string())? = cert_pem;
        *self.ca_key_pem.lock().map_err(|e| e.to_string())? = key_pem;
        // Clear host cert cache so new leaf certs use new CA
        *self.host_certs.lock().map_err(|e| e.to_string())? = HashMap::new();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_ca_key_pem(&self) -> String {
        self.ca_key_pem.lock().unwrap().clone()
    }

    #[allow(dead_code)]
    pub fn generate_host_cert(&self, host: &str) -> Result<(String, String), String> {
        let mut host_certs = self.host_certs.lock().map_err(|e| e.to_string())?;

        if let Some(cert) = host_certs.get(host) {
            return Ok(cert.clone());
        }

        let key_pair = KeyPair::generate().map_err(|e| format!("Failed to generate host key: {}", e))?;

        let mut params = CertificateParams::default();
        params.is_ca = IsCa::NoCa;
        params.distinguished_name.push(DnType::CommonName, host);
        params.subject_alt_names = vec![SanType::DnsName(host.try_into().map_err(|e: rcgen::Error| format!("Invalid hostname: {}", e))?)];

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
