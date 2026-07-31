//! WebSocket frame payload truncation.

/// Payloads above this size are truncated to PREVIEW_SIZE bytes.
pub const MAX_PAYLOAD_SIZE: usize = 64 * 1024;

/// When truncating, keep this many bytes of preview.
pub const PREVIEW_SIZE: usize = 1024;

/// Truncate a payload to fit within MAX_PAYLOAD_SIZE. Returns
/// (preview_string, was_truncated). Binary payloads are passed
/// through String::from_utf8_lossy; the hex view in the frontend
/// can use base64 if lossless rendering is needed.
pub fn truncate_payload(payload: &[u8]) -> (String, bool) {
    if payload.len() <= MAX_PAYLOAD_SIZE {
        (String::from_utf8_lossy(payload).to_string(), false)
    } else {
        (
            String::from_utf8_lossy(&payload[..PREVIEW_SIZE]).to_string(),
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_small_payload_not_truncated() {
        let payload = b"hello world";
        let (s, truncated) = truncate_payload(payload);
        assert_eq!(s, "hello world");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_exact_limit_not_truncated() {
        let payload = vec![b'x'; MAX_PAYLOAD_SIZE];
        let (s, truncated) = truncate_payload(&payload);
        assert_eq!(s.len(), MAX_PAYLOAD_SIZE);
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_oversize_truncated() {
        let payload = vec![b'y'; MAX_PAYLOAD_SIZE + 1];
        let (s, truncated) = truncate_payload(&payload);
        assert_eq!(s.len(), PREVIEW_SIZE);
        assert!(truncated);
    }

    #[test]
    fn test_truncate_way_oversize() {
        let payload = vec![b'z'; 1024 * 1024]; // 1MB
        let (_, truncated) = truncate_payload(&payload);
        assert!(truncated);
    }
}
