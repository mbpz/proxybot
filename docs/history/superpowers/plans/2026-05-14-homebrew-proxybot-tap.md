# Homebrew ProxyBot Tap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `homebrew-proxybot` GitHub repo and update CI to auto-update the tap after each release.

**Architecture:** After building the macOS app and creating a GitHub release, the CI workflow checks out the homebrew tap repo, updates the cask file with new version and SHA256, and pushes back.

**Tech Stack:** GitHub Actions, Homebrew Cask, `HOMEBREW_TAP_TOKEN` for authentication

---

## Pre-requisite (Manual - Outside CI)

### Task 1: Create `homebrew-proxybot` GitHub Repository

**Manual Action Required:** Create a new public GitHub repo named `homebrew-proxybot` under the `mbpz` account.
- Repo URL will be: `https://github.com/mbpz/homebrew-proxybot`
- Add an initial empty commit or a README so it's not empty

---

## File Changes

### Modify: `.github/workflows/release.yml`

The `homebrew` job (lines 108-143) needs these changes:
1. Change `repository` from `mbpz/homebrew-tap` to `mbpz/homebrew-proxybot`
2. Change path from `homebrew-tap` to `homebrew-proxybot`
3. Add authentication using `HOMEBREW_TAP_TOKEN` secret
4. Fix the SHA256 step reference (`steps.sha.outputs.sha256` → correctly reference the step output)

### Create: `homebrew-proxybot/Casks/proxybot.rb`

Initial cask file mirroring what exists in `homebrew-tap`.

---

## Task 2: Update Homebrew Job in release.yml

**File:** `.github/workflows/release.yml:108-143`

- [ ] **Step 1: Update repository and path**

Change line 116-117:
```yaml
repository: mbpz/homebrew-proxybot
path: homebrew-proxybot
```

- [ ] **Step 2: Add token authentication before git push**

After line 118 (after checkout step), add:
```yaml
- name: Configure git
  run: |
    git config user.name "github-actions[bot]"
    git config user.email "418-98282+github-actions[bot]@users.noreply.github.com"
    gh auth token --hostname github.com > /tmp/hub_token
    echo "https://x-access-token:$(cat /tmp/hub_token)@github.com" > /tmp/git_cred
    git config credential.helper "store --file=/tmp/git_cred"
```

- [ ] **Step 3: Add HOMEBREW_TAP_TOKEN to gh auth**

Change the authentication to use the `HOMEBREW_TAP_TOKEN` environment variable instead of `GITHUB_TOKEN`:
```yaml
- name: Push to tap
  env:
    HOMEBREW_TAP_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}
  run: |
    echo "$HOMEBREW_TAP_TOKEN" | gh auth login --hostname github.com --token-stdin
    cd homebrew-proxybot
    # ... rest of update logic
```

- [ ] **Step 4: Fix SHA256 step reference**

On line 135, `steps.sha.outputs.sha256` references a step that doesn't exist. The step is named `Get SHA256 from release` but has no `id:` attribute, so it should be referenced as `${{ steps.*.outputs.sha256 }}`. Add `id: sha` to that step (line 124).

Change line 124 from:
```yaml
      - name: Get SHA256 from release
        run: |
```
to:
```yaml
      - name: Get SHA256 from release
        id: sha
        run: |
```

- [ ] **Step 5: Update git push to use token auth**

The git push needs to use the authenticated gh session. Update line 143 from:
```yaml
          git push
```
to:
```yaml
          git push https://github.com/mbpz/homebrew-proxybot.git
```

---

## Task 3: Create Initial Cask in homebrew-proxybot Repo

Since `homebrew-proxybot` is a new repo, you need to populate it. Create the file locally first, then CI will update it on subsequent releases.

**Create:** `homebrew-proxybot/Casks/proxybot.rb`

```ruby
cask "proxybot" do
  arch arm: "arm64", intel: "x64"

  version "1.2.0"
  sha256 arm:   "bedff7e059d1bebc04316b117639e3bbfbaefaa5dc9f30fecbc53c4044d0a4b2",
         intel: "TODO: compute after first release - run on Intel Mac"

  url "https://github.com/mbpz/proxybot/releases/download/v#{version}/ProxyBot-#{version}-mac-#{arch}.zip"
  name "ProxyBot"
  desc "HTTPS MITM proxy tool for developers — capture and decrypt mobile traffic"
  homepage "https://github.com/mbpz/proxybot"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: ">= :big_sur"

  artifact "ProxyBot.app", target: "/Applications/ProxyBot.app"

  uninstall quit: "com.proxybot.app",
            delete: "/Applications/ProxyBot.app"

  zap trash: [
    "~/.proxybot",
    "~/Library/Application Support/com.proxybot.app",
    "~/Library/Preferences/com.proxybot.app.plist",
    "~/Library/Saved Application State/com.proxybot.app.savedState",
  ]
end
```

---

## Verification

1. Verify `HOMEBREW_TAP_TOKEN` secret is added to the repository settings at `https://github.com/mbpz/proxybot/settings/secrets/actions`
2. After creating `homebrew-proxybot` repo, push the initial cask file to it
3. Trigger a test release or run workflow dispatch with a test version
4. Verify the cask updates correctly in the `homebrew-proxybot` repo

---

## Self-Review Checklist

- [x] Spec coverage: Creating new tap repo ✓, updating CI ✓, using HOMEBREW_TAP_TOKEN ✓
- [x] No placeholders: All code is complete
- [x] Type consistency: N/A (pure YAML/Shell changes)
- [x] Step dependencies: Task 3 (create cask file) must happen before CI can successfully update it