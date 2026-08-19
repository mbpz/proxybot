import { existsSync, readFileSync } from "node:fs";
import { resolve, relative, isAbsolute, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const START_MARKER = "<!-- parity-matrix:start -->";
const END_MARKER = "<!-- parity-matrix:end -->";
const REPOSITORY = "RockxyApp/Rockxy";
const EVIDENCE_GRADES = [
  "documented",
  "source-backed",
  "test-backed",
  "observable-build",
  "release-proven",
];
const SCOPES = ["community", "private", "future"];
const STATUSES = [
  "Present",
  "Partial",
  "Missing",
  "Out-of-scope private",
  "Future-not-shipped",
];
const REQUIRED_FIELDS = [
  "id",
  "category",
  "capability",
  "scope",
  "target_evidence_grade",
  "target_evidence",
  "proxybot_status",
  "proxybot_evidence_grade",
  "proxybot_evidence",
  "owner",
  "acceptance",
];
const REQUIRED_CATEGORIES = [
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

const checkerRoot = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(checkerRoot, "..");

function errorMessage(message) {
  return `parity matrix: ${message}`;
}

export function extractManifest(markdown) {
  if (typeof markdown !== "string") {
    throw new Error("manifest source is not text");
  }

  const starts = markdown.split(START_MARKER).length - 1;
  const ends = markdown.split(END_MARKER).length - 1;
  if (starts !== 1 || ends !== 1) {
    throw new Error("expected exactly one parity matrix envelope");
  }

  const start = markdown.indexOf(START_MARKER) + START_MARKER.length;
  const end = markdown.indexOf(END_MARKER);
  if (end < start) {
    throw new Error("parity matrix envelope markers are out of order");
  }
  const body = markdown.slice(start, end);
  const jsonMatches = [...body.matchAll(/```json\s*([\s\S]*?)\s*```/g)];
  if (jsonMatches.length !== 1) {
    throw new Error("parity matrix envelope must contain exactly one json code block");
  }
  const jsonMatch = jsonMatches[0];
  const before = body.slice(0, jsonMatch.index).trim();
  const after = body.slice(jsonMatch.index + jsonMatch[0].length).trim();
  if (before || after) {
    throw new Error("parity matrix envelope must contain only one json object");
  }
  try {
    return JSON.parse(jsonMatch[1]);
  } catch (error) {
    throw new Error(`invalid parity matrix JSON: ${error.message}`);
  }
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function rank(grade) {
  return EVIDENCE_GRADES.indexOf(grade);
}

function pathExistsBelowRepository(item) {
  if (typeof item !== "string") return false;
  const separator = item.indexOf(":");
  if (separator <= 0) return true;
  const pathPart = item.slice(separator + 1);
  if (!pathPart || isAbsolute(pathPart)) return false;
  const segments = pathPart.split(/[\\/]+/);
  if (segments.includes("..")) return false;
  const candidate = resolve(repositoryRoot, pathPart);
  const rel = relative(repositoryRoot, candidate);
  return rel !== "" && !rel.startsWith("..") && !isAbsolute(rel) && existsSync(candidate);
}

function validateReference(reference, violations) {
  if (!isObject(reference)) {
    violations.push("reference must be an object");
    return;
  }
  for (const field of [
    "repository",
    "commit",
    "captured_at",
    "source_license",
    "public_artifact",
    "excluded_artifacts",
  ]) {
    if (!(field in reference)) violations.push(`reference is missing ${field}`);
  }
  if (reference.repository !== REPOSITORY) {
    violations.push(`reference repository must be ${REPOSITORY}`);
  }
  if (typeof reference.commit !== "string" || !/^[0-9a-f]{40}$/.test(reference.commit)) {
    violations.push("reference commit must be a 40-character lowercase hexadecimal SHA");
  }
  if (typeof reference.captured_at !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(reference.captured_at)) {
    violations.push("reference captured_at must be YYYY-MM-DD");
  }
  if (!isNonEmptyString(reference.source_license)) {
    violations.push("reference source_license must be non-empty");
  }
  if (!isNonEmptyString(reference.public_artifact)) {
    violations.push("reference public_artifact must be non-empty");
  }
  if (!Array.isArray(reference.excluded_artifacts)) {
    violations.push("reference excluded_artifacts must be an array");
  }
}

function validateTargetEvidence(row, reference, violations) {
  const evidence = row.target_evidence;
  if (!Array.isArray(evidence) || evidence.length === 0) {
    violations.push(`capability ${row.id ?? "<unknown>"} requires target evidence`);
    return;
  }
  for (const item of evidence) {
    if (typeof item !== "string") {
      violations.push(`capability ${row.id ?? "<unknown>"} has invalid target evidence`);
      continue;
    }
    if (!item.startsWith("https://github.com/RockxyApp/Rockxy/")) {
      violations.push(`capability ${row.id ?? "<unknown>"} has invalid Rockxy target reference`);
      continue;
    }
    if (/\/(?:main|master)(?:\/|$)/.test(item) || /(?:[?&])ref=(?:main|master)(?:&|$)/.test(item)) {
      violations.push(`capability ${row.id ?? "<unknown>"} has floating Rockxy reference`);
    }
    const commit = typeof reference?.commit === "string" ? reference.commit : "<invalid-commit>";
    if (!item.includes(`/blob/${commit}/`) && !item.includes(`/tree/${commit}/`)) {
      violations.push(`capability ${row.id ?? "<unknown>"} target evidence is not pinned to reference commit`);
    }
  }
}

function validateProxyEvidence(row, violations) {
  const evidence = row.proxybot_evidence;
  if (!Array.isArray(evidence) || evidence.length === 0) {
    violations.push(`capability ${row.id ?? "<unknown>"} requires ProxyBot evidence`);
    return;
  }
  for (const item of evidence) {
    if (typeof item === "string" && /^(source|test|docs):/.test(item) && !pathExistsBelowRepository(item)) {
      violations.push(`capability ${row.id ?? "<unknown>"} has missing or unsafe ProxyBot evidence path ${item}`);
    }
  }
}

export function validateManifest(manifest) {
  const violations = [];
  if (!isObject(manifest)) return ["manifest must be a JSON object"];
  if (manifest.schema_version !== 1) violations.push("schema_version must be 1");
  validateReference(manifest.reference, violations);
  if (!Array.isArray(manifest.capabilities)) {
    violations.push("capabilities must be an array");
    return violations;
  }

  const ids = new Set();
  const categories = new Set();
  for (const row of manifest.capabilities) {
    if (!isObject(row)) {
      violations.push("capability row must be an object");
      continue;
    }
    const id = typeof row.id === "string" ? row.id : "<unknown>";
    for (const field of REQUIRED_FIELDS) {
      if (!(field in row)) violations.push(`capability ${id} is missing ${field}`);
    }
    if (typeof row.id !== "string" || !/^RXC-[0-9]{3}$/.test(row.id)) {
      violations.push(`capability ${id} has invalid ID`);
    } else if (ids.has(row.id)) {
      violations.push(`duplicate capability id ${row.id}`);
    } else {
      ids.add(row.id);
    }
    if (!isNonEmptyString(row.category)) {
      violations.push(`capability ${id} is missing category`);
    } else {
      categories.add(row.category);
      if (!REQUIRED_CATEGORIES.includes(row.category)) {
        violations.push(`capability ${id} has unsupported category ${row.category}`);
      }
    }
    if (!isNonEmptyString(row.capability)) violations.push(`capability ${id} is missing capability name`);
    if (!SCOPES.includes(row.scope)) violations.push(`capability ${id} has invalid scope ${row.scope}`);
    if (!EVIDENCE_GRADES.includes(row.target_evidence_grade)) {
      violations.push(`capability ${id} has invalid target evidence grade ${row.target_evidence_grade}`);
    }
    if (!STATUSES.includes(row.proxybot_status)) violations.push(`capability ${id} has invalid ProxyBot status ${row.proxybot_status}`);
    if (!EVIDENCE_GRADES.includes(row.proxybot_evidence_grade)) {
      violations.push(`capability ${id} has invalid ProxyBot evidence grade ${row.proxybot_evidence_grade}`);
    }
    if (typeof row.owner !== "string" || !/^M(?:[0-9]|1[0-7])$/.test(row.owner)) {
      if (!row.owner) violations.push(`capability ${id} is missing owner`);
      else violations.push(`capability ${id} has invalid owner ${row.owner}`);
    }
    if (!isNonEmptyString(row.acceptance) || row.acceptance.trim().length < 24) {
      violations.push(`capability ${id} is missing acceptance criteria`);
    }

    validateTargetEvidence(row, manifest.reference, violations);
    validateProxyEvidence(row, violations);

    if (row.scope === "community" && rank(row.target_evidence_grade) < rank("source-backed")) {
      violations.push(`capability ${id} target evidence grade ${row.target_evidence_grade} is below source-backed`);
    }
    if (row.scope === "community" && !["Present", "Partial", "Missing"].includes(row.proxybot_status)) {
      violations.push(`capability ${id} community scope has invalid status ${row.proxybot_status}`);
    }
    if (row.scope === "private" && row.proxybot_status !== "Out-of-scope private") {
      violations.push(`capability ${id} private scope requires Out-of-scope private status`);
    }
    if (row.scope === "future" && row.proxybot_status !== "Future-not-shipped") {
      violations.push(`capability ${id} future scope requires Future-not-shipped status`);
    }

    const proxyEvidence = Array.isArray(row.proxybot_evidence) ? row.proxybot_evidence : [];
    const hasExistingSourceOrTest = proxyEvidence.some(
      (item) => /^(source|test):/.test(item) && pathExistsBelowRepository(item),
    );
    if (row.proxybot_status === "Present") {
      if (!["test-backed", "release-proven"].includes(row.proxybot_evidence_grade) || !proxyEvidence.some((item) => /^test:/.test(item) && pathExistsBelowRepository(item))) {
        violations.push(`capability ${id} Present claim requires test-backed or release-proven ProxyBot evidence`);
      }
    }
    if (row.proxybot_status === "Partial") {
      if (rank(row.proxybot_evidence_grade) < rank("source-backed") || !hasExistingSourceOrTest) {
        violations.push(`capability ${id} Partial claim requires source-backed evidence and an existing source or test item`);
      }
    }
  }

  for (const category of REQUIRED_CATEGORIES) {
    if (!categories.has(category)) violations.push(`missing required inventory category: ${category}`);
  }
  return violations;
}

function main() {
  const markdownPath = process.argv[2] || "docs/parity/rockxy-community-matrix.md";
  let manifest;
  try {
    manifest = extractManifest(readFileSync(markdownPath, "utf8"));
  } catch (error) {
    console.error(errorMessage(error.message));
    process.exitCode = 1;
    return;
  }
  const violations = validateManifest(manifest);
  if (violations.length > 0) {
    for (const violation of violations) console.error(errorMessage(violation));
    process.exitCode = 1;
    return;
  }
  console.log(
    `parity matrix: ${manifest.capabilities.length} capabilities validated against ${manifest.reference.repository}@${manifest.reference.commit}`,
  );
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url))) {
  main();
}
