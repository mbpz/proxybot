import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repositoryRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const checkerPath = join(repositoryRoot, "scripts", "check_parity_matrix.mjs");
const commit = "6a676d631820b577cf3a651c78d856733a7df995";
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
      source_license: "AGPL-3.0-or-later Community source",
      public_artifact: `https://github.com/RockxyApp/Rockxy/tree/${commit}`,
      excluded_artifacts: [
        "official downstream DMG",
        "private/Pro behavior",
      ],
    },
    capabilities: Array.from({ length: 47 }, (_, index) => {
      const category = categories[index % categories.length];
      return {
        id: `RXC-${String(index + 1).padStart(3, "0")}`,
        category,
        capability: `${category} capability ${index + 1}`,
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
      };
    }),
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
  const output = `${result.stdout}\n${result.stderr}`;
  assert.ok(output.includes(message), output);
}

test("accepts a complete pinned parity manifest", () => {
  const result = runChecker(validManifest());
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /47 capabilities validated/);
});

test("accepts the fixed capability inventory in any row order", () => {
  const manifest = validManifest();
  manifest.capabilities.reverse();
  const result = runChecker(manifest);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /47 capabilities validated/);
});

test("rejects coordinated reference drift from the fixed M0 contract", () => {
  const manifest = validManifest();
  const driftedCommit = "0123456789abcdef0123456789abcdef01234567";
  manifest.reference.commit = driftedCommit;
  manifest.reference.captured_at = "2026-08-20";
  manifest.reference.source_license = "Community clean-room review";
  manifest.reference.public_artifact =
    `https://github.com/RockxyApp/Rockxy/tree/${driftedCommit}`;
  manifest.reference.excluded_artifacts = [
    "private/Pro behavior",
    "official downstream DMG",
  ];
  for (const row of manifest.capabilities) {
    row.target_evidence = [
      `https://github.com/RockxyApp/Rockxy/blob/${driftedCommit}/README.md`,
    ];
  }

  const result = runChecker(manifest);
  assert.equal(result.status, 1, result.stdout || result.stderr);
  const output = `${result.stdout}\n${result.stderr}`;
  for (const message of [
    `reference commit must be ${commit}`,
    "reference captured_at must be 2026-08-19",
    "reference source_license must be AGPL-3.0-or-later Community source",
    `reference public_artifact must be https://github.com/RockxyApp/Rockxy/tree/${commit}`,
    "reference excluded_artifacts must be official downstream DMG, private/Pro behavior in that order",
  ]) {
    assert.ok(output.includes(message), output);
  }
});

test("rejects a missing capability from the fixed inventory", () => {
  const manifest = validManifest();
  manifest.capabilities.splice(25, 1);
  assertFailure(manifest, "missing capability id RXC-026");
});

test("rejects a capability outside the fixed inventory", () => {
  const manifest = validManifest();
  manifest.capabilities[46].id = "RXC-048";
  assertFailure(manifest, "unexpected capability id RXC-048");
});

test("rejects undeclared top-level manifest fields", () => {
  const manifest = validManifest();
  manifest.unexpected = true;
  assertFailure(manifest, "manifest has undeclared field unexpected");
});

test("rejects undeclared reference fields", () => {
  const manifest = validManifest();
  manifest.reference.unexpected = true;
  assertFailure(manifest, "reference has undeclared field unexpected");
});

test("rejects duplicate capability IDs", () => {
  const manifest = validManifest();
  manifest.capabilities[1].id = manifest.capabilities[0].id;
  assertFailure(manifest, "duplicate capability id RXC-001");
});

test("rejects undeclared capability fields", () => {
  const manifest = validManifest();
  manifest.capabilities[0].unexpected = true;
  assertFailure(manifest, "capability RXC-001 has undeclared field unexpected");
});

test("rejects ProxyBot evidence with an unknown prefix", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_evidence = [
    "source:package.json",
    "binary:package.json",
  ];
  assertFailure(
    manifest,
    "capability RXC-001 has ProxyBot evidence with unknown prefix binary:package.json",
  );
});

test("rejects ProxyBot evidence with a missing prefix", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_evidence = [
    "source:package.json",
    "package.json",
  ];
  assertFailure(
    manifest,
    "capability RXC-001 has ProxyBot evidence with missing prefix package.json",
  );
});

test("rejects an absolute ProxyBot evidence path", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_evidence = [
    "source:package.json",
    "source:/etc/passwd",
  ];
  assertFailure(
    manifest,
    "capability RXC-001 has absolute ProxyBot evidence path source:/etc/passwd",
  );
});

test("rejects a traversing ProxyBot evidence path", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_evidence = [
    "source:package.json",
    "source:../package.json",
  ];
  assertFailure(
    manifest,
    "capability RXC-001 has unsafe ProxyBot evidence path source:../package.json",
  );
});

test("rejects a nonexistent ProxyBot evidence path", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_evidence = [
    "source:package.json",
    "source:path/that/does/not/exist",
  ];
  assertFailure(
    manifest,
    "capability RXC-001 has missing ProxyBot evidence path source:path/that/does/not/exist",
  );
});

test("rejects a non-string ProxyBot evidence entry", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_evidence = ["source:package.json", 42];
  assertFailure(
    manifest,
    "capability RXC-001 has non-string ProxyBot evidence",
  );
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

test("rejects Missing claims without existing docs evidence", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_status = "Missing";
  manifest.capabilities[0].proxybot_evidence_grade = "documented";
  manifest.capabilities[0].proxybot_evidence = ["source:package.json"];
  assertFailure(
    manifest,
    "Missing claim requires documented evidence and an existing docs item",
  );
});

test("rejects private claims without existing docs evidence", () => {
  const manifest = validManifest();
  manifest.capabilities[0].scope = "private";
  manifest.capabilities[0].proxybot_status = "Out-of-scope private";
  manifest.capabilities[0].proxybot_evidence_grade = "documented";
  manifest.capabilities[0].proxybot_evidence = ["source:package.json"];
  assertFailure(
    manifest,
    "Out-of-scope private claim requires documented evidence and an existing docs item",
  );
});

test("rejects future claims without existing docs evidence", () => {
  const manifest = validManifest();
  manifest.capabilities[0].scope = "future";
  manifest.capabilities[0].proxybot_status = "Future-not-shipped";
  manifest.capabilities[0].proxybot_evidence_grade = "documented";
  manifest.capabilities[0].proxybot_evidence = ["source:package.json"];
  assertFailure(
    manifest,
    "Future-not-shipped claim requires documented evidence and an existing docs item",
  );
});

test("rejects a missing inventory category", () => {
  const manifest = validManifest();
  manifest.capabilities = manifest.capabilities.filter(
    (row) => row.category !== "performance",
  );
  assertFailure(manifest, "missing required inventory category: performance");
});
