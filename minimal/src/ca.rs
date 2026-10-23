//! Root CA generation and persistence for TLS interception.

use anyhow::{Context, Result};
use hudsucker::rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose};
use std::fs;
use std::path::Path;

pub struct CaMaterial {
    pub cert_pem: String,
    pub key_pem: String,
}

/// Load the local CA from `dir`, generating and persisting it on first run.
pub fn load_or_create(dir: &Path) -> Result<CaMaterial> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let cert_path = dir.join("ca.pem");
    let key_path = dir.join("ca.key");

    if cert_path.exists() && key_path.exists() {
        let cert_pem = fs::read_to_string(&cert_path)?;
        let key_pem = fs::read_to_string(&key_path)?;
        // Validate the pair early so a corrupted CA fails here, not in a handshake.
        let key = KeyPair::from_pem(&key_pem).context("parse stored CA private key")?;
        Issuer::from_ca_cert_pem(&cert_pem, key).context("parse stored CA certificate")?;
        return Ok(CaMaterial { cert_pem, key_pem });
    }

    let key_pair = KeyPair::generate().context("generate CA key pair")?;
    // A CA must not carry a DNS SAN (the Common Name is enough); an invalid SAN
    // such as "ProxyBot Local CA" makes OpenSSL reject the chain with
    // "unsupported or invalid name syntax".
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    params.distinguished_name.push(DnType::CommonName, "ProxyBot Local CA");
    let cert = params.self_signed(&key_pair).context("sign CA certificate")?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();
    fs::write(&cert_path, &cert_pem)?;
    fs::write(&key_path, &key_pem)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
    }

    Ok(CaMaterial { cert_pem, key_pem })
}

pub fn issuer(material: &CaMaterial) -> Result<Issuer<'static, KeyPair>> {
    let key = KeyPair::from_pem(&material.key_pem).context("parse CA private key")?;
    Issuer::from_ca_cert_pem(&material.cert_pem, key).context("parse CA certificate")
}
