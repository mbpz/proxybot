//! HTTP body decompression for captured responses.
//!
//! Captured response bodies arrive as raw wire bytes. Most JSON APIs
//! today respond `Content-Encoding: gzip` (or `br`/`deflate`), so the
//! raw bytes aren't UTF-8 — a naive `String::from_utf8` drops them and
//! the user sees nothing. [`decompress`] inflates the body so the real
//! payload reaches the capture, the spec generator, and HAR export.
//!
//! Unknown / absent encodings (or `identity`) pass through unchanged.
//! Decompression failure falls back to the original bytes rather than
//! erroring — a truncated stream is better shown as binary than lost.

use std::io::Read;

/// Decompress `body` according to the `content_encoding` token
/// (case-insensitive). Returns the inflated bytes, or the input
/// unchanged for `identity` / unknown / failed decompression.
///
/// `content_encoding` is the raw header value (e.g. `"gzip"`,
/// `"br"`); pass an empty string when the header is absent.
pub fn decompress(content_encoding: &str, body: &[u8]) -> Vec<u8> {
    if body.is_empty() {
        return body.to_vec();
    }
    match content_encoding.trim().to_ascii_lowercase().as_str() {
        "gzip" | "x-gzip" => decode_gzip(body).unwrap_or_else(|| body.to_vec()),
        "deflate" => decode_deflate(body).unwrap_or_else(|| body.to_vec()),
        "br" => decode_brotli(body).unwrap_or_else(|| body.to_vec()),
        _ => body.to_vec(),
    }
}

fn decode_gzip(body: &[u8]) -> Option<Vec<u8>> {
    use flate2::read::GzDecoder;
    let mut out = Vec::new();
    GzDecoder::new(body).read_to_end(&mut out).ok()?;
    Some(out)
}

fn decode_deflate(body: &[u8]) -> Option<Vec<u8>> {
    // Servers disagree: "deflate" is sometimes zlib-wrapped, sometimes
    // raw. Try zlib first (the common case), then raw deflate.
    use flate2::read::{DeflateDecoder, ZlibDecoder};
    let mut out = Vec::new();
    if ZlibDecoder::new(body).read_to_end(&mut out).is_ok() {
        return Some(out);
    }
    let mut raw = Vec::new();
    DeflateDecoder::new(body).read_to_end(&mut raw).ok()?;
    Some(raw)
}

fn decode_brotli(body: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    brotli::Decompressor::new(body, 4096)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn gzip(data: &[u8]) -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    fn zlib(data: &[u8]) -> Vec<u8> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    fn brotli_enc(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut w = brotli::CompressorWriter::new(&mut out, 4096, 5, 22);
        w.write_all(data).unwrap();
        drop(w);
        out
    }

    const PAYLOAD: &[u8] = br#"{"id":1,"name":"alice","items":[1,2,3]}"#;

    #[test]
    fn gzip_round_trips() {
        let compressed = gzip(PAYLOAD);
        assert_ne!(compressed, PAYLOAD, "should actually be compressed");
        assert_eq!(decompress("gzip", &compressed), PAYLOAD);
        // Case-insensitive + x-gzip alias.
        assert_eq!(decompress("GZIP", &compressed), PAYLOAD);
        assert_eq!(decompress("x-gzip", &compressed), PAYLOAD);
    }

    #[test]
    fn deflate_zlib_round_trips() {
        let compressed = zlib(PAYLOAD);
        assert_eq!(decompress("deflate", &compressed), PAYLOAD);
    }

    #[test]
    fn brotli_round_trips() {
        let compressed = brotli_enc(PAYLOAD);
        assert_eq!(decompress("br", &compressed), PAYLOAD);
    }

    #[test]
    fn identity_and_unknown_pass_through() {
        assert_eq!(decompress("identity", PAYLOAD), PAYLOAD);
        assert_eq!(decompress("", PAYLOAD), PAYLOAD);
        assert_eq!(decompress("weird-encoding", PAYLOAD), PAYLOAD);
    }

    #[test]
    fn empty_body_is_empty() {
        assert!(decompress("gzip", b"").is_empty());
    }

    #[test]
    fn malformed_stream_falls_back_to_input() {
        // "gzip" header but the bytes aren't valid gzip — return as-is
        // rather than erroring, so the caller can show it as binary.
        let garbage = b"not actually gzip";
        assert_eq!(decompress("gzip", garbage), garbage);
    }

    #[test]
    fn whitespace_in_encoding_token_tolerated() {
        let compressed = gzip(PAYLOAD);
        assert_eq!(decompress("  gzip  ", &compressed), PAYLOAD);
    }
}
