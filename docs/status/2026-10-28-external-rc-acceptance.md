# External Mac RC acceptance (0.2.0)

Use a Mac that has never had ProxyBot installed. Do not rebuild from source; use
the exact archive and checksum.

## Artifact

```text
proxybot-min-aarch64-apple-darwin.tar.gz
sha256 c777146c2a54b08b8bfec38a9bacd387c9fd4a35c3c65344b3cc521ad478fbb4
contents proxybot, LICENSE, THIRD_PARTY.md, uninstall.sh
```

Copy these to the test Mac:

- the archive and `.sha256`;
- `minimal/install.sh` (published separately in the GitHub release);
- `minimal/uninstall.sh` (standalone release asset; also inside the archive);
- `minimal/rc-smoke-test.sh` (copy next to the archive/install.sh).

## 1. Automated smoke test

```bash
shasum -a 256 proxybot-min-aarch64-apple-darwin.tar.gz
cat proxybot-min-aarch64-apple-darwin.tar.gz.sha256
sh rc-smoke-test.sh proxybot-min-aarch64-apple-darwin.tar.gz
```

`rc-smoke-test.sh` now requires the `.sha256` file, compares it before doing
anything, and fails if it is missing/empty/mismatched. It also records macOS and
architecture, kills its ProxyBot process, and removes its temporary prefix/data
on exit. Set `KEEP_TMP=1` if you need to retain failure logs.

Expected: `SMOKE RESULT: PASS (PASS=18 FAIL=0)`.

This verifies checksum, archive manifest/no sing-box, install into a temporary
prefix, 0.2.0 version, fresh DB, HTTP proxy, HTTPS MITM with the generated CA,
request persistence, stop, and uninstall. It does **not** modify system CA trust.

## 2. Manual CA / HTTPS acceptance

Install to the real user prefix:

```bash
PROXYBOT_LOCAL_ARCHIVE="$PWD/proxybot-min-aarch64-apple-darwin.tar.gz" sh install.sh
proxybot start
```

### Choose one client

Complete the manual pass with **one** client (Mac browser or iPhone), not a mix.
Record which one.

### UI vs CA page

- Web UI: `http://127.0.0.1:8899` — loopback only; never use this URL from iPhone.
- CA page: `http://<Mac LAN IP>:8090/` — LAN listeners; verify from the client.

Find the Mac LAN IP and confirm the page from the client:

```bash
route -n get default | grep -E 'interface|gateway'
ipconfig getifaddr en1   # or en0, whichever is the Wi-Fi interface
```

`http://<Mac LAN IP>:8090/` must load on the client. If it does not, check the
macOS firewall and whether the client is on the same LAN. A CA page that only
loads on the Mac's own browser is not sufficient for iPhone acceptance.

### CA install vs proxy config

These are independent:

1. Install the CA on the client.
2. Explicitly enable trust:
   - macOS: correct Keychain + "Always Trust";
   - iPhone: profile installed **and** Certificate Trust Settings enabled.
3. Set the client HTTP proxy to `<Mac LAN IP>:8088`.

A proxy setting alone does not trust the CA; a CA file on disk does not prove
interception.

### HTTPS request evidence

Visit a controlled HTTPS site without pinning, e.g. `https://example.com`.
Require a real request row in Live/API:

| Field | Required evidence |
| --- | --- |
| URL | expected HTTPS URL |
| Method | e.g. GET |
| Status | real HTTP status (e.g. 200) |
| Request/response | actual HTTP record, not only TLS/SNI metadata |

Browsers may redirect or generate extra requests; identify the target request
rather than requiring exactly one row. Page load alone or SNI-only rows are not
PASS.

### End order

1. Turn off the client HTTP proxy.
2. Revisit the HTTPS site and confirm normal internet.
3. Record ProxyBot state.
4. Stop ProxyBot and confirm process/port exit.
5. Run uninstall.
6. Check TUN/forwarding/processes/network.
7. Handle CA/data separately.

## 3. Clean data-directory check

Use a new `--data-dir` (or a fresh user account) and verify:

- DB is created without schema errors;
- `/api/requests` is reachable;
- a request is recorded;
- restart reads the record back;
- no provenance/index migration error.

`rc-smoke-test.sh` already exercises a fresh temp data directory.

## 4. Uninstall

First stop ProxyBot (`Ctrl+C`), then run the installed uninstaller:

```bash
~/.local/bin/share/proxybot/uninstall.sh
# or:
proxybot uninstall
# non-interactive full removal:
proxybot uninstall --purge --yes
```

The uninstaller prompts to remove user data and a trusted `ProxyBot Local CA`.
`--yes` removes both without prompting. The login keychain CA is removed when
possible; System keychain removal still needs sudo, and iPhone profiles/trust are
always manual.

Checks:

```bash
lsof -nP -iTCP:8088 -sTCP:LISTEN
pgrep -fl 'proxybot start|sing-box run'
launchctl print system/com.proxybot.gateway 2>&1 | head
ifconfig utun6 2>&1          # non-zero/absent is expected
sysctl net.inet.ip.forwarding
curl -sS -o /dev/null -w '%{http_code}
' https://example.com
```

Interpretation:

- `ifconfig utun6` failing because the interface is absent is the desired state;
  do not mark the test failed only from its exit code.
- `pgrep -fl` only matches known patterns; combine with port checks and
  `gateway status`/launchd state.
- forwarding must be `0`; ordinary network must work by actual HTTPS request.

CA/data handling is a separate acceptance item:

- user data is kept unless `--purge`;
- CA file is kept unless data is purged;
- system CA trust is **not** removed by uninstall; remove it separately;
- if iPhone was used, remove the test profile/trust unless intentionally kept.

Do not leave the temporary acceptance CA trusted.

## 5. Upgrade check (separate from fresh install)

Local evidence: 0.1.11 binary → 0.2.0 archive on the same data directory preserved
the existing request and added `source_type/peer_ip/device_id/connection_id/
identity_source/confidence` with defaults. On the external Mac, repeat with the
0.1.11 release artifact:

1. install 0.1.11;
2. create at least one real request row;
3. install 0.2.0 over it;
4. verify the old row is readable and provenance columns exist;
5. verify schema migration and normal use;
6. verify Gateway did not auto-start;
7. verify uninstall behavior.

A fresh database does not substitute for this upgrade path.

## Result template

```text
macOS:
arch:
archive checksum:
smoke:
HTTP proxy:
CA/HTTPS:
fresh data:
uninstall:
upgrade:
residual TUN/service:
Mac internet:
notes:
```


## RC Acceptance Gate (fill only after external execution)

- [ ] Archive SHA-256 matches the published `.sha256`
- [ ] Smoke PASS=18, no SKIP/NOT_VERIFIED
- [ ] Installed version 0.2.0 and expected path
- [ ] CA installed and trust explicitly enabled
- [ ] HTTPS MITM succeeds with expected URL/method/status
- [ ] ProxyBot request row persisted
- [ ] Client proxy restored; normal internet works
- [ ] Stop: process/port cleaned
- [ ] Uninstall: no residual TUN/forwarding/service
- [ ] CA/user-data handling documented and completed
- [ ] 0.1.11 → 0.2.0 upgrade verified separately
