# Review Request: Unit Tests Added

**Ready for Review: YES**

## Summary

Added comprehensive unit tests for two core modules.

## Files Changed

### src-tauri/src/dns.rs

Added `#[cfg(test)] mod tests` with 4 test cases for `parse_dns_query()`:

- `test_simple_domain` - Tests parsing "example.com" from DNS wire format with proper 12-byte header prefix
- `test_subdomain` - Tests parsing "www.example.com" with multi-label domain
- `test_empty_query` - Tests that empty buffer returns None
- `test_truncated_query` - Tests that truncated label (length 5 but only 2 bytes available) returns None

**Note:** DNS query data must include the 12-byte DNS header prefix that `parse_dns_query` expects.

### src-tauri/src/proxy.rs

Added `#[cfg(test)] mod tests` with 5 test cases:

- `test_parse_http_response_headers` - Verifies HTTP response headers are parsed correctly
- `test_decode_body_utf8` - Verifies valid UTF-8 body decodes correctly
- `test_decode_body_binary_fallback` - Verifies binary bytes fall back to "[Binary N bytes]" format
- `test_decode_body_truncation` - Verifies large bodies (15000 bytes) are truncated without panic
- `test_extract_response_body` - Verifies response body extraction from HTTP response data
- `test_extract_response_body_empty` - Verifies empty body case is handled

**Note:** Header comparison uses `to_lowercase()` since header parsing preserves original case.

## Test Results

```
cargo test: 14 passed (3 suites, 0.00s)
```

All tests pass successfully.
