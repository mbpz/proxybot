import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const expectedVersion = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
const defaultExecutable = join(
  root,
  "target/release/bundle/macos/ProxyBot.app/Contents/MacOS/proxybot",
);
const executable = resolve(process.argv[2] ?? process.env.PROXYBOT_APP_EXECUTABLE ?? defaultExecutable);

if (!existsSync(executable)) {
  throw new Error(
    `Packaged ProxyBot executable not found at ${executable}. Build a Tauri app bundle first or pass its executable path.`,
  );
}

const workspace = mkdtempSync(join(tmpdir(), "proxybot-desktop-acceptance-"));
const reportPath = join(workspace, "desktop-acceptance.json");

try {
  const result = spawnSync(executable, ["--desktop-acceptance", workspace], {
    encoding: "utf8",
    env: { ...process.env, RUST_LOG: process.env.RUST_LOG ?? "warn" },
    timeout: 60_000,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const report = existsSync(reportPath) ? readFileSync(reportPath, "utf8") : "no report";
    throw new Error(
      `Packaged desktop acceptance exited ${result.status}.\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}\nreport:\n${report}`,
    );
  }
  if (!existsSync(reportPath)) {
    throw new Error(`Packaged desktop acceptance did not write ${reportPath}`);
  }

  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  const expected = {
    schema_version: 1,
    product_version: expectedVersion,
    ca_prepared: true,
    captured_request: {
      method: "GET",
      scheme: "https",
      host: "localhost",
      path: "/proxybot-acceptance",
      status: 403,
    },
    stopped_cleanly: true,
    restart_stopped_cleanly: true,
  };
  for (const [key, value] of Object.entries(expected)) {
    if (key === "captured_request") continue;
    if (report[key] !== value) {
      throw new Error(`Acceptance report ${key}=${JSON.stringify(report[key])}, expected ${JSON.stringify(value)}`);
    }
  }
  for (const [key, value] of Object.entries(expected.captured_request)) {
    if (report.captured_request?.[key] !== value) {
      throw new Error(
        `Acceptance report captured_request.${key}=${JSON.stringify(report.captured_request?.[key])}, expected ${JSON.stringify(value)}`,
      );
    }
  }
  for (const key of ["first_proxy_addr", "restart_proxy_addr"]) {
    const port = Number.parseInt(String(report[key]).split(":").at(-1) ?? "", 10);
    if (typeof report[key] !== "string" || !Number.isInteger(port) || port < 1 || port > 65_535) {
      throw new Error(`Acceptance report ${key} is not a socket address: ${JSON.stringify(report[key])}`);
    }
  }

  console.log(
    `desktop acceptance: ${report.product_version}, request #${report.captured_request.id}, ${report.first_proxy_addr} -> restart ${report.restart_proxy_addr}`,
  );
} finally {
  rmSync(workspace, { recursive: true, force: true });
}
