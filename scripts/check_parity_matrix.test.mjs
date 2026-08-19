import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repositoryRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const checkerPath = join(repositoryRoot, "scripts", "check_parity_matrix.mjs");
const commit = "0123456789abcdef0123456789abcdef01234567";
const categories = [
  "capture",
  "filtering",
  "focus-noise",
  "workspaces",
  "assistant",
  "mcp",
  "setup",
  "certificates",
  "proxy-rules",
  "compose-compare",
  "sessions-export",
  "scripting",
  "protocols",
  "logs",
  "nearby-transfer",
  "updates",
  "security",
  "accessibility",
  "performance",
];

function validManifest() {
  return {
    schema_version: 1,
    reference: {
      repository: "RockxyApp/Rockxy",
      commit,
      captured_at: "2026-08-19",
      source_license: "Community clean-room review",
      public_artifact: "https://github.com/RockxyApp/Rockxy",
      excluded_artifacts: ["private build artifacts"],
    },
    capabilities: categories.map((category, index) => ({
      id: `RXC-${String(index + 1).padStart(3, "0")}`,
      category,
      capability: `${category} capability`,
      scope: "community",
      target_evidence_grade: "source-backed",
      target_evidence: [
        `https://github.com/RockxyApp/Rockxy/blob/${commit}/README.md`,
      ],
      proxybot_status: "Partial",
      proxybot_evidence_grade: "source-backed",
      proxybot_evidence: ["source:package.json"],
      owner: "M1",
      acceptance:
        "A hand-written acceptance statement that independently verifies this capability.",
    })),
  };
}

function runChecker(manifest) {
  const directory = mkdtempSync(join(tmpdir(), "parity-matrix-"));
  const markdownPath = join(directory, "matrix.md");
  const markdown = [
    "# Rockxy Community Matrix",
    "",
    "<!-- parity-matrix:start -->",
    "```json",
    JSON.stringify(manifest, null, 2),
    "```",
    "<!-- parity-matrix:end -->",
    "",
  ].join("\n");
  writeFileSync(markdownPath, markdown);
  return spawnSync(process.execPath, [checkerPath, markdownPath], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
}

function assertFailure(manifest, message) {
  const result = runChecker(manifest);
  assert.equal(result.status, 1, result.stdout || result.stderr);
  assert.match(`${result.stdout}\n${result.stderr}`, new RegExp(message));
}

test("accepts a complete pinned parity manifest", () => {
  const result = runChecker(validManifest());
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /19 capabilities validated/);
});

test("rejects duplicate capability IDs", () => {
  const manifest = validManifest();
  manifest.capabilities[1].id = manifest.capabilities[0].id;
  assertFailure(manifest, "duplicate capability id RXC-001");
});

test("rejects a row without an owner", () => {
  const manifest = validManifest();
  manifest.capabilities[0].owner = "";
  assertFailure(manifest, "capability RXC-001 is missing owner");
});

test("rejects a row without independent acceptance criteria", () => {
  const manifest = validManifest();
  manifest.capabilities[0].acceptance = "";
  assertFailure(manifest, "capability RXC-001 is missing acceptance criteria");
});

test("rejects floating Rockxy references", () => {
  const manifest = validManifest();
  manifest.capabilities[0].target_evidence[0] =
    "https://github.com/RockxyApp/Rockxy/blob/main/README.md";
  assertFailure(manifest, "floating Rockxy reference");
});

test("rejects documented-only evidence for Community parity scope", () => {
  const manifest = validManifest();
  manifest.capabilities[0].target_evidence_grade = "documented";
  assertFailure(
    manifest,
    "target evidence grade documented is below source-backed",
  );
});

test("rejects unsupported Present claims", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_status = "Present";
  assertFailure(
    manifest,
    "Present claim requires test-backed or release-proven ProxyBot evidence",
  );
});

test("rejects a missing inventory category", () => {
  const manifest = validManifest();
  manifest.capabilities = manifest.capabilities.filter(
    (row) => row.category !== "performance",
  );
  assertFailure(manifest, "missing required inventory category: performance");
});
