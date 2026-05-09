// src-tauri/src/protobuf/decoder.rs

/// gRPC-Web frame
#[derive(Debug)]
pub struct GrpcWebFrame {
    pub frame_type: u8,
    pub data: Vec<u8>,
}

/// Decode a gRPC-Web frame (base64 encoded)
/// gRPC-Web uses 5-byte header: [version (0), type (1), flags (2-4), stream_id (5-8)]
pub fn decode_grpc_web_frame(encoded: &str) -> Result<GrpcWebFrame, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    let bytes = BASE64.decode(encoded).map_err(|e| e.to_string())?;
    if bytes.len() < 5 {
        return Err("Frame too short".into());
    }

    Ok(GrpcWebFrame {
        frame_type: bytes[1], // byte 1 = frame type
        data: bytes[5..].to_vec(),
    })
}

/// Decode a protobuf message using descriptor
/// Returns JSON representation
pub fn decode_protobuf(_body: &[u8], _descriptor: &[u8]) -> Result<String, String> {
    // TODO: Use prost for decoding when descriptor is available
    Ok("{}".into())
}
