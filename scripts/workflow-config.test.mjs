import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const workflows = ["ci.yml", "release.yml"].map((name) =>
  readFileSync(new URL(`../.github/workflows/${name}`, import.meta.url), "utf8"),
);

test("package.json is the only pnpm version authority", () => {
  assert.match(pkg.packageManager, /^pnpm@10\.33\.0\+/);
  for (const workflow of workflows) {
    assert.doesNotMatch(
      workflow,
      /pnpm\/action-setup@v4[\s\S]{0,120}\n\s+with:\n\s+version:/,
    );
  }
});
