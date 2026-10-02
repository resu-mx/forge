# Extension Store Publishing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish the Forge Job Tools browser extension to Chrome Web Store and Firefox Add-ons, with automated CI/CD for future releases.

**Architecture:** Three workstreams across three repos: repo split (`job-hunting` → `forge`), privacy policy (`landing-zone`), and store prep + CI/CD (`forge`). After Task 1, all code work happens in the new `aRustyDev/forge` repo.

**Tech Stack:** git filter-repo, GitHub Actions, Chrome Web Store API, web-ext (Mozilla), just

**Spec:** `.claude/plans/forge-resume-browser-extension/refs/specs/2026-04-21-store-publishing-design.md`

---

## File Map

### In `aRustyDev/forge` (after repo split)

| File | Purpose |
|------|---------|
| `packages/extension/manifest.json` | Rename to "Forge Job Tools", add homepage_url |
| `packages/extension/manifest.firefox.json` | Same updates |
| `justfile` | Add `pack-extension` and `pack-extension-source` recipes |
| `.github/workflows/extension-publish.yml` | CI/CD: build, package, release, publish |
| `README.md` | Public-facing project README |
| `.gitignore` | Ensure `data/` excluded, add `*.zip` |

### In `aRustyDev/landing-zone`

| File | Purpose |
|------|---------|
| Privacy policy page (framework-dependent path) | Universal privacy policy with Forge Job Tools section |

---

## Task 1: Repo Split — Create aRustyDev/forge

**Prerequisite:** `git-filter-repo` installed (`brew install git-filter-repo` or `pip install git-filter-repo`).

This task runs from the user's machine, NOT inside a worktree.

- [ ] **Step 1: Install git-filter-repo if needed**

Run:
```bash
git filter-repo --version 2>/dev/null || brew install git-filter-repo
```

- [ ] **Step 2: Create the empty public repo on GitHub**

Run:
```bash
gh repo create aRustyDev/forge --public --description "Self-hosted resume builder and job application toolkit" --confirm
```

- [ ] **Step 3: Clone job-hunting into a temp directory for filtering**

Run:
```bash
cd /tmp
git clone git@github.com:aRustyDev/job-hunting.git forge-filter-tmp
cd forge-filter-tmp
```

- [ ] **Step 4: Run git filter-repo to keep only application code**

Run:
```bash
git filter-repo \
  --path packages/ \
  --path justfile \
  --path package.json \
  --path bun.lockb \
  --path tsconfig.json \
  --path .gitignore \
  --path CLAUDE.md \
  --path .claude/rules/ \
  --path .claude/plans/forge-resume-browser-extension/ \
  --path .claude/plans/forge-resume-builder/
```

Expected: Rewrites history. Only commits touching those paths remain.

- [ ] **Step 5: Audit for personal data leaks**

Run:
```bash
# List all files that ever existed in the filtered history
git log --all --diff-filter=A --name-only --pretty=format: | sort -u | grep -v '^$' | head -100
```

Scan the output for anything personal (salary data, org assessments, personal notes). If found, re-run filter-repo with additional `--path-regex` excludes.

Also check CLAUDE.md:
```bash
cat CLAUDE.md
```

If CLAUDE.md contains personal references (specific resume names, personal goals, etc.), edit it to keep only project structure documentation. Commit the cleanup.

- [ ] **Step 6: Review .claude/plans/ for personal data**

Run:
```bash
find .claude/plans/ -name "*.md" | head -30
```

Check spec files for personal career data (company names in examples are fine — actual salary figures, personal assessments, etc. are not). Remove or redact any files with personal data.

- [ ] **Step 7: Update .gitignore for the clean repo**

Ensure these lines exist in `.gitignore`:
```
data/
*.zip
.beads/
.env
```

- [ ] **Step 8: Add README.md**

Create `README.md`:

```markdown
# Forge

Self-hosted resume builder and job application toolkit.

## What is Forge?

Forge helps you manage your career data — experience, skills, bullets, perspectives — and build targeted resumes. It includes:

- **Core API** — HTTP server (Bun + Hono + SQLite)
- **SDK** — TypeScript client library
- **MCP Server** — 67 tools for Claude Code integration
- **Web UI** — Svelte frontend for managing everything
- **Browser Extension** — Capture job listings and auto-fill applications (Chrome + Firefox)

## Quick Start

```bash
# Install dependencies
bun install

# Start all services (API :3000, MCP :5174, WebUI :5173)
just dev
```

## Browser Extension

The Forge Job Tools extension captures job descriptions from LinkedIn and Workday, and auto-fills application forms using your Forge profile data.

Install from:
- [Chrome Web Store](#) (coming soon)
- [Firefox Add-ons](#) (coming soon)

Or load from source: `packages/extension/dist/chrome/` or `packages/extension/dist/firefox/`

## License

MIT
```

- [ ] **Step 9: Add LICENSE file**

Create `LICENSE` with MIT license text (replace year/name):

```
MIT License

Copyright (c) 2026 Adam Russell

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 10: Commit cleanup changes**

```bash
git add -A
git commit -m "chore: clean up for public repo — README, LICENSE, gitignore"
```

- [ ] **Step 11: Push to the new remote**

```bash
git remote add forge git@github.com:aRustyDev/forge.git
git push -u forge main
```

- [ ] **Step 12: Clean up temp directory**

```bash
cd /tmp && rm -rf forge-filter-tmp
```

- [ ] **Step 13: Verify on GitHub**

Run:
```bash
gh repo view aRustyDev/forge --web
```

Verify: repo is public, has commit history, no personal data visible.

---

## Task 2: Clone forge repo and verify

**All subsequent tasks work from this clone.**

- [ ] **Step 1: Clone the new repo via SSH**

Run:
```bash
cd ~/code  # or wherever you keep repos
git clone git@github.com:aRustyDev/forge.git
cd forge
```

- [ ] **Step 2: Install dependencies**

Run:
```bash
bun install
```

- [ ] **Step 3: Create data directory and verify the app starts**

Run:
```bash
mkdir -p data
just api &
sleep 2
curl -s http://localhost:3000/api/health | jq .
kill %1
```

Expected: `{"data":{"server":"ok","version":"..."}}`

If you want to use your existing data:
```bash
FORGE_DB_PATH=~/notes/job-hunting/data/forge.db just api
```

- [ ] **Step 4: Run tests**

Run:
```bash
bun test --timeout 30000 2>&1 | tail -10
```

Expected: Tests pass (same baseline as job-hunting repo).

---

## Task 3: Privacy Policy — Add /privacy to arusty.dev

**Repo:** `aRustyDev/landing-zone`

- [ ] **Step 1: Clone the landing-zone repo**

Run:
```bash
cd /tmp
git clone git@github.com:aRustyDev/landing-zone.git
cd landing-zone
```

- [ ] **Step 2: Explore the site structure**

Run:
```bash
ls -la
cat package.json 2>/dev/null || cat Cargo.toml 2>/dev/null || ls *.html 2>/dev/null
```

Determine the framework (Astro, Hugo, plain HTML, etc.) and how pages are added.

- [ ] **Step 3: Add the privacy policy page**

Create the privacy policy page in whatever format the site uses. The content:

```markdown
# Privacy Policy

Last updated: 2026-04-21

## General

All software published by Adam Russell ("aRustyDev") is designed to be local-first.
Your data stays on your devices unless you explicitly configure a connection to a
remote server you control.

I do not collect, store, or transmit any personal data. I do not use analytics,
telemetry, or tracking of any kind.

## Forge Job Tools (Browser Extension)

### What data is accessed

- Job listing page content (title, company, salary, location, description) on
  LinkedIn and Workday when you explicitly trigger a capture action.
- Form field structure on Workday application pages for auto-fill.

### Where data goes

- All captured data is sent exclusively to your self-hosted Forge server
  (localhost:3000 by default, configurable).
- No data is sent to any third-party server, cloud service, or remote endpoint.

### What is stored locally

- Extension configuration is cached in browser local storage as a fallback when
  your Forge server is unreachable.
- No browsing history, cookies, or personal information is stored by the extension.

### What is NOT collected

- No analytics or usage tracking
- No personal information beyond what you explicitly capture
- No data shared with third parties
- No cookies or cross-site tracking

## Contact

For questions about this privacy policy: adam@arusty.dev
```

- [ ] **Step 4: Test locally if the site has a dev server**

Run the site's dev command (e.g., `bun dev`, `npm run dev`, `hugo server`) and verify `/privacy` renders correctly.

- [ ] **Step 5: Commit and push**

```bash
git add -A
git commit -m "feat: add privacy policy page"
git push
```

- [ ] **Step 6: Verify deployment**

Wait for Cloudflare to deploy (usually < 2 minutes), then:
```bash
curl -s -o /dev/null -w "%{http_code}" https://arusty.dev/privacy
```

Expected: `200`

---

## Task 4: Manifest Updates — Rename to Forge Job Tools

**Repo:** `aRustyDev/forge` (cloned in Task 2)

**Files:**
- Modify: `packages/extension/manifest.json`
- Modify: `packages/extension/manifest.firefox.json`

- [ ] **Step 1: Update Chrome manifest**

In `packages/extension/manifest.json`, update these fields:

```json
{
  "name": "Forge Job Tools",
  "description": "Capture job listings and auto-fill applications with your self-hosted Forge server.",
  "homepage_url": "https://github.com/aRustyDev/forge"
}
```

Leave all other fields unchanged.

- [ ] **Step 2: Update Firefox manifest**

In `packages/extension/manifest.firefox.json`, update the same three fields:

```json
{
  "name": "Forge Job Tools",
  "description": "Capture job listings and auto-fill applications with your self-hosted Forge server.",
  "homepage_url": "https://github.com/aRustyDev/forge"
}
```

- [ ] **Step 3: Rebuild extension**

Run:
```bash
cd packages/extension && bun run build
```

Expected: Both `dist/chrome/` and `dist/firefox/` rebuild with the new name.

- [ ] **Step 4: Verify manifest name in built output**

Run:
```bash
jq .name packages/extension/dist/chrome/manifest.json
jq .name packages/extension/dist/firefox/manifest.json
```

Expected: Both output `"Forge Job Tools"`

- [ ] **Step 5: Run extension tests**

Run:
```bash
cd packages/extension && bun test
```

Expected: All tests pass. If any test asserts on the extension name, update it.

- [ ] **Step 6: Commit**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json packages/extension/dist/
git commit -m "chore(ext): rename to 'Forge Job Tools' + add homepage_url"
```

---

## Task 5: Packaging Commands — just pack-extension

**Repo:** `aRustyDev/forge`

**Files:**
- Modify: `justfile`

- [ ] **Step 1: Add pack-extension recipe**

Append to the `justfile` (before any trailing comments or after the last recipe):

```just
# ─── Extension packaging ────────────────────────────────

# Package extension for store submission (both browsers)
pack-extension:
    cd packages/extension && bun run build
    @mkdir -p dist
    cd packages/extension/dist/chrome && zip -r ../../../../dist/forge-job-tools-chrome-v$(cd ../.. && jq -r .version manifest.json).zip .
    cd packages/extension/dist/firefox && zip -r ../../../../dist/forge-job-tools-firefox-v$(cd ../.. && jq -r .version manifest.firefox.json).zip .
    @echo "Packaged:"
    @ls -la dist/forge-job-tools-*.zip

# Package source for Firefox AMO review (required for bundled code)
pack-extension-source:
    git archive HEAD --prefix=forge-source/ -o dist/forge-job-tools-source.zip
    @echo "Source archive: dist/forge-job-tools-source.zip"
```

- [ ] **Step 2: Add dist/ to .gitignore (for zip artifacts, not extension dist)**

Ensure the root `.gitignore` has:

```
# Packaging artifacts
/dist/
```

Note: `packages/extension/dist/` is tracked (it's the built extension). The root `/dist/` is for zip artifacts only.

- [ ] **Step 3: Test the packaging command**

Run:
```bash
just pack-extension
```

Expected: Two zip files created in `dist/`:
```
dist/forge-job-tools-chrome-v1.0.0.zip
dist/forge-job-tools-firefox-v1.0.0.zip
```

- [ ] **Step 4: Verify zip contents**

Run:
```bash
unzip -l dist/forge-job-tools-chrome-v1.0.0.zip | head -10
unzip -l dist/forge-job-tools-firefox-v1.0.0.zip | head -10
```

Expected: Contains `manifest.json`, `background.js`, popup files — no `node_modules`, no source maps.

- [ ] **Step 5: Test source packaging**

Run:
```bash
just pack-extension-source
```

Expected: `dist/forge-job-tools-source.zip` created.

- [ ] **Step 6: Commit**

```bash
git add justfile .gitignore
git commit -m "feat: add pack-extension and pack-extension-source just recipes"
```

---

## Task 6: GitHub Actions — Build + Package + Release

**Repo:** `aRustyDev/forge`

**Files:**
- Create: `.github/workflows/extension-publish.yml`

- [ ] **Step 1: Create the workflow file**

Create `.github/workflows/extension-publish.yml`:

```yaml
name: Extension Publish

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
    inputs:
      skip_store_publish:
        description: 'Skip store publishing (build + release only)'
        type: boolean
        default: true

permissions:
  contents: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: oven-sh/setup-bun@v2
        with:
          bun-version: latest

      - name: Install dependencies
        run: bun install

      - name: Run extension tests
        run: cd packages/extension && bun test

      - name: Build both browsers
        run: cd packages/extension && bun run build

      - name: Package Chrome zip
        run: |
          VERSION=$(jq -r .version packages/extension/manifest.json)
          cd packages/extension/dist/chrome
          zip -r ../../../../forge-job-tools-chrome-v${VERSION}.zip .

      - name: Package Firefox zip
        run: |
          VERSION=$(jq -r .version packages/extension/manifest.firefox.json)
          cd packages/extension/dist/firefox
          zip -r ../../../../forge-job-tools-firefox-v${VERSION}.zip .

      - name: Package source for AMO review
        run: git archive HEAD --prefix=forge-source/ -o forge-job-tools-source.zip

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: extension-packages
          path: |
            forge-job-tools-chrome-v*.zip
            forge-job-tools-firefox-v*.zip
            forge-job-tools-source.zip

  release:
    needs: build
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/v')
    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          name: extension-packages

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            forge-job-tools-chrome-v*.zip
            forge-job-tools-firefox-v*.zip
            forge-job-tools-source.zip
          generate_release_notes: true

  publish-chrome:
    needs: release
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/v') && !inputs.skip_store_publish
    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          name: extension-packages

      - name: Upload to Chrome Web Store
        uses: mnao305/chrome-extension-upload@v5.0.0
        with:
          file-path: forge-job-tools-chrome-v*.zip
          extension-id: ${{ secrets.CHROME_EXTENSION_ID }}
          client-id: ${{ secrets.CHROME_CLIENT_ID }}
          client-secret: ${{ secrets.CHROME_CLIENT_SECRET }}
          refresh-token: ${{ secrets.CHROME_REFRESH_TOKEN }}
          publish: true

  publish-firefox:
    needs: release
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/v') && !inputs.skip_store_publish
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Install web-ext
        run: npm install -g web-ext

      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          name: extension-packages

      - name: Sign and upload to AMO
        run: |
          # Unzip the Firefox build into a temp dir for web-ext
          mkdir -p /tmp/firefox-ext
          unzip forge-job-tools-firefox-v*.zip -d /tmp/firefox-ext
          web-ext sign \
            --source-dir /tmp/firefox-ext \
            --api-key ${{ secrets.AMO_API_KEY }} \
            --api-secret ${{ secrets.AMO_API_SECRET }} \
            --channel listed
```

- [ ] **Step 2: Verify workflow syntax**

Run:
```bash
# GitHub Actions syntax check (if act is installed)
act --list 2>/dev/null || echo "act not installed — push to verify"
```

Alternatively, just review the YAML for indentation issues.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/extension-publish.yml
git commit -m "feat: add GitHub Actions workflow for extension build + release + publish"
```

- [ ] **Step 4: Push to trigger workflow validation**

```bash
git push
```

The workflow won't run (no tag push), but GitHub will validate the YAML syntax. Check the Actions tab for any config errors.

---

## Task 7: Placeholder Icons

**Repo:** `aRustyDev/forge`

**Files:**
- Create: `packages/extension/icons/icon-16.png`
- Create: `packages/extension/icons/icon-48.png`
- Create: `packages/extension/icons/icon-128.png`
- Modify: `packages/extension/manifest.json`
- Modify: `packages/extension/manifest.firefox.json`

- [ ] **Step 1: Generate placeholder icons**

Create simple placeholder icons using ImageMagick (or any tool). These are temporary — to be replaced with designed icons before store submission.

```bash
cd packages/extension
mkdir -p icons

# 128x128 — primary store icon
convert -size 128x128 xc:'#4a90d9' \
  -gravity center -fill white -font Helvetica-Bold -pointsize 48 \
  -annotate 0 'FJT' icons/icon-128.png

# 48x48 — toolbar
convert -size 48x48 xc:'#4a90d9' \
  -gravity center -fill white -font Helvetica-Bold -pointsize 18 \
  -annotate 0 'FJT' icons/icon-48.png

# 16x16 — favicon
convert -size 16x16 xc:'#4a90d9' icons/icon-16.png
```

If ImageMagick isn't available, create any 3 PNG files at the right dimensions — even solid-color squares work as placeholders.

- [ ] **Step 2: Add icon references to Chrome manifest**

In `packages/extension/manifest.json`, add the `icons` field (at the top level, alongside `name`):

```json
{
  "icons": {
    "16": "icons/icon-16.png",
    "48": "icons/icon-48.png",
    "128": "icons/icon-128.png"
  }
}
```

Also update the `action` field to include the default icon:

```json
{
  "action": {
    "default_icon": {
      "16": "icons/icon-16.png",
      "48": "icons/icon-48.png"
    }
  }
}
```

Preserve any existing `action` fields (like `default_popup`).

- [ ] **Step 3: Add icon references to Firefox manifest**

Same changes in `packages/extension/manifest.firefox.json`:

```json
{
  "icons": {
    "16": "icons/icon-16.png",
    "48": "icons/icon-48.png",
    "128": "icons/icon-128.png"
  }
}
```

And the `action.default_icon` field.

- [ ] **Step 4: Rebuild and test**

Run:
```bash
bun run build
bun test
```

Expected: Both browsers build, icons copied to dist, tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/icons/ packages/extension/manifest.json packages/extension/manifest.firefox.json packages/extension/dist/
git commit -m "chore(ext): add placeholder icons for store submission"
```

---

## Task 8: Store Account Setup + First Submission

This is a manual/interactive task — not automatable.

- [ ] **Step 1: Set up Firefox Add-on Developer account**

1. Go to https://addons.mozilla.org/developers/
2. Sign in with (or create) a Mozilla account
3. You're immediately a developer — no fee

- [ ] **Step 2: Set up Chrome Web Store Developer account**

1. Go to https://chrome.google.com/webstore/devconsole
2. Sign in with a Google account
3. Pay the one-time $5 registration fee
4. Verify email, agree to developer agreement

- [ ] **Step 3: Build the submission packages**

Run (from `aRustyDev/forge`):
```bash
just pack-extension
just pack-extension-source
```

- [ ] **Step 4: Submit to Firefox Add-ons**

1. Go to https://addons.mozilla.org/developers/addon/submit/distribution
2. Choose "On this site" (listed on AMO)
3. Upload `dist/forge-job-tools-firefox-v1.0.0.zip`
4. When prompted for source code (because code is minified), upload `dist/forge-job-tools-source.zip`
5. Fill in build instructions: "Requires Bun (https://bun.sh). Run: `bun install && cd packages/extension && BROWSER=firefox bun run vite build`"
6. Fill in metadata:
   - Name: Forge Job Tools
   - Summary: Capture job listings and auto-fill applications with your self-hosted Forge server.
   - Description: (use full description from spec)
   - Category: Other
   - Privacy policy: https://arusty.dev/privacy
   - Homepage: https://github.com/aRustyDev/forge
   - Support URL: https://github.com/aRustyDev/forge/issues
7. Upload icon (128x128)
8. Upload at least 1 screenshot
9. Submit for review

- [ ] **Step 5: Submit to Chrome Web Store**

1. Go to https://chrome.google.com/webstore/devconsole
2. Click "New Item"
3. Upload `dist/forge-job-tools-chrome-v1.0.0.zip`
4. Fill in metadata:
   - Name: Forge Job Tools
   - Summary: Capture job listings and auto-fill applications with your self-hosted Forge server.
   - Description: (use full description from spec)
   - Category: Productivity
   - Single purpose justification: "Capture job descriptions from job boards and auto-fill job application forms using data from a self-hosted Forge server."
   - Privacy policy: https://arusty.dev/privacy
   - Homepage: https://github.com/aRustyDev/forge
5. Fill in permissions justifications (one per permission — see spec for exact text)
6. Upload icons (16, 48, 128)
7. Upload at least 1 screenshot (1280x800)
8. Submit for review

- [ ] **Step 6: Generate API credentials for CI/CD Phase 2**

**Chrome:**
1. Go to Google Cloud Console → APIs & Services → Credentials
2. Create OAuth 2.0 Client ID (type: Web application)
3. Note the Client ID and Client Secret
4. Use the OAuth Playground or a script to get a Refresh Token
5. Note the Extension ID from the Chrome Developer Dashboard

**Firefox:**
1. Go to https://addons.mozilla.org/developers/addon/api/key/
2. Generate API credentials
3. Note the API Key (JWT issuer) and API Secret

- [ ] **Step 7: Store credentials as GitHub Secrets**

Run:
```bash
# Chrome secrets
gh secret set CHROME_EXTENSION_ID --repo aRustyDev/forge
gh secret set CHROME_CLIENT_ID --repo aRustyDev/forge
gh secret set CHROME_CLIENT_SECRET --repo aRustyDev/forge
gh secret set CHROME_REFRESH_TOKEN --repo aRustyDev/forge

# Firefox secrets
gh secret set AMO_API_KEY --repo aRustyDev/forge
gh secret set AMO_API_SECRET --repo aRustyDev/forge
```

Each command will prompt for the secret value interactively.

- [ ] **Step 8: Test CI/CD with a tag push**

```bash
cd ~/code/forge  # or wherever you cloned it
git tag v1.0.0
git push --tags
```

Check the Actions tab on GitHub. The `build` and `release` jobs should run. The `publish-chrome` and `publish-firefox` jobs will skip (they require `skip_store_publish` to be false, which defaults to true on manual dispatch, and the secrets need to be set).

---

## Summary

| Task | Repo | Description |
|------|------|-------------|
| 1 | job-hunting → forge | Repo split via git filter-repo |
| 2 | forge | Clone, verify, install |
| 3 | landing-zone | Privacy policy page |
| 4 | forge | Manifest rename to "Forge Job Tools" |
| 5 | forge | Packaging just recipes |
| 6 | forge | GitHub Actions CI/CD workflow |
| 7 | forge | Placeholder icons |
| 8 | (manual) | Store accounts, first submission, CI/CD secrets |
