//! Integration tests for AppError types.

use proxybot_lib::error::{AppError, DbError, CertError, ProxyError, DnsError, RulesError};

#[test]
fn test_app_error_db_variant() {
    let err = AppError::Db(DbError::Connection("timeout".into()));
    assert!(err.to_string().contains("database error"));
    assert!(err.to_string().contains("timeout"));
}

#[test]
fn test_app_error_cert_variant() {
    let err = AppError::Cert(CertError::NotFound("ca.pem missing".into()));
    assert!(err.to_string().contains("certificate error"));
    assert!(err.to_string().contains("ca.pem missing"));
}

#[test]
fn test_app_error_proxy_variant() {
    let err = AppError::Proxy(ProxyError::Bind("port in use".into()));
    assert!(err.to_string().contains("proxy error"));
    assert!(err.to_string().contains("port in use"));
}

#[test]
fn test_app_error_dns_variant() {
    let err = AppError::Dns(DnsError::Upstream(" unreachable".into()));
    assert!(err.to_string().contains("DNS error"));
}

#[test]
fn test_app_error_rules_variant() {
    let err = AppError::Rules(RulesError::Load("parse error".into()));
    assert!(err.to_string().contains("rules error"));
}

#[test]
fn test_app_error_io_variant() {
    let err = AppError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
    assert!(err.to_string().contains("I/O error"));
}

#[test]
fn test_app_error_config_variant() {
    let err = AppError::Config("HOME not set".into());
    assert!(err.to_string().contains("configuration error"));
    assert!(err.to_string().contains("HOME not set"));
}

#[test]
fn test_db_error_display() {
    let err = DbError::Query("SELECT failed".into());
    assert_eq!(err.to_string(), "query failed: SELECT failed");
}

#[test]
fn test_proxy_error_display() {
    let err = ProxyError::Tls("handshake failed".into());
    assert_eq!(err.to_string(), "TLS error: handshake failed");
}

#[test]
fn test_dns_error_display() {
    let err = DnsError::Parse("invalid packet".into());
    assert_eq!(err.to_string(), "parse error: invalid packet");
}
