// Example: mock a 500 error response so the spec generator
// sees a clean JSON body instead of a raw server crash dump.
//
// Copy into ~/.proxybot/scripts/ for auto-loading, or write
// an inline script in the Rhai editor.
if status >= 500 {
    log("mocking 500 response body for " + host + path);
    rewrite_response_body(`{"error": "internal server error (mocked)"}`);
}
true