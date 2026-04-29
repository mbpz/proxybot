# ProxyBot

## Project
A macOS desktop proxy tool for developers. Phone and PC on the same LAN — phone sets gateway/DNS to PC IP, and ProxyBot captures + decrypts all HTTPS/WSS traffic from the phone. Traffic is classified by app (WeChat, Douyin, Alipay), then by domain within each app.

**Phase 1:** macOS only. Developer-facing tool.
**Phase 2:** Windows support (later).

## Stack
- **Proxy core:** Rust — hyper + rustls + tokio + tokio-tungstenite
- **Desktop UI:** Tauri v2 + React + TypeScript + shadcn/ui
- **App classification:** DNS correlation + domain rule sets (WeChat/Douyin/Alipay) + SNI inspection
- **Network routing:** macOS pf transparent proxy + IP forwarding
- **DNS server:** Built-in DNS server on PC to log phone's DNS queries

## Key Architecture
1. PC acts as both gateway and DNS server for the phone
2. pf redirects port 80/443 traffic to local proxy (needs admin on first launch)
3. MITM SSL: generate root CA on first launch, user installs it on phone
4. Per-connection leaf certificates signed by root CA
5. DNS query log correlated with connections for app identification
6. Domain rule library: WeChat (`*.weixin.qq.com`, `*.wechat.com`, `*.qq.com`), Douyin (`*.douyin.com`, `*.tiktokv.com`), Alipay (`*.alipay.com`, `*.alipayusercontent.com`)

## Three Man Team
Available agents: Arch (Architect), Bob (Builder), Richard (Reviewer)

## Agent skills

### Issue tracker

GitHub issues. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the canonical five-label vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context — one `CONTEXT.md` at the repo root, `docs/adr/` for architectural decisions. See `docs/agents/domain.md`.

<!-- code-review-graph MCP tools -->
## MCP Tools: code-review-graph

**IMPORTANT: This project has a knowledge graph. ALWAYS use the
code-review-graph MCP tools BEFORE using Grep/Glob/Read to explore
the codebase.** The graph is faster, cheaper (fewer tokens), and gives
you structural context (callers, dependents, test coverage) that file
scanning cannot.

### When to use graph tools FIRST

- **Exploring code**: `semantic_search_nodes` or `query_graph` instead of Grep
- **Understanding impact**: `get_impact_radius` instead of manually tracing imports
- **Code review**: `detect_changes` + `get_review_context` instead of reading entire files
- **Finding relationships**: `query_graph` with callers_of/callees_of/imports_of/tests_for
- **Architecture questions**: `get_architecture_overview` + `list_communities`

Fall back to Grep/Glob/Read **only** when the graph doesn't cover what you need.

### Key Tools

| Tool | Use when |
|------|----------|
| `detect_changes` | Reviewing code changes — gives risk-scored analysis |
| `get_review_context` | Need source snippets for review — token-efficient |
| `get_impact_radius` | Understanding blast radius of a change |
| `get_affected_flows` | Finding which execution paths are impacted |
| `query_graph` | Tracing callers, callees, imports, tests, dependencies |
| `semantic_search_nodes` | Finding functions/classes by name or keyword |
| `get_architecture_overview` | Understanding high-level codebase structure |
| `refactor_tool` | Planning renames, finding dead code |

### Workflow

1. The graph auto-updates on file changes (via hooks).
2. Use `detect_changes` for code review.
3. Use `get_affected_flows` to understand impact.
4. Use `query_graph` pattern="tests_for" to check coverage.
