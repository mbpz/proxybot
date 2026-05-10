// Stdio transport for MCP server - handles JSON-RPC requests via stdin/stdout

use std::io::{self, BufRead, Write};

use crate::mcp::{JsonRpcRequest, JsonRpcResponse};

/// Run the MCP server using stdio transport.
/// Reads JSON-RPC requests from stdin and writes responses to stdout.
pub fn run_stdio_server<F>(handler: F) -> io::Result<()>
where
    F: Fn(JsonRpcRequest) -> JsonRpcResponse + Send + Sync + 'static,
{
    let stdin = io::stdin();
    let mut reader = BufRead::lines(stdin.lock());
    let mut stdout = io::stdout().lock();

    for line in reader {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse the request
        let request: Result<JsonRpcRequest, _> = serde_json::from_str(trimmed);
        let response = match request {
            Ok(req) => handler(req),
            Err(e) => JsonRpcResponse::error(
                None,
                super::JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {}", e),
                    data: None,
                },
            ),
        };

        // Write response
        let response_json = serde_json::to_string(&response).unwrap_or_else(|_| {
            r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Serialization error"}}"#.to_string()
        });
        writeln!(stdout, "{}", response_json)?;
    }

    Ok(())
}

/// Stdio transport entry point for CLI
pub fn start_stdio_mode() {
    use super::{McpServer, McpState};
    let state: std::sync::Arc<McpState> = std::sync::Arc::new(McpState::new_insecure());
    let server = McpServer::new(state);
    run_stdio_server(move |req| server.handle_request(req))
        .expect("Stdio server error");
}