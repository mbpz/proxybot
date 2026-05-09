// src-tauri/src/protobuf/decoder.rs

/// gRPC-Web frame
#[derive(Debug)]
pub struct GrpcWebFrame {
    pub frame_type: u8,
    pub data: Vec<u8>,
}

/// Decode a gRPC-Web frame (base64 encoded)
/// gRPC-Web uses 5-byte header: [version, type, flags, stream_id (4 bytes)]
pub fn decode_grpc_web_frame(encoded: &str) -> Result<GrpcWebFrame, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    let bytes = BASE64.decode(encoded).map_err(|e| e.to_string())?;
    if bytes.len() < 5 {
        return Err("Frame too short".into());
    }

    Ok(GrpcWebFrame {
        frame_type: bytes[0],
        data: bytes[5..].to_vec(),
    })
}

/// Decode a protobuf message using descriptor
/// Returns JSON representation
pub fn decode_protobuf(body: &[u8], _descriptor: &[u8]) -> Result<String, String> {
    // TODO: Use prost for decoding when descriptor is available
    Ok("{}".into())
}
