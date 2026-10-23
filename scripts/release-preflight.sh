#!/usr/bin/env bash
# Preflight for a manual ProxyBot release.
#
# The release workflow is manual-only (`workflow_dispatch`); pushing a tag does
# not start it. This script checks the local tag, GitHub authentication, and the
# required signing/notarization secrets before dispatching the workflow.
#
# Usage:
#   scripts/release-preflight.sh            # checks only
#   scripts/release-preflight.sh --dispatch # checks, then dispatch the release
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DISPATCH=false
case "${1:-}" in
  --dispatch) DISPATCH=true ;;
  --help|-h)
    sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  "") ;;
  *)
    echo "unknown argument: $1 (use --dispatch or --help)" >&2
    exit 2
    ;;
esac

REQUIRED_SECRETS=(
  APPLE_CERTIFICATE
  APPLE_CERTIFICATE_PASSWORD
  APPLE_SIGNING_IDENTITY
  APPLE_ID
  APPLE_PASSWORD
  APPLE_TEAM_ID
  KEYCHAIN_PASSWORD
)

fail() {
  echo "release preflight: $*" >&2
  exit 1
}

VERSION="$(node -p "require('./package.json').version")"
TAG="v${VERSION}"

echo "product version : ${VERSION}"
echo "release tag     : ${TAG}"

BRANCH="$(git branch --show-current)"
if [ "$BRANCH" != "main" ]; then
  echo "warning: current branch is '${BRANCH}', not 'main'"
fi
if [ -n "$(git status --porcelain)" ]; then
  echo "warning: working tree has uncommitted changes"
fi

# The workflow checks out `ref: <version>`, so the tag must exist locally and
# on origin before dispatching.
git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null \
  || fail "tag ${TAG} does not exist locally; create it with: git tag -s ${TAG} -m \"ProxyBot ${VERSION}\""
git ls-remote --exit-code --tags origin "refs/tags/${TAG}" >/dev/null 2>&1 \
  || fail "tag ${TAG} is not on origin; push it with: git push origin ${TAG}"

command -v gh >/dev/null 2>&1 || fail "gh CLI is not installed"
gh auth status >/dev/null 2>&1 || fail "gh is not authenticated; run: gh auth login"

HAVE_SECRETS="$(gh secret list --json name --jq '.[].name' 2>/dev/null || true)"
MISSING=()
for secret in "${REQUIRED_SECRETS[@]}"; do
  if ! grep -qx "$secret" <<<"$HAVE_SECRETS"; then
    MISSING+=("$secret")
  fi
done

if [ "${#MISSING[@]}" -gt 0 ]; then
  echo
  echo "missing repository secrets:"
  printf '  - %s\n' "${MISSING[@]}"
  cat >&2 <<'EOF'

Signed releases require an Apple Developer Program membership. Provision the
secrets with `gh secret set <NAME>`; see docs/releasing.md for the exact export
and base64 steps. Nothing is published while any secret is missing.
EOF
  exit 1
fi

echo "all required release secrets are present"

if [ "$DISPATCH" = true ]; then
  echo "dispatching release workflow for ${TAG} ..."
  gh workflow run release.yml -f "version=${TAG}"
  echo "watch it with: gh run list --workflow=release.yml --limit 1"
else
  echo "checks passed; dispatch with: $0 --dispatch"
fi
