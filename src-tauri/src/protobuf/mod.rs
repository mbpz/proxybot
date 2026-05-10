// src-tauri/src/protobuf/mod.rs

pub mod decoder;

#[cfg(test)]
mod tests;

pub use decoder::{
    decode_grpc_frames, decode_grpc_web_frame, decode_protobuf, extract_grpc_web_trailers,
    parse_protobuf_fields, GrpcFrame, GrpcWebFrame, ProtobufField, ProtobufValue,
};

/// Detect if request is gRPC (application/grpc, application/grpc+proto, etc.)
pub fn is_grpc_request(headers: &[(String, String)]) -> bool {
    headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "content-type")
        .map(|(_, v)| v.to_lowercase().starts_with("application/grpc"))
        .unwrap_or(false)
}

/// Detect if request is standard gRPC (application/grpc, application/grpc+proto)
/// but NOT gRPC-Web (application/grpc-web, application/grpc-web+proto)
pub fn is_standard_grpc(headers: &[(String, String)]) -> bool {
    headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "content-type")
        .map(|(_, v)| {
            let v = v.to_lowercase();
            v.starts_with("application/grpc") && !v.starts_with("application/grpc-web")
        })
        .unwrap_or(false)
}

/// Detect if request is gRPC-Web (application/grpc-web, application/grpc-web+proto, etc.)
pub fn is_grpc_web(headers: &[(String, String)]) -> bool {
    headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "content-type")
        .map(|(_, v)| v.to_lowercase().starts_with("application/grpc-web"))
        .unwrap_or(false)
}

/// Detect if request is protobuf (application/x-protobuf, etc.)
pub fn is_protobuf(headers: &[(String, String)]) -> bool {
    let ct = headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "content-type")
        .map(|(_, v)| v.to_lowercase());

    match ct {
        Some(s) => s.contains("x-protobuf") || s.contains("google.protobuf"),
        None => false,
    }
}

/// Convenience: check if any kind of gRPC content-type (standard or web).
pub fn is_any_grpc(headers: &[(String, String)]) -> bool {
    is_grpc_request(headers)
}
