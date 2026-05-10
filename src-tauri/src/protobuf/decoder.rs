// src-tauri/src/protobuf/decoder.rs

/// Standard gRPC frame (non-Web).
/// gRPC uses a 5-byte header: [compressed_flag: u8, message_length: u32 BE].
#[derive(Debug, Clone, PartialEq)]
pub struct GrpcFrame {
    pub compressed: bool,
    pub data: Vec<u8>,
}

/// Decode all standard gRPC frames from raw body bytes.
/// Parses headers of the form [compressed(1), length(4 BE)], then data.
/// Handles partial/invalid frames gracefully.
pub fn decode_grpc_frames(body: &[u8]) -> Vec<GrpcFrame> {
    let mut frames = Vec::new();
    let mut offset = 0;

    while offset + 5 <= body.len() {
        let compressed = body[offset] == 1;
        let msg_len = u32::from_be_bytes([
            body[offset + 1],
            body[offset + 2],
            body[offset + 3],
            body[offset + 4],
        ]) as usize;
        offset += 5;

        if offset + msg_len > body.len() {
            // Partial frame at end — include what we have
            let available = body.len() - offset;
            if available > 0 {
                frames.push(GrpcFrame {
                    compressed,
                    data: body[offset..].to_vec(),
                });
            }
            break;
        }

        frames.push(GrpcFrame {
            compressed,
            data: body[offset..offset + msg_len].to_vec(),
        });
        offset += msg_len;
    }

    frames
}

/// gRPC-Web frame (from gRPC-Web wire format).
#[derive(Debug)]
pub struct GrpcWebFrame {
    pub frame_type: u8,
    pub data: Vec<u8>,
}

/// Decode a gRPC-Web frame (base64 encoded).
/// gRPC-Web uses 5-byte header: [version (0), type (1), flags (2-4), stream_id (5-8)].
/// Note: Frame type byte has format: bit 7 = trailer indicator, bits 6-0 = type.
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

/// Extract gRPC-Web trailers from a list of decoded gRPC-Web frames.
/// In gRPC-Web, trailers are sent in a separate frame with the 0x80 (bit 7) flag set
/// on the frame type byte. Trailer data is encoded as HTTP/1.1-style header lines.
pub fn extract_grpc_web_trailers(frames: &[GrpcWebFrame]) -> Vec<(String, String)> {
    let mut trailers = Vec::new();

    for frame in frames {
        // gRPC-Web trailer indicator: bit 7 (0x80) set on frame type
        if frame.frame_type & 0x80 != 0 {
            if let Ok(text) = String::from_utf8(frame.data.clone()) {
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(colon_idx) = line.find(':') {
                        let name = line[..colon_idx].trim().to_string();
                        let value = line[colon_idx + 1..].trim().to_string();
                        trailers.push((name, value));
                    }
                }
            }
        }
    }

    trailers
}

/// A single protobuf field parsed from the wire format.
#[derive(Debug, Clone, PartialEq)]
pub struct ProtobufField {
    pub field_number: u32,
    pub wire_type: u8,
    pub value: ProtobufValue,
}

/// Protobuf wire format value types.
#[derive(Debug, Clone, PartialEq)]
pub enum ProtobufValue {
    Varint(u64),
    Fixed64(u64),
    LengthDelimited(Vec<u8>),
    Fixed32(u32),
}

/// Read a varint from bytes starting at `offset`, returning the value and new offset.
fn read_varint(data: &[u8], offset: usize) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift = 0;
    let mut pos = offset;

    while pos < data.len() {
        let byte = data[pos];
        pos += 1;
        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((value, pos));
        }
        shift += 7;
        if shift >= 64 {
            // Malformed: varint too long
            return None;
        }
    }

    None // Unexpected EOF
}

/// Parse raw protobuf bytes into fields (best-effort, no schema required).
///
/// The protobuf wire format uses a tag (varint-encoded field_number << 3 | wire_type)
/// followed by the value, whose format depends on the wire type:
///
/// - Wire type 0 (Varint): varint-encoded integer
/// - Wire type 1 (64-bit): 8 bytes, little-endian
/// - Wire type 2 (Length-delimited): varint length prefix, then that many bytes
/// - Wire type 5 (32-bit): 4 bytes, little-endian
///
/// Returns all successfully parsed fields. Invalid/malformed data is skipped.
pub fn parse_protobuf_fields(data: &[u8]) -> Vec<ProtobufField> {
    let mut fields = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        // Read tag (varint: field_number << 3 | wire_type)
        let (tag, new_offset) = match read_varint(data, offset) {
            Some(t) => t,
            None => break, // Can't read tag, stop
        };
        offset = new_offset;

        let wire_type = (tag & 0x07) as u8;
        let field_number = (tag >> 3) as u32;

        match wire_type {
            0 => {
                // Varint
                match read_varint(data, offset) {
                    Some((val, new_off)) => {
                        fields.push(ProtobufField {
                            field_number,
                            wire_type,
                            value: ProtobufValue::Varint(val),
                        });
                        offset = new_off;
                    }
                    None => break,
                }
            }
            1 => {
                // 64-bit (fixed64, double)
                if offset + 8 <= data.len() {
                    let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
                    let val = u64::from_le_bytes(bytes);
                    fields.push(ProtobufField {
                        field_number,
                        wire_type,
                        value: ProtobufValue::Fixed64(val),
                    });
                    offset += 8;
                } else {
                    break;
                }
            }
            2 => {
                // Length-delimited (string, bytes, embedded message, packed repeated)
                match read_varint(data, offset) {
                    Some((len, new_off)) => {
                        offset = new_off;
                        let len = len as usize;
                        if offset + len <= data.len() {
                            fields.push(ProtobufField {
                                field_number,
                                wire_type,
                                value: ProtobufValue::LengthDelimited(data[offset..offset + len].to_vec()),
                            });
                            offset += len;
                        } else {
                            // Partial data at end
                            if offset < data.len() {
                                fields.push(ProtobufField {
                                    field_number,
                                    wire_type,
                                    value: ProtobufValue::LengthDelimited(data[offset..].to_vec()),
                                });
                            }
                            offset = data.len();
                        }
                    }
                    None => break,
                }
            }
            5 => {
                // 32-bit (fixed32, float)
                if offset + 4 <= data.len() {
                    let bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
                    let val = u32::from_le_bytes(bytes);
                    fields.push(ProtobufField {
                        field_number,
                        wire_type,
                        value: ProtobufValue::Fixed32(val),
                    });
                    offset += 4;
                } else {
                    break;
                }
            }
            _ => {
                // Unknown wire type (3, 4, 6, 7) — can't determine length, bail out
                break;
            }
        }
    }

    fields
}

/// Decode a protobuf message from raw bytes and return a human-readable JSON representation.
///
/// Returns a JSON array of field objects. Each object has:
/// - `field`: the field number
/// - `type`: the wire type name ("varint", "fixed64", "length_delimited", "fixed32")
/// - `value`: the value representation (decimal for varints, hex for length-delimited, etc.)
/// - `utf8`: for length-delimited fields that are valid UTF-8, the decoded string
pub fn decode_protobuf(body: &[u8]) -> Result<String, String> {
    let fields = parse_protobuf_fields(body);

    let mut json_parts: Vec<String> = Vec::new();

    for field in &fields {
        let type_name = match field.wire_type {
            0 => "varint",
            1 => "fixed64",
            2 => "length_delimited",
            5 => "fixed32",
            _ => "unknown",
        };

        let value_repr = match &field.value {
            ProtobufValue::Varint(v) => v.to_string(),
            ProtobufValue::Fixed64(v) => format!("0x{:016x}", v),
            ProtobufValue::Fixed32(v) => format!("0x{:08x}", v),
            ProtobufValue::LengthDelimited(data) => {
                format!("0x{}", hex_encode(data))
            }
        };

        let mut obj = format!(
            r#"{{"field":{},"type":"{}","value":"{}""#,
            field.field_number, type_name, value_repr
        );

        // For length-delimited fields that are valid UTF-8, include the decoded string
        if let ProtobufValue::LengthDelimited(data) = &field.value {
            if let Ok(s) = String::from_utf8(data.clone()) {
                if s.chars().all(|c| !c.is_control() || c.is_whitespace()) && !s.is_empty() {
                    // Escape the string for JSON
                    let escaped = json_escape(&s);
                    obj.push_str(&format!(r#","utf8":"{}""#, escaped));
                }
            }
        }

        obj.push('}');
        json_parts.push(obj);
    }

    Ok(format!("[{}]", json_parts.join(",")))
}

/// Simple hex encoding for bytes.
fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join("")
}

/// Minimal JSON string escaping.
fn json_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0C}' => escaped.push_str("\\f"),
            c if c.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Standard gRPC frame tests ──

    #[test]
    fn test_decode_standard_grpc_frame_single() {
        // gRPC frame: [0, 0, 0, 0, 5, h, e, l, l, o]
        // compressed=0, length=5, data="hello"
        let frames = decode_grpc_frames(&[0, 0, 0, 0, 5, b'h', b'e', b'l', b'l', b'o']);
        assert_eq!(frames.len(), 1);
        assert!(!frames[0].compressed);
        assert_eq!(frames[0].data, b"hello");
    }

    #[test]
    fn test_decode_standard_grpc_frame_compressed() {
        // compressed=1, length=3, data=[1, 2, 3]
        let frames = decode_grpc_frames(&[1, 0, 0, 0, 3, 1, 2, 3]);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].compressed);
        assert_eq!(frames[0].data, vec![1, 2, 3]);
    }

    #[test]
    fn test_decode_standard_grpc_frame_multiple() {
        // Frame 1: length=3, data="abc"
        // Frame 2: length=3, data="def"
        let body = [
            0, 0, 0, 0, 3, b'a', b'b', b'c',
            0, 0, 0, 0, 3, b'd', b'e', b'f',
        ];
        let frames = decode_grpc_frames(&body);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].data, b"abc");
        assert_eq!(frames[1].data, b"def");
    }

    #[test]
    fn test_decode_standard_grpc_frame_empty() {
        let frames = decode_grpc_frames(&[]);
        assert_eq!(frames.len(), 0);
    }

    #[test]
    fn test_decode_standard_grpc_frame_too_short() {
        let frames = decode_grpc_frames(&[0, 0, 0]); // Less than 5 bytes header
        assert_eq!(frames.len(), 0);
    }

    // ── Protobuf wire format tests ──

    #[test]
    fn test_parse_protobuf_varint() {
        // Wire type 0, field 1, value 150
        // tag = (1 << 3) | 0 = 0x08, value 150 = varint 0x96 0x01
        let fields = parse_protobuf_fields(&[0x08, 0x96, 0x01]);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_number, 1);
        assert_eq!(fields[0].wire_type, 0);
        assert_eq!(fields[0].value, ProtobufValue::Varint(150));
    }

    #[test]
    fn test_parse_protobuf_string_field() {
        // Field 2, wire type 2 (length-delimited), length 5, "hello"
        // tag = (2 << 3) | 2 = 0x12
        let data = [0x12, 0x05, b'h', b'e', b'l', b'l', b'o'];
        let fields = parse_protobuf_fields(&data);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_number, 2);
        assert_eq!(fields[0].wire_type, 2);
        assert_eq!(
            fields[0].value,
            ProtobufValue::LengthDelimited(b"hello".to_vec())
        );
    }

    #[test]
    fn test_parse_protobuf_fixed64() {
        // Field 3, wire type 1 (fixed64), value = 0xDEADBEEFCAFEBABE (little-endian)
        // tag = (3 << 3) | 1 = 0x19
        let mut data = vec![0x19];
        data.extend_from_slice(&0xDEADBEEFCAFEBABEu64.to_le_bytes());
        let fields = parse_protobuf_fields(&data);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_number, 3);
        assert_eq!(fields[0].wire_type, 1);
        assert_eq!(fields[0].value, ProtobufValue::Fixed64(0xDEADBEEFCAFEBABE));
    }

    #[test]
    fn test_parse_protobuf_fixed32() {
        // Field 4, wire type 5 (fixed32), value = 0x12345678 (little-endian)
        // tag = (4 << 3) | 5 = 0x25
        let mut data = vec![0x25];
        data.extend_from_slice(&0x12345678u32.to_le_bytes());
        let fields = parse_protobuf_fields(&data);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_number, 4);
        assert_eq!(fields[0].wire_type, 5);
        assert_eq!(fields[0].value, ProtobufValue::Fixed32(0x12345678));
    }

    #[test]
    fn test_parse_protobuf_multiple_fields() {
        // Field 1: varint 150
        // Field 2: length-delimited "test"
        let data = [
            0x08, 0x96, 0x01,                   // field 1, varint 150
            0x12, 0x04, b't', b'e', b's', b't', // field 2, string "test"
        ];
        let fields = parse_protobuf_fields(&data);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].field_number, 1);
        assert_eq!(fields[0].value, ProtobufValue::Varint(150));
        assert_eq!(fields[1].field_number, 2);
        assert_eq!(fields[1].value, ProtobufValue::LengthDelimited(b"test".to_vec()));
    }

    #[test]
    fn test_parse_protobuf_empty() {
        let fields = parse_protobuf_fields(&[]);
        assert_eq!(fields.len(), 0);
    }

    // ── decode_protobuf tests ──

    #[test]
    fn test_decode_protobuf_string_field() {
        // Field 2, wire type 2 (length-delimited), length 5, "hello"
        let data = [0x12, 0x05, b'h', b'e', b'l', b'l', b'o'];
        let result = decode_protobuf(&data).unwrap();
        assert!(result.contains("hello"));
        assert!(result.contains("\"field\":2"));
    }

    #[test]
    fn test_decode_protobuf_varint() {
        let data = [0x08, 0x96, 0x01];
        let result = decode_protobuf(&data).unwrap();
        assert!(result.contains("\"field\":1"));
        assert!(result.contains("\"value\":\"150\""));
        assert!(result.contains("\"type\":\"varint\""));
    }

    #[test]
    fn test_decode_protobuf_empty() {
        let result = decode_protobuf(&[]).unwrap();
        assert_eq!(result, "[]");
    }

    // ── gRPC-Web trailer tests ──

    #[test]
    fn test_extract_grpc_web_trailers() {
        let frames = vec![
            GrpcWebFrame {
                frame_type: 0x00, // regular data frame
                data: b"hello".to_vec(),
            },
            GrpcWebFrame {
                frame_type: 0x80, // trailer frame (bit 7 set)
                data: b"grpc-status: 0\r\ngrpc-message: OK\r\n".to_vec(),
            },
        ];
        let trailers = extract_grpc_web_trailers(&frames);
        assert_eq!(trailers.len(), 2);
        assert_eq!(trailers[0], ("grpc-status".to_string(), "0".to_string()));
        assert_eq!(trailers[1], ("grpc-message".to_string(), "OK".to_string()));
    }

    #[test]
    fn test_extract_grpc_web_trailers_no_trailers() {
        let frames = vec![
            GrpcWebFrame {
                frame_type: 0x00,
                data: b"data".to_vec(),
            },
        ];
        let trailers = extract_grpc_web_trailers(&frames);
        assert_eq!(trailers.len(), 0);
    }

    #[test]
    fn test_extract_grpc_web_trailers_empty() {
        let trailers = extract_grpc_web_trailers(&[]);
        assert_eq!(trailers.len(), 0);
    }
}
