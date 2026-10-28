# External Mac RC acceptance (0.2.0)

Use a Mac that has never had ProxyBot installed. Do not rebuild from source; use
the exact archive and checksum.

## Artifact

```text
proxybot-min-aarch64-apple-darwin.tar.gz
sha256 444f96c7607ecf7da82c7e89f5fceeb4cde670bf505b02bbe72a52e659f1d319
contents proxybot, LICENSE, THIRD_PARTY.md, uninstall.sh
```

Copy these to the test Mac:

- the archive and `.sha256`;
- `minimal/install.sh` (published separately in the GitHub release);
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

- Open the CA page printed by the CLI (`http://<Mac IP>:8090/`);
- Install and trust the CA in Keychain Access / System Settings;
- Configure a test client to use `<Mac IP>:8088`;
- Visit a controlled HTTPS site without certificate pinning;
- Confirm the request appears with decrypted URL/method/status;
- Restore the client proxy setting and confirm normal internet;
- Do not claim pinned apps or all Apple services are decryptable.

CA installation, trust, and proxy configuration are separate steps. A CA file on
disk is not proof of HTTPS interception.

## 3. Clean data-directory check

Use a new `--data-dir` (or a fresh user account) and verify:

- DB is created without schema errors;
- `/api/requests` is reachable;
- a request is recorded;
- restart reads the record back;
- no provenance/index migration error.

`rc-smoke-test.sh` already exercises a fresh temp data directory.

## 4. Uninstall

Use the installed uninstaller, not manual deletion:

```bash
~/.local/bin/share/proxybot/uninstall.sh
```

Confirm binary/license removal, no Gateway process/service/TUN, forwarding 0, and
Mac internet. User data/CA are kept unless `--purge`; CA trust must be removed
separately. Document that uninstalling the binary does not remove the trusted CA.

## 5. Upgrade check

Local evidence: 0.1.11 binary → 0.2.0 archive on the same data directory preserved
the existing request and added `source_type/peer_ip/device_id/connection_id/
identity_source/confidence` with defaults. On the external Mac, repeat with the
0.1.11 release artifact if available, then verify data and no auto-start.

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
