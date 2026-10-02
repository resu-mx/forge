# Extension Store Publishing Design

**Status**: Approved
**Date**: 2026-04-21
**Predecessor**: M7 Infrastructure (v1.0.0, MVP complete)

## Overview

Publish the Forge Job Tools browser extension to Chrome Web Store and Firefox Add-ons. Three workstreams: repo split (public `aRustyDev/forge`), store listing prep (metadata, privacy policy, permissions), and CI/CD pipeline (automated build + publish on tag push).

## Workstream 1: Repo Split

### Goal

Create a clean public repo `aRustyDev/forge` from the private `aRustyDev/job-hunting` monorepo, stripping personal data from git history while preserving commit history for application code.

### Approach

Use `git filter-repo` to rewrite history, keeping only application-relevant paths.

**Paths to keep:**
```
packages/          — core, sdk, mcp, webui, extension
justfile           — build/dev tooling
package.json       — workspace root
bun.lockb          — lockfile
tsconfig.json      — TypeScript config
.gitignore         — ignore rules
CLAUDE.md          — project instructions (review for personal data before including)
.claude/rules/     — extension cross-browser rules, UI component rules
.claude/plans/forge-resume-browser-extension/  — extension specs + plans
.claude/plans/forge-resume-builder/            — Forge app specs + plans (review for personal data)
```

**Paths stripped (personal data):**
```
data/              — forge.db (personal resume data)
.beads/            — personal issue tracker
.claude/plans/claude-code-telemetry/  — personal tooling
.claude/plans/forge-prod-infra/       — personal infra
.claude/plans/job-hunting-pipeline/   — personal pipeline
.claude/plans/ast-grep/               — personal tooling
*.md files with personal career data
```

**Post-filter cleanup:**
- Review CLAUDE.md — strip personal references, keep project structure docs
- Add a proper README.md with: what Forge is, how to install, how to run, screenshot
- Add LICENSE (MIT or Apache-2.0 — user to decide)
- Update `.gitignore` for the clean repo (add `data/` to prevent accidental DB commits)
- Ensure `data/` directory is created at runtime with a `.gitkeep` or init script

### Execution Steps

```bash
# 1. Create the new repo on GitHub (empty, public)
gh repo create aRustyDev/forge --public --description "Self-hosted resume builder and job application toolkit"

# 2. Clone job-hunting as a fresh copy for filtering
git clone aRustyDev/job-hunting forge-filter-tmp
cd forge-filter-tmp

# 3. Run git filter-repo (preserves commits touching these paths)
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

# 4. Review for any remaining personal data
git log --all --diff-filter=A --name-only --pretty=format: | sort -u | head -100
# Manually inspect any suspicious file paths

# 5. Set new remote and push
git remote add origin git@github.com:aRustyDev/forge.git
git push -u origin main

# 6. Clean up
cd .. && rm -rf forge-filter-tmp
```

### Running Forge with External DB

After split, Forge locates the database via `FORGE_DB_PATH` environment variable:

- Default: `./data/forge.db` (relative to repo root, for fresh installs)
- Override: `FORGE_DB_PATH=~/notes/job-hunting/data/forge.db just dev`

This requires a small code change in `packages/core/src/db/connection.ts` to respect the env var.

### job-hunting Repo

Stays private. Contains personal data (`data/forge.db`, `.beads/`, notes). Will be archived as Forge matures. No code changes needed — it just stops being the active development repo.

## Workstream 2: Store Listing Prep

### Manifest Updates

Both `manifest.json` and `manifest.firefox.json`:

```json
{
  "name": "Forge Job Tools",
  "description": "Capture job listings and auto-fill applications with your self-hosted Forge server.",
  "homepage_url": "https://github.com/aRustyDev/forge"
}
```

Description must be under 132 characters (currently 82 — fits).

### Store Listing Metadata

**Chrome Web Store:**

| Field | Value |
|-------|-------|
| Name | Forge Job Tools |
| Summary | Capture job listings and auto-fill applications with your self-hosted Forge server. |
| Category | Productivity |
| Language | English |
| Privacy policy URL | https://arusty.dev/privacy |
| Homepage URL | https://github.com/aRustyDev/forge |
| Single purpose | Capture job descriptions from job boards and auto-fill job application forms using data from a self-hosted Forge server. |

**Firefox Add-ons:**

| Field | Value |
|-------|-------|
| Name | Forge Job Tools |
| Summary | Capture job listings and auto-fill applications with your self-hosted Forge server. |
| Category | Other (closest fit — no "Productivity" on AMO, alternatives: "Search Tools" or "Other") |
| Privacy policy URL | https://arusty.dev/privacy |
| Homepage URL | https://github.com/aRustyDev/forge |
| Support URL | https://github.com/aRustyDev/forge/issues |

**Full description (shared, both stores):**

```
Forge Job Tools captures job descriptions from LinkedIn and Workday, 
extracts structured data (title, company, salary, location, requirements), 
and auto-fills job application forms — all powered by your self-hosted 
Forge server.

Features:
- One-click capture from LinkedIn and Workday job pages
- Right-click context menu for quick capture
- Structured JD parsing with salary, location, and requirements extraction
- Auto-fill Workday application forms from your Forge profile
- Answer bank for EEO and work authorization questions
- Page overlay for reviewing extracted data before saving
- Server-side config and error logging

Requirements:
- A running Forge server (self-hosted, localhost by default)
- See https://github.com/aRustyDev/forge for setup instructions

Privacy:
- All data stays on your machine — sent only to your self-hosted server
- No analytics, no telemetry, no third-party data collection
- See https://arusty.dev/privacy for full privacy policy
```

### Permissions Justification

Both stores require per-permission justification during submission:

| Permission | Justification |
|------------|--------------|
| `storage` | Caches extension configuration locally for offline fallback when the user's Forge server is unreachable. |
| `contextMenus` | Adds a "Capture to Forge" right-click menu item on supported job board pages (LinkedIn, Workday). |
| `activeTab` | Reads job listing content from the current tab when the user explicitly triggers a capture action (click or context menu). |
| `scripting` | Injects content scripts to extract job description data and auto-fill application form fields on supported sites. |
| `*://*.linkedin.com/*` | Extracts job description data (title, company, salary, location, requirements) from LinkedIn job listing pages. |
| `*://*.myworkdayjobs.com/*` | Extracts job descriptions and auto-fills application forms on Workday-hosted career sites. |
| `*://*.myworkday.com/*` | Same as above — Workday uses both myworkdayjobs.com and myworkday.com domains. |

### Icons

Placeholder icons for initial submission (128x128, 48x48, 16x16 PNG). To be replaced with designed icon before public listing goes live. A simple colored square with "FJT" or anvil symbol as placeholder.

### Privacy Policy

Add `/privacy` page to `aRustyDev/landing-zone` (arusty.dev, hosted on Cloudflare).

Universal privacy policy with per-product sections:

```markdown
# Privacy Policy

Last updated: 2026-04-21

## General

All software published by Adam Russell ("aRustyDev") is designed to be 
local-first. Your data stays on your devices unless you explicitly 
configure a connection to a remote server you control.

I do not collect, store, or transmit any personal data. I do not use 
analytics, telemetry, or tracking of any kind.

## Forge Job Tools (Browser Extension)

### What data is accessed
- Job listing page content (title, company, salary, location, description) 
  on LinkedIn and Workday when you explicitly trigger a capture action.
- Form field structure on Workday application pages for auto-fill.

### Where data goes
- All captured data is sent exclusively to your self-hosted Forge server 
  (localhost:3000 by default, configurable).
- No data is sent to any third-party server, cloud service, or remote endpoint.

### What is stored locally
- Extension configuration is cached in browser local storage as a fallback 
  when your Forge server is unreachable.
- No browsing history, cookies, or personal information is stored by the extension.

### What is NOT collected
- No analytics or usage tracking
- No personal information beyond what you explicitly capture
- No data shared with third parties
- No cookies or cross-site tracking

## Contact

For questions about this privacy policy: adam@arusty.dev
```

### Screenshots

Minimum 1 screenshot per store. Capture:
1. LinkedIn job page with capture overlay showing extracted data
2. Workday form with auto-filled fields
3. WebUI config editor (shows it's a real tool with a backend)

Screenshots can be captured manually before submission. Dimensions: 1280x800 (Chrome) or similar (Firefox is flexible).

## Workstream 3: CI/CD Pipeline

### Local Packaging Command

Add to `justfile`:

```just
# Package extension for store submission
pack-extension:
    cd packages/extension && BROWSER=chrome bun run build
    cd packages/extension && BROWSER=firefox bun run build
    cd packages/extension/dist/chrome && zip -r ../../../forge-job-tools-chrome-v$(jq -r .version ../../manifest.json).zip .
    cd packages/extension/dist/firefox && zip -r ../../../forge-job-tools-firefox-v$(jq -r .version ../../manifest.firefox.json).zip .
    @echo "Packaged:"
    @ls -la forge-job-tools-*.zip

# Package Firefox source (required for AMO review of bundled code)
pack-extension-source:
    git archive HEAD --prefix=forge-source/ -o forge-job-tools-source.zip
    @echo "Source archive: forge-job-tools-source.zip"
```

### GitHub Actions Workflow

File: `.github/workflows/extension-publish.yml`

**Trigger:** Push of version tag (`v*`) or manual `workflow_dispatch`.

**Jobs:**

1. **build** — Install deps, build both browsers, run extension tests
2. **package** — Zip chrome + firefox + source archive
3. **release** — Upload zips to GitHub Releases (tagged)
4. **publish-chrome** — Upload to Chrome Web Store via API
5. **publish-firefox** — Upload to Firefox AMO via `web-ext sign`

**GitHub Secrets needed:**
- `CHROME_EXTENSION_ID` — assigned after first manual upload
- `CHROME_CLIENT_ID` — from Google Cloud Console OAuth
- `CHROME_CLIENT_SECRET` — from Google Cloud Console OAuth
- `CHROME_REFRESH_TOKEN` — from OAuth flow
- `AMO_API_KEY` — from Firefox Add-on Developer Hub
- `AMO_API_SECRET` — from Firefox Add-on Developer Hub

**Workflow phases:**
- Phase 1 (initial): Build + package + GitHub Release only (manual store upload for first submission)
- Phase 2 (after approval): Add automated store publishing jobs

This phased approach means the first submission is manual (required — stores need human review of a new extension), then subsequent updates are automated.

### Version Workflow

```
1. Bump version in both manifests
2. Commit: "chore(ext): bump version to X.Y.Z"
3. Tag: git tag vX.Y.Z
4. Push: git push && git push --tags
5. CI builds, packages, creates GitHub Release
6. (Phase 2) CI publishes to both stores
```

## Execution Order

1. **Repo split** — Create `aRustyDev/forge`, filter history, push (done from `aRustyDev/job-hunting`)
2. **Switch working directory** — All subsequent work happens in a fresh SSH clone of `aRustyDev/forge` (`git clone git@github.com:aRustyDev/forge.git`)
3. **Privacy policy** — Add `/privacy` to landing-zone (separate repo: `aRustyDev/landing-zone`)
4. **Manifest updates** — Rename to "Forge Job Tools", add homepage_url (in `aRustyDev/forge`)
5. **CI/CD Phase 1** — GitHub Actions workflow for build + package + release (in `aRustyDev/forge`)
6. **Store accounts** — Set up Chrome + Firefox developer accounts
7. **First submission** — Manual upload to both stores with metadata + screenshots
8. **CI/CD Phase 2** — Add automated store publishing after approval (in `aRustyDev/forge`)

**Important:** After step 2, all code changes (manifests, CI/CD, packaging) are committed to `aRustyDev/forge`, NOT `aRustyDev/job-hunting`. The `job-hunting` repo is only touched during the filter-repo step and is otherwise left as-is.

## Not In Scope

- Icon/branding design (separate task, placeholder for initial submission)
- New job board plugins (ship with LinkedIn + Workday only)
- Optional permissions UX (future, when adding new job boards)
- Hosted/SaaS Forge backend
