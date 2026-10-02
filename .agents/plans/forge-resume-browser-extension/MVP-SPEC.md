# Forge Browser Extension — MVP Spec

**Status**: Approved
**Date**: 2026-04-15
**Predecessor**: Prototype P1–P7 (v0.0.9, 58 tests passing)
**Target**: v0.1.1–v1.0.0

## Overview

Seven MVP phases building on the prototype series. Interleaved 2:1 capture-quality:autofill-completeness. Firefox/Zen port first (user's daily driver), then alternating capture UX and autofill improvements, culminating in infrastructure hardening.

## Architecture Decisions

### Versioning
MVP phases bump from 0.0.9 → 0.1.x (minor = MVP era, patch = phase). M1 = 0.1.1 through M7 = 1.0.0 (MVP complete).

### Parser Location
JD structural parser lives in `packages/core/src/parser/` — not extension-only. Both extension (3bp.21.5) and core ingestion (3bp.21.6) consume it. Extension bundles only the parser module via workspace dependency. Pure functions, no Forge service dependencies.

### Answer Bank Storage
Answer bank stores in Forge DB via `answer_bank` table — not `chrome.storage.local`. Consistent with "extension is a thin client" principle (SPEC §1). New API: `GET/PUT /api/profile/answers`.

### Cross-Browser from M1
After M1, build produces `dist/chrome/` and `dist/firefox/`. Manifest differences handled at build time. All subsequent phases developed and tested in both browsers.

### Overlay Injection
Page overlay (M6) is a content-script-injected shadow DOM panel. In-page, avoids popup-closing-on-blur, shadow DOM prevents host page CSS leakage.

## Phase Summary

| Phase | Version | Track | Beads | Summary |
|-------|---------|-------|-------|---------|
| M1 | 0.1.1 | Cross | 3bp.27 | Firefox/Zen port |
| M2 | 0.1.2 | Capture | 3bp.23, 3bp.24, 3bp.13, 3bp.14, 3bp.16 | Capture entry points + LinkedIn polish |
| M3 | 0.1.3 | Capture | 3bp.21.1, 3bp.21.2, 3bp.21.3 | Parser foundation (L1+L2+extractors) |
| M4 | 0.1.4 | Autofill | 3bp.25 | Workday dropdown filling |
| M5 | 0.1.5 | Capture | 3bp.21.5, 3bp.21.6, 3bp.15, 3bp.17, 3bp.18, 3bp.19 | Parser wiring + extraction fixes |
| M6 | 0.1.6 | Capture+Autofill | 3bp.30, 3bp.28, 3bp.26 | Page overlay + answer bank + app questions |
| M7 | 1.0.0 | Infra | 3bp.29, 3bp.31 | Config migration + logging |

## Phase Details

### M1 — Firefox/Zen Port (3bp.27)

Adds `webextension-polyfill` shim. Vite config produces two outputs.

**Key adaptations:**
- `chrome.scripting.executeScript` → polyfill equivalent
- `chrome.storage.local` → `browser.storage.local`
- Service worker → background script (Firefox MV2 persistent background page)
- `host_permissions` (MV3) → `permissions` array (MV2)
- `browser_specific_settings.gecko.id` in Firefox manifest

**Success criteria:**
- All 58 existing tests pass
- Extension loads in Zen with no console errors
- Capture a LinkedIn JD and autofill a Workday form — same as Chrome v0.0.9

### M2 — Capture Entry Points + LinkedIn Polish (3bp.23, 3bp.24, 3bp.13, 3bp.14, 3bp.16)

Two new capture triggers beyond the popup button:

**Context menu (3bp.23, subsumes 3bp.22):** Right-click on any job page → "Capture to Forge". Registered via `chrome.contextMenus.create` in background worker, filtered by `host_permissions`. Requires adding `contextMenus` permission to manifest.

**Injected button (3bp.24):** On LinkedIn job detail pages, inject a "Capture to Forge" button as sibling to existing Apply/Save buttons, anchored by `aria-label`. Programmatic injection only.

**LinkedIn extraction fixes (bundled):**
- 3bp.13: Salary chip — fix leading digit truncation in regex
- 3bp.14: Extract external Apply link URL (not just Easy Apply)
- 3bp.16: Capture org's LinkedIn URL during org creation

**Success criteria:**
- Context menu works on LinkedIn + Workday in both Chrome and Zen
- Injected button appears on LinkedIn job detail pages
- All 3 extraction bugs fixed with fixture tests

### M3 — Parser Foundation (3bp.21.1, 3bp.21.2, 3bp.21.3)

Standalone library in `packages/core/src/parser/`. Pure functions, no Forge dependencies.

**L1 — Heading splitter (3bp.21.1):** Split raw JD text by headings (markdown `##`, bold `**`, ALL CAPS, HTML `<h2>`–`<h4>`). Returns `Section[]` with raw text + heading text + byte offset.

**L2 — Taxonomy classifier (3bp.21.2):** Classify each section by keyword density. Taxonomy: `responsibilities | requirements | preferred | description | location | compensation | benefits | about_company | eeo`. Confidence score per section.

**Inline extractors (3bp.21.3):** Extract structured fields from classified sections:
- `salary_min`/`salary_max` from compensation sections
- Location list from location sections
- Work posture (remote/hybrid/on-site) from location/description sections

These power 3bp.15 (salary from body), 3bp.17 (work posture), 3bp.18 (multiple locations).

**Success criteria:**
- Parser tested against 20+ real JD fixtures covering diverse heading formats
- L2 achieves >85% classification accuracy on test corpus
- Extractors return structured salary/location/posture from body text

### M4 — Workday Dropdown Filling (3bp.25)

Extends P6/P7 autofill to non-text-input controls:

**Native `<select>` elements:** Country/state dropdowns with `data-automation-id`. Dispatch `change` events Workday's framework recognizes.

**Custom select widgets:** Workday's proprietary dropdown components (not native `<select>`). Click-to-open → search/filter → click-to-select interaction sequence.

**Radio buttons:** Gender, veteran status, disability — click + synthetic event dispatch.

Profile mapping extended: `address.country` → country dropdown, `address.state` → state dropdown.

**Success criteria:**
- Country, state, and custom dropdowns fill correctly on real Workday forms
- Radio buttons selectable programmatically
- Framework validation passes (no red borders after fill)

### M5 — Parser Wiring + Extraction Fixes (3bp.21.5, 3bp.21.6, 3bp.15, 3bp.17, 3bp.18, 3bp.19)

Wire the M3 parser into both consumers:

**Extension wiring (3bp.21.5):** Content script runs parser on extracted `raw_text` before sending to background. Structured sections available for overlay (M6).

**Core ingestion wiring (3bp.21.6):** `forge_ingest_job_description` MCP tool runs parser at ingest time. New columns on `job_descriptions`: `parsed_sections` (JSON), `salary_min`, `salary_max` auto-populated from parser. Enables MCP queries for specific sections.

**Parser-dependent extraction fixes:**
- 3bp.15: Salary from JD body text (parser's compensation extractor)
- 3bp.17: Work posture as separate field (parser's posture extractor)
- 3bp.18: Multiple/alternate locations (parser enumerates all location mentions)
- 3bp.19: JD location → Org Campus linkage (parsed location feeds campus resolution)

**Forge-side schema changes:**
- `job_descriptions` table: add `parsed_sections` JSON column (`salary_min`/`salary_max` columns already exist from migration 027 — parser auto-populates them)
- Migration to backfill existing JDs through parser

**Success criteria:**
- Extension capture includes parsed sections in payload
- MCP-ingested JDs get auto-populated salary/location
- Backfill migration parses existing JDs without data loss
- Org campus linked when parsed location matches existing campus

### M6 — Page Overlay + Answer Bank + App Questions (3bp.30, 3bp.28, 3bp.26)

Largest phase. Combines review UX with remaining autofill.

**Page overlay (3bp.30):** After extraction, inject shadow DOM overlay showing editable structured fields (title, company, location, salary, parsed sections from M5). User corrects before submitting. "Submit" replaces direct-to-API flow. "Cancel" dismisses without writing.

**Answer bank (3bp.28):** New `answer_bank` table in Forge DB. API: `GET/PUT /api/profile/answers`. Stores reusable answers keyed by `FieldKind`:
- `work_auth.us`, `work_auth.sponsorship`
- `eeo.gender`, `eeo.race`, `eeo.veteran`, `eeo.disability`

WebUI page to manage stored answers.

**Workday app questions (3bp.26):** Extend Workday plugin to detect Application Questions and Voluntary Disclosures pages. Map detected fields to answer bank entries via `FieldKind`. Fill from stored answers using M4's dropdown/radio filling.

**Forge-side changes:**
- `answer_bank` table: `id`, `field_kind` (unique), `value`, `created_at`, `updated_at`
- `GET /api/profile/answers` — list all
- `PUT /api/profile/answers` — upsert by field_kind
- WebUI: `/answers` page with form editor

**Success criteria:**
- Overlay renders on capture with editable fields from parser output
- Edited values persist to Forge on submit
- Answer bank CRUD via API + webUI
- Workday EEO/work-auth fields fill from answer bank

### M7 — Infrastructure (3bp.29, 3bp.31) → v1.0.0

**Config migration (3bp.29):** Extension fetches `GET /api/extension/config` on startup. Config stored in Forge DB, editable via webUI. Fallback to `chrome.storage.local`/`browser.storage.local` if API unreachable (offline graceful degradation).

**Server logging (3bp.31):** `POST /api/extension/log` accepts serialized `ExtensionError` objects. Stored in `extension_logs` table. Gated by config flag `enableServerLogging`. WebUI page to view recent extension errors.

**Forge-side changes:**
- `extension_config` table: key-value with defaults
- `extension_logs` table: `id`, `error_code`, `message`, `layer`, `plugin`, `url`, `context` (JSON), `timestamp`
- `GET /api/extension/config` — returns full config
- `PUT /api/extension/config` — update config keys
- `POST /api/extension/log` — append error log
- WebUI: `/extension/config` editor + `/extension/logs` viewer

Final version bump to **1.0.0** marks MVP complete.

**Success criteria:**
- Config loads from Forge API on startup
- Graceful fallback to local storage when API unreachable
- Logging endpoint receives and stores ExtensionError objects
- WebUI shows config editor and log viewer
- `devMode` defaults to `false` (toggle available in config)

## Dependency Graph

```
M1 (Firefox)
 │
 ├─► M2 (Entry points + LinkedIn polish)
 │    │
 │    └─► M3 (Parser foundation) ──────► M5 (Parser wiring + extraction)
 │                                         │
 │                                         └─► M6 (Overlay + answer bank + app questions)
 │                                               │
 │                                               └─► M7 (Infra → 1.0.0)
 │
 └─► M4 (Workday dropdowns) ──────────────► M6
```

- M1 is universal prerequisite
- M2 and M4 are independent after M1 (parallelizable)
- M3 is independent of M4
- M5 requires M3 (parser must exist before wiring)
- M6 requires M5 (overlay needs parsed sections) and M4 (app questions extends dropdown filling)
- M7 requires M6 (infra wraps completed features)

## Forge-Side Schema Changes by Phase

| Phase | Table/Column Changes |
|-------|---------------------|
| M5 | `job_descriptions`: add `parsed_sections` JSON (`salary_min`/`salary_max` already exist, parser auto-populates) |
| M6 | New `answer_bank` table |
| M7 | New `extension_config` table, new `extension_logs` table |

## Deferred to Post-MVP

| Bead | Item | Reason |
|------|------|--------|
| 3bp.21.4 | L3 model fallback | L1+L2 heuristics sufficient; model adds inference dependency |
| 3bp.20 | Session cache | Localhost API latency negligible; cache invalidation complexity not justified |

## Hard Rules (Carried from Prototype)

- All work in worktrees under `.claude/worktrees/`
- Every phase must produce a buildable, dev-installable extension (Chrome + Firefox/Zen)
- Programmatic injection only — no `content_scripts` in manifest
- Background worker must NOT import plugin modules (shared chunk constraint)
- Rebuild `dist/` on main after every merge
- `bd close <id>` with commit SHA on phase completion
- Subagent-driven development for implementation
- Extension version = phase version (0.1.1 = M1, etc.)
