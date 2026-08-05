import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const semverPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

function path(relative) {
  return root + relative;
}

function read(relative) {
  return readFileSync(path(relative), "utf8");
}

function readJson(relative) {
  return JSON.parse(read(relative));
}

function manifestVersion(relative) {
  const match = read(relative).match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error("No package version in " + relative);
  return match[1];
}

function lockedPackageVersion(name) {
  for (const block of read("Cargo.lock").split("\n\n")) {
    if (!block.includes('name = "' + name + '"')) continue;
    const match = block.match(/^version = "([^"]+)"/m);
    if (match) return match[1];
  }
  throw new Error("No Cargo.lock package named " + name);
}

function fail(errors, message) {
  errors.push(message);
}

function check(tag) {
  const errors = [];
  const version = readJson("package.json").version;
  if (!semverPattern.test(version)) {
    fail(errors, "package.json version is not supported semver: " + version);
  }

  for (const relative of ["proxybot-core/Cargo.toml", "src-tauri/Cargo.toml"]) {
    const actual = manifestVersion(relative);
    if (actual !== version) fail(errors, relative + " is " + actual + ", expected " + version);
  }
  for (const name of ["proxybot", "proxybot-core"]) {
    const actual = lockedPackageVersion(name);
    if (actual !== version) fail(errors, "Cargo.lock " + name + " is " + actual + ", expected " + version);
  }

  const tauri = readJson("src-tauri/tauri.conf.json");
  if (tauri.version !== "../package.json") {
    fail(errors, "Tauri must read version from ../package.json");
  }
  if (tauri.identifier !== "com.mbpz.proxybot") {
    fail(errors, "Tauri bundle identifier must remain com.mbpz.proxybot");
  }
  if (!read("vite.config.ts").includes("__APP_VERSION__: JSON.stringify(packageJson.version)")) {
    fail(errors, "Vite must inject package.json version as __APP_VERSION__");
  }
  const updateHook = read("src/hooks/useUpdateCheck.ts");
  if (!updateHook.includes("CURRENT_VERSION = __APP_VERSION__")) {
    fail(errors, "Update check must consume the Vite-injected app version");
  }
  const mcp = read("src-tauri/src/mcp/server.rs");
  if (!mcp.includes('env!("CARGO_PKG_VERSION")')) {
    fail(errors, "MCP serverInfo must consume CARGO_PKG_VERSION");
  }

  const release = read(".github/workflows/release.yml");
  if (!release.includes("scripts/version.mjs --check --tag")) {
    fail(errors, "Release workflow must verify its tag against the product version");
  }
  if (!release.includes("tauri build")) {
    fail(errors, "Release workflow must build with the Tauri bundler");
  }
  if (!release.includes("attestations: write")) {
    fail(errors, "Release workflow must be allowed to publish build provenance");
  }
  for (const action of [
    "actions/attest-build-provenance@v4",
    "actions/upload-artifact@v7",
    "actions/download-artifact@v8",
    "softprops/action-gh-release@v3",
  ]) {
    if (!release.includes(action)) fail(errors, "Release workflow must use " + action);
  }
  if (!release.includes('ref: ${{ inputs.version || github.ref }}')) {
    fail(errors, "Release workflow must build the exact requested tag");
  }
  if (!release.includes('CFBundleIdentifier raw "$APP_PATH/Contents/Info.plist"')) {
    fail(errors, "Release workflow must verify the stable bundle identifier");
  }
  if (release.includes('mkdir -p "ProxyBot.app"') || release.includes("codesign --force --deep --sign -")) {
    fail(errors, "Release workflow still contains a hand-built or ad-hoc-signed app bundle");
  }

  if (tag) {
    const expectedTag = "v" + version;
    if (tag !== expectedTag) fail(errors, "release tag is " + tag + ", expected " + expectedTag);
  }

  if (errors.length > 0) {
    for (const error of errors) console.error("version check: " + error);
    process.exit(1);
  }
  console.log("version check: " + version + " is consistent");
}

function writeVersion(version) {
  if (!semverPattern.test(version)) {
    throw new Error("Expected a version such as 1.3.1, received: " + version);
  }

  const packageJson = readJson("package.json");
  packageJson.version = version;
  writeFileSync(path("package.json"), JSON.stringify(packageJson, null, 2) + "\n");

  for (const relative of ["proxybot-core/Cargo.toml", "src-tauri/Cargo.toml"]) {
    const source = read(relative);
    const updated = source.replace(/^version\s*=\s*"[^"]+"/m, 'version = "' + version + '"');
    writeFileSync(path(relative), updated);
  }

  const blocks = read("Cargo.lock").split("\n\n").map((block) => {
    if (!block.includes('name = "proxybot"') && !block.includes('name = "proxybot-core"')) {
      return block;
    }
    return block.replace(/^version = "[^"]+"/m, 'version = "' + version + '"');
  });
  writeFileSync(path("Cargo.lock"), blocks.join("\n\n"));
}

const args = process.argv.slice(2);
const setIndex = args.indexOf("--set");
if (setIndex >= 0) {
  const version = args[setIndex + 1];
  if (!version) throw new Error("--set requires a version");
  writeVersion(version);
}
const tagIndex = args.indexOf("--tag");
if (tagIndex >= 0 && !args[tagIndex + 1]) throw new Error("--tag requires a tag");
check(tagIndex >= 0 ? args[tagIndex + 1] : undefined);
