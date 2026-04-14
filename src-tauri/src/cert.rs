use rcgen::{BasicConstraints, Certificate, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose, SanType};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

pub struct CertManager {
    ca_cert: Certificate,
    #[allow(dead_code)]
    ca_key_pem: String,
    #[allow(dead_code)]
    host_certs: Mutex<HashMap<String, (String, String)>>,
}

impl CertManager {
    pub fn new() -> Result<Self, String> {
        let ca_dir = Self::get_ca_dir()?;
        fs::create_dir_all(&ca_dir).map_err(|e| format!("Failed to create ca dir: {}", e))?;

        let (ca_cert, ca_key_pem) = Self::generate_and_save_ca(&ca_dir)?;

        Ok(Self {
            ca_cert,
            ca_key_pem,
            host_certs: Mutex::new(HashMap::new()),
        })
    }

    fn get_ca_dir() -> Result<PathBuf, String> {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        Ok(PathBuf::from(home).join(".proxybot"))
    }

    fn generate_and_save_ca(ca_dir: &PathBuf) -> Result<(Certificate, String), String> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.distinguished_name.push(DnType::CommonName, "ProxyBot MITM CA");
        params.distinguished_name.push(DnType::OrganizationName, "ProxyBot");
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];

        let not_after = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(365 * 24 * 60 * 60 * 10))
            .expect("date arithmetic overflow");
        params.not_after = not_after.into();

        let key_pair = KeyPair::generate().map_err(|e| format!("Failed to generate key: {}", e))?;
        let cert = params.self_signed(&key_pair)
            .map_err(|e| format!("Failed to sign CA: {}", e))?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let cert_path = ca_dir.join("ca.crt");
        let key_path = ca_dir.join("ca.key");

        fs::write(&cert_path, &cert_pem).map_err(|e| format!("Failed to write cert: {}", e))?;
        fs::write(&key_path, &key_pem).map_err(|e| format!("Failed to write key: {}", e))?;

        log::info!("Generated new CA certificate at {:?}", cert_path);
        Ok((cert, key_pem))
    }

    pub fn get_ca_cert_pem(&self) -> String {
        self.ca_cert.pem()
    }

    #[allow(dead_code)]
    pub fn get_ca_key_pem(&self) -> String {
        self.ca_key_pem.clone()
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

        let not_after = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(86400))
            .expect("date arithmetic overflow");
        params.not_after = not_after.into();

        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];

        let ca_key_pair = KeyPair::from_pem(&self.ca_key_pem)
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
