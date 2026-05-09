// src-tauri/src/protobuf/mod.rs

pub mod decoder;

/// Detect if request is gRPC (application/grpc, application/grpc+proto, etc.)
pub fn is_grpc_request(headers: &[(String, String)]) -> bool {
    headers.iter()
        .find(|(k, _)| k.to_lowercase() == "content-type")
        .map(|(_, v)| v.to_lowercase().starts_with("application/grpc"))
        .unwrap_or(false)
}

/// Detect if request is protobuf (application/x-protobuf, etc.)
pub fn is_protobuf(headers: &[(String, String)]) -> bool {
    let ct = headers.iter()
        .find(|(k, _)| k.to_lowercase() == "content-type")
        .map(|(_, v)| v.to_lowercase());

    match ct {
        Some(s) => s.contains("x-protobuf") || s.contains("google.protobuf"),
        None => false,
    }
}
