# MCP stdio Adapter

ProxyBot includes an experimental MCP stdio Adapter for reading persisted
Captured Requests, classification data, Devices, and Alerts. It is an Advanced
integration, not part of the supported first-capture workflow.

## Current boundary

- `--mcp-stdio` starts the MCP Adapter only. It does **not** start a Capture
  Session or the desktop application.
- There is no supported `--headless` capture mode.
- Capture traffic with the desktop application before querying its persisted
  data from MCP.
- The Adapter exposes six tools. It does not expose `analyze_api` or
  `get_session`.
- `apply_rule` currently stores a request-scoped allow/block/log record. It does
  not create the Routing Rules used by the MITM Runtime.
- The `capture_traffic.filter` field is currently returned as metadata but is
  not yet applied to the persistence query.

These gaps are tracked in the [product roadmap](roadmap.md). MCP should remain a
thin Adapter over shared domain Modules rather than develop a second set of SQL
and rule semantics.

## Run from a source build

Build the Rust binary:

```bash
cargo build -p proxybot --release --locked --no-default-features
```

Start MCP stdio mode:

```bash
./target/release/proxybot --mcp-stdio
```

No normal console output is expected because stdin and stdout carry JSON-RPC.

Use the absolute path to that binary in an MCP client configuration:

```json
{
  "mcpServers": {
    "proxybot": {
      "command": "/absolute/path/to/proxybot/target/release/proxybot",
      "args": ["--mcp-stdio"]
    }
  }
}
```

Do not assume an installed application bundle uses the same executable path
until the release workflow is standardized.

## Available tools

| Tool | Purpose | Important inputs |
| --- | --- | --- |
| `capture_traffic` | Read recent persisted Captured Requests | `limit`, `since`; `filter` is not yet enforced |
| `classify_request` | Calculate Application Attribution evidence | `host`, optional `sni`, optional `dns_query` |
| `apply_rule` | Store a request-scoped app rule | `request_id`, `action`: `allow`, `block`, or `log` |
| `get_devices` | List persisted Devices | none |
| `get_alerts` | Query persisted Alerts | `since`, `limit`, `device_id`, `severity`, `acknowledged` |
| `acknowledge_alert` | Persist an Alert Acknowledgement | `alert_id` |

Alert Severity values are `Info`, `Warning`, and `Critical`.

## Security

An MCP client can read sensitive request metadata and change persisted state.
Only configure a trusted local client, protect the database and binary paths,
and do not paste captured secrets into remote models or public issues.

## Verification

The current Adapter uses MCP protocol version `2024-11-05`. A successful
initialization and `tools/list` call prove only the stdio contract. They do not
prove that desktop capture is running or that the release bundle is installable.
