# MCP Server Integration

ProxyBot v1.2+ ships an MCP Server (Model Context Protocol) that exposes proxy
capabilities as tools consumable by any MCP-compatible AI agent — Claude Desktop,
Cursor, Windsurf, Claude Code, and others.

## Quick Start

### 1. Verify ProxyBot MCP mode

```bash
# ProxyBot starts in MCP stdio mode with --mcp-stdio
proxybot --mcp-stdio
```

You should see no output — MCP uses stdio JSON-RPC, so it waits for
the client to initiate the handshake.

### 2. Configure Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "proxybot": {
      "command": "/Applications/ProxyBot.app/Contents/MacOS/proxybot",
      "args": ["--mcp-stdio"]
    }
  }
}
```

Restart Claude Desktop. You'll see a hammer icon 🔨 in the chat input indicating
MCP tools are available.

### 3. Configure Cursor

In Cursor settings → MCP, add a new server:

```json
{
  "mcpServers": {
    "proxybot": {
      "command": "/Applications/ProxyBot.app/Contents/MacOS/proxybot",
      "args": ["--mcp-stdio"]
    }
  }
}
```

### 4. Configure Windsurf

Edit `~/.windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "proxybot": {
      "command": "/Applications/ProxyBot.app/Contents/MacOS/proxybot",
      "args": ["--mcp-stdio"]
    }
  }
}
```

## Available Tools

ProxyBot exposes 5+ MCP tools:

| Tool | Description | Parameters |
|------|-------------|------------|
| `capture_traffic` | Get captured HTTP requests | `limit` (int, optional) |
| `classify_request` | Classify a host by app | `host` (string) |
| `apply_rule` | Apply a routing rule | `pattern`, `value`, `action`, `target` |
| `get_devices` | List connected devices | — |
| `get_alerts` | Get anomaly alerts | `severity` (optional: SEV1/SEV2/SEV3) |
| `analyze_api` | Analyze captured API traffic | `session_id` (optional) |
| `get_session` | Get current session info | — |

### Example Claude Desktop Prompts

After connecting ProxyBot MCP Server, try these prompts in Claude Desktop:

**Capture and classify traffic:**
> "Show me the last 20 captured requests from my phone, classified by app."

Claude will call `capture_traffic(limit: 20)` then `classify_request(host: ...)` for each.

**Add a routing rule:**
> "Add a rule to reject all traffic to ad domains."

Claude will call `apply_rule(pattern: "domain-suffix", value: "doubleclick.net", action: "REJECT")`.

**Audit AI API usage:**
> "Analyze the last 100 requests and show me which AI providers are being called and the estimated token cost."

Claude will call `capture_traffic(limit: 100)` then `analyze_api()`.

## Headless Mode

ProxyBot can run in headless MCP-only mode without the GUI:

```bash
# Start proxy in the background, then launch MCP server
proxybot --headless &
proxybot --mcp-stdio
```

Or use a launchd plist for persistent background service:

```xml
<!-- ~/Library/LaunchAgents/com.proxybot.mcp.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.proxybot.mcp</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Applications/ProxyBot.app/Contents/MacOS/proxybot</string>
        <string>--mcp-stdio</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

## Troubleshooting

### "MCP Server not found"
- Verify the proxybot binary path: `which proxybot`
- For Tauri app bundle: use `/Applications/ProxyBot.app/Contents/MacOS/proxybot`
- For Homebrew CLI: use `$(brew --prefix)/bin/proxybot`

### "Tool call failed: proxy not running"
MCP tools require the proxy to be capturing traffic. Start the proxy first:
- In GUI: click "Start Proxy"
- In CLI: `proxybot --headless &`

### "No traffic captured"
1. Ensure your phone is configured to use your Mac as gateway and DNS
2. Verify pf is enabled: `sudo pfctl -s nat | grep proxybot`
3. Check DNS tab for incoming queries
4. Open `http://example.com` in Safari on the phone

### "Certificate error on phone"
The CA certificate must be installed and trusted on the phone:
1. Export CA from Certs tab → `~/.proxybot/ca.crt`
2. AirDrop to phone
3. Settings → General → About → Certificate Trust Settings → Enable full trust

Or use the QR code in the Tauri GUI Certs tab for one-scan install.

## Technical Details

- **Transport**: stdio (stdin/stdout JSON-RPC 2.0)
- **Protocol**: MCP 1.0 (Model Context Protocol)
- **Binary**: Same `proxybot` binary, `--mcp-stdio` flag
- **No network**: All communication is local over stdio pipes
- **Concurrent**: Tools are called sequentially within a session (JSON-RPC req/resp)
