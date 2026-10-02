# Forge Browser Extension — Design Spec

**Status**: Draft
**Date**: 2026-04-10
**Author**: Adam + Claude (brainstorming session)
**Scope**: Browser extension that wraps the Forge HTTP API with two user-facing workflows: JD capture and application autofill.

## 1. System Overview

The Forge Browser Extension is a Chrome (MV3) extension that wraps the existing Forge HTTP API with two user-facing workflows:

1. **JD Capture**: User browses a job posting, clicks the extension icon, extension extracts structured data via a site-specific plugin, resolves/creates the organization, and creates a JD in Forge — all in one click.
2. **Application Autofill**: User navigates to an application form (Workday, Greenhouse), clicks the extension icon, extension fills known profile fields (name, email, phone, etc.) using a plugin's field map, with a generic label-matching fallback.

### Core Constraints

- Forge HTTP API must be running on `localhost:3000`
- Extension is a **thin client** — holds no data, only configuration and a plugin registry
- All persistence lives in the Forge database
- Each supported site ships as a **plugin module** implementing `JobBoardPlugin` with optional capabilities
- Extension discovers the active plugin by hostname matching and calls only the capabilities relevant to the current action

### Phasing Summary

| Phase | Purpose |
|-------|---------|
| **Prototype (P1–P7)** | Incrementally verify extraction, API read, API write, and autofill in isolation |
| **MVP** | Health check, org resolve UX, page overlay, Workday plugin, Firefox support, application answer bank |
| **Post-MVP** | Greenhouse plugin, LLM-assisted field matching, resume-sourced autofill, side panel, Safari support |

## 2. Plugin Contract

The core abstraction. A plugin is a TypeScript module that implements `JobBoardPlugin`:

```typescript
// packages/extension/src/plugin/types.ts

export interface JobBoardPlugin {
  /** Unique plugin identifier, used for logging and config */
  name: string

  /** Hostname patterns this plugin claims. Matched against window.location.hostname. */
  matches: string[]  // e.g., ["linkedin.com", "*.linkedin.com"]

  /** Capabilities are optional — plugin implements only what it supports */
  capabilities: {
    /** Read JD structure from the current page */
    extractJD?: (doc: Document, url: string) => ExtractedJob | null

    /** Read company info from the current page (may duplicate from JD) */
    extractCompany?: (doc: Document, url: string) => ExtractedOrg | null

    /** Normalize a URL to its canonical form (for dedup, P5+) */
    normalizeUrl?: (url: string) => string

    /** Discover fillable form fields on the current page */
    detectFormFields?: (doc: Document) => DetectedField[]

    /** Write a value to a specific form field element */
    fillField?: (element: Element, value: string) => boolean
  }
}

export interface ExtractedJob {
  title: string | null
  company: string | null
  location: string | null
  salary_range: string | null
  description: string | null       // maps to raw_text on Forge JD
  url: string
  extracted_at: string             // ISO timestamp
  source_plugin: string            // plugin name for traceability
  raw_fields?: Record<string, unknown>  // debug payload preserved on failure
}

export interface ExtractedOrg {
  name: string | null
  website: string | null
  location: string | null
  source_plugin: string
}

export interface DetectedField {
  element: Element                 // live DOM reference
  label_text: string | null        // nearby label text (for fallback matching)
  field_kind: FieldKind            // canonical key, see below
  required: boolean
}

export type FieldKind =
  | 'name.full' | 'name.first' | 'name.last'
  | 'email' | 'phone'
  | 'address.city' | 'address.state' | 'address.country'
  | 'profile.linkedin' | 'profile.github' | 'profile.website'
  | 'work_auth.us' | 'work_auth.sponsorship'    // answer-bank targets (MVP)
  | 'eeo.gender' | 'eeo.race' | 'eeo.veteran' | 'eeo.disability'  // answer-bank targets (MVP)
  | 'unknown'                                    // fallback for label matching
```

### Design Decisions

1. **Extraction is synchronous and pure** — plugins take a `Document` and return a POJO. No network calls, no storage access. Trivially testable with static HTML fixtures.
2. **Field kinds are a closed enum** — not free-form strings. The profile mapping layer only needs to know how to fill known kinds. New kinds require an explicit schema change.
3. **`unknown` kind is the fallback** — plugin couldn't categorize the field, but label text is still captured. Post-MVP LLM matching uses these.
4. **`source_plugin` on every extraction** — traceability. When something looks wrong in the modal, you know which plugin's selectors produced it.
5. **No `version` field on the plugin** — plugins are in-repo TypeScript, versioned with the extension. No runtime plugin loading, no version skew.
6. **`raw_fields` preserved on failure** — debug payload visible in error context. Useful fallback for DOM ↔ plugin drift diagnosis.
7. **`normalizeUrl` is per-plugin** — LinkedIn treats `?currentJobId` as canonical, other sites differ. Plugin responsibility.

## 3. Project Layout

```
packages/extension/
├── package.json               # @forge/extension, depends on @forge/sdk workspace
├── manifest.json              # MV3 manifest (Chrome) + Firefox adaptation at build time
├── vite.config.ts             # Builds popup + content scripts + background worker
├── src/
│   ├── background/
│   │   └── index.ts           # Service worker: SDK client, plugin registry, message router
│   ├── popup/
│   │   ├── index.html
│   │   ├── main.ts
│   │   └── Popup.svelte       # Match webui stack (Svelte 5)
│   ├── content/
│   │   ├── linkedin.ts        # Content script bundle: LinkedIn plugin + modal mount
│   │   ├── workday.ts         # Content script bundle: Workday plugin + autofill
│   │   └── shared/
│   │       ├── modal.ts       # P1-era extraction modal (removed after P4)
│   │       └── messaging.ts   # Content ↔ background chrome.runtime.sendMessage wrapper
│   ├── plugin/
│   │   ├── types.ts           # JobBoardPlugin, ExtractedJob, DetectedField, FieldKind
│   │   ├── registry.ts        # matchPluginForHost(hostname): JobBoardPlugin | null
│   │   └── plugins/
│   │       ├── linkedin.ts    # LinkedIn plugin implementation
│   │       └── workday.ts     # Workday plugin (MVP)
│   ├── storage/
│   │   └── config.ts          # chrome.storage.local wrapper (prototype) → Forge config (MVP)
│   └── lib/
│       ├── sdk-client.ts      # ForgeClient singleton with baseUrl from config
│       ├── health.ts          # GET /api/health poller
│       └── errors.ts          # ExtensionError taxonomy, extError() helper
├── tests/
│   ├── fixtures/
│   │   ├── linkedin/
│   │   │   ├── job-detail-standard.html
│   │   │   ├── job-detail-no-salary.html
│   │   │   ├── job-detail-remote-only.html
│   │   │   ├── job-detail-easy-apply.html
│   │   │   └── job-search-listing.html
│   │   └── workday/
│   │       └── application-form.html
│   └── plugins/
│       ├── linkedin.test.ts   # Feed fixture HTML, assert extracted fields
│       └── workday.test.ts
└── dist/                      # Build output, .gitignored
```

### Layout Decisions

1. **One content script per plugin** — not a mega-bundle. Chrome's manifest matches URLs to content scripts, so `linkedin.ts` only runs on linkedin.com. Less permission surface, smaller payloads.
2. **Background service worker is the orchestrator** — popup and content scripts never talk directly. All messages route through background, which owns the SDK client and config. This is the only place HTTP calls happen.
3. **Plugin modules are pure** — `plugin/plugins/linkedin.ts` exports a `JobBoardPlugin` object with no DOM side effects. The content script imports the plugin, calls `extract()`, then handles messaging/UI separately.
4. **Tests use static HTML fixtures** — save real LinkedIn job page HTML to `tests/fixtures/`, feed to `jsdom`, assert extraction output. No live-site scraping in tests.
5. **Svelte 5 for popup** — matches the existing `packages/webui/` stack. Component patterns and types are familiar.
6. **Monorepo workspace package** — `@forge/extension` depends on `@forge/sdk` as a workspace dep (no publishing needed), shares types with core, but has its own manifest/build.

## 4. Prototype Series

Each prototype is independently verifiable and must pass its checklist before the next begins.

### P1 — Pure Extraction

**Proves**: LinkedIn plugin reads DOM, modal renders `ExtractedJob` correctly. Zero backend.

**Data flow**:
```
User on linkedin.com/jobs/view/123
  → clicks extension icon → popup opens
  → popup: "Extract Job" button → click
  → chrome.tabs.sendMessage(activeTab, { cmd: 'extract' })
  → content/linkedin.ts receives
    → imports plugins/linkedin.ts
    → calls plugin.capabilities.extractJD(document, location.href)
    → returns ExtractedJob | null
  → content script mounts modal.ts with extracted payload
  → modal renders structured view + "Copy JSON" + "Close"
  → popup closes
```

**Touch surface**: popup, content script, plugin, modal. No background worker, no network, no storage.

**Success criteria**:
1. Click extension on 5 different LinkedIn job pages → modal shows correct title/company/location/description/salary
2. Fields missing from the DOM render as `(not found)` rather than silently dropping
3. Plugin interface shape validated by surviving first contact with real LinkedIn markup
4. `Copy JSON` button copies a valid JSON object to clipboard
5. Click extension on a non-LinkedIn site → popup shows `NO_PLUGIN_FOR_HOST` toast
6. All unit tests pass: `bun test plugins/linkedin`

### P2 — API Read-Only Sanity

**Proves**: Extension can reach Forge, health check works, SDK-based reads succeed. Includes smoke test.

**Data flow**:
```
Popup opens
  → background worker boots
    → loads chrome.storage.local config (baseUrl = 'http://localhost:3000')
    → instantiates ForgeClient({ baseUrl })
    → calls GET /api/health
    → sets popup green/red dot
  → popup: "List Organizations" button (debug mode)
  → click → chrome.runtime.sendMessage({ cmd: 'orgs.list' })
  → background: sdk.organizations.list()
  → returns Result<Organization[]>
  → popup renders count + first 5 org names
```

**Dependencies**:
- Forge-side: **CORS allowlist for `chrome-extension://*`** (must land first, see Section 8)

**Touch surface**: background worker, SDK bundle, chrome.storage.local, CORS.

**Success criteria**:
1. Health dot green when Forge running, red when stopped
2. Org list returns same data as webui shows
3. Smoke test passes: extension can complete a full popup → background → SDK → API roundtrip against a running Forge server
4. Red-state behavior verified: stop Forge mid-session, click list → user-friendly `API_UNREACHABLE` toast

### P3 — API Write (Simple)

**Proves**: Write path works end-to-end with hardcoded payload.

**Data flow**:
```
Popup: "Create Test Org" button
  → sends { cmd: 'orgs.create', payload: { name: 'Test Org ' + Date.now() } }
  → background: sdk.organizations.create(payload)
  → returns Result<Organization>
  → success: popup shows toast "Created Org <id>"
  → failure: popup shows toast with error code + message
```

**Touch surface**: write path, error handling UI, idempotency (timestamp prevents collisions).

**Success criteria**:
1. Created org appears in Forge webui
2. Error toast appears if Forge is down
3. Validation error toast appears if payload is malformed

### P4 — Extraction → API Write

**Proves**: Full capture flow with no dedup. P1 modal removed in this prototype.

**Data flow**:
```
User on linkedin.com/jobs/view/123
  → popup: "Capture Job" → sends cmd: 'jd.captureActive'
  → background: chrome.tabs.sendMessage(activeTab, { cmd: 'extract' })
    → content script returns ExtractedJob
  → background: validateExtraction(extracted)
    → if invalid, toast EXTRACTION_INCOMPLETE and stop
  → background: sdk.jobDescriptions.create({
      title: extracted.title,
      raw_text: extracted.description,
      url: extracted.url,
      location: extracted.location,
      organization_id: null,  // no dedup yet
    })
  → success: popup toast "JD created: <id>"
  → failure: popup toast with error
```

**Touch surface**: content → background → SDK flow, extraction validation, error propagation.

**Success criteria**:
1. JD appears in Forge webui with correct title, description, url, location
2. Missing `title` or `description` triggers `EXTRACTION_INCOMPLETE` toast, no DB write
3. P1 modal code removed from repo
4. All LinkedIn fixture tests still pass

### P5 — Extraction + Dedup → API Write

**Proves**: Org resolution and JD dedup reduce duplicate creations.

**Data flow**:
```
... same extraction as P4 ...
  → background receives valid ExtractedJob
  → plugin.normalizeUrl(extracted.url) → canonical URL
  → jd dedup:
    → sdk.jobDescriptions.lookupByUrl(canonicalUrl)  // NEW endpoint
    → if found → toast "JD already captured" with link, skip create
  → org resolution:
    → sdk.organizations.list({ search: extracted.company })
    → if exact name match → use that org_id
    → if no match → sdk.organizations.create({ name: extracted.company, website })
    → if multiple matches → prototype picks first, MVP disambiguates
  → sdk.jobDescriptions.create({ ...payload, organization_id })
  → success: popup toast "JD created: <id>" with link
```

**Dependencies**:
- Forge-side: **`POST /api/job-descriptions/lookup-by-url`** endpoint (does not exist today)

**Touch surface**: search + match logic, conditional create paths, URL normalization.

**Success criteria**:
1. Capturing same URL twice → "already exists" toast on second attempt
2. Company with exact name match reuses existing org
3. Unknown company creates new org and links JD to it
4. URL normalization verified: `?utm_source` tracking params stripped before lookup

### P6 — Field Filling (Simple)

**Proves**: Content script can write to DOM form fields on a Workday page.

**Data flow**:
```
User on Workday form page
  → popup: "Test Fill" → sends cmd: 'form.testFill'
  → background: chrome.tabs.sendMessage(activeTab, { cmd: 'testFill' })
  → content/workday.ts receives
    → imports plugins/workday.ts
    → calls plugin.capabilities.detectFormFields(document)
    → returns DetectedField[]
    → for each field, plugin.fillField(field.element, 'HARDCODED-' + field.field_kind)
  → returns { filled: count, failed: count }
  → popup toast: "Filled 12 fields"
```

**Touch surface**: Workday plugin, DOM write permissions, field discovery via `data-automation-id`.

**Success criteria**:
1. Hardcoded strings appear in form inputs on a real Workday application
2. `fillField` returns correct success/failure count
3. Framework change detection verified (no ghost values that revert on blur)

### P7 — Field Filling + API Read (Profile Source)

**Proves**: End-to-end autofill roundtrip from Forge profile.

**Data flow**:
```
... same detection as P6 ...
  → background loads profile: sdk.userProfile.get()  // NEW endpoint
  → builds FieldKind → value map from profile
  → for each DetectedField, look up profile value by field_kind
    → skip fields with unknown kind or missing value
    → call plugin.fillField(field.element, value)
  → popup toast: "Filled 8/12 fields (4 require answer bank)"
```

**Dependencies**:
- P6 (field filling) must land first
- Forge-side: **`GET /api/profile`** endpoint (does not exist today)

**Touch surface**: profile read endpoint, FieldKind → profile mapping, partial fills.

**Success criteria**:
1. Name/email/phone fill with real profile values on a Workday form
2. EEO/work-auth fields detected but not filled (answer bank is MVP)
3. Partial fill count accurate

## 5. Error Handling

### Error Shape

```typescript
// packages/extension/src/lib/errors.ts

export interface ExtensionError {
  code: ExtensionErrorCode         // enumerated, stable identifiers
  message: string                   // human-readable, safe for toast
  layer: 'plugin' | 'content' | 'background' | 'popup' | 'sdk'
  plugin?: string                   // source plugin if known
  url?: string                      // page URL if relevant
  context?: Record<string, unknown> // debug payload (raw_fields, selectors, etc.)
  cause?: ExtensionError            // wrapped underlying error
  timestamp: string                 // ISO
}

export function extError(
  code: ExtensionErrorCode,
  message: string,
  opts?: Partial<Omit<ExtensionError, 'code' | 'message' | 'timestamp'>>
): ExtensionError {
  return {
    code,
    message,
    layer: opts?.layer ?? 'background',
    plugin: opts?.plugin,
    url: opts?.url,
    context: opts?.context,
    cause: opts?.cause,
    timestamp: new Date().toISOString(),
  }
}
```

### Error Codes

```typescript
export type ExtensionErrorCode =
  // Extraction errors (P1, P4, P5)
  | 'NO_PLUGIN_FOR_HOST'           // hostname didn't match any registered plugin
  | 'PLUGIN_THREW'                 // plugin.extract threw unexpectedly
  | 'EXTRACTION_INCOMPLETE'        // required fields missing (title or raw_text null)
  | 'EXTRACTION_EMPTY'             // plugin returned null (page structure unrecognized)
  // API errors (P2–P7)
  | 'API_UNREACHABLE'              // fetch network error, server not running
  | 'API_CORS_BLOCKED'             // preflight failed
  | 'API_VALIDATION_FAILED'        // 400 with validation error
  | 'API_DUPLICATE'                // URL already exists (P5)
  | 'API_NOT_FOUND'                // 404 lookup
  | 'API_INTERNAL_ERROR'           // 500, unknown server error
  | 'API_TIMEOUT'                  // request exceeded timeout
  // Dedup errors (P5)
  | 'ORG_AMBIGUOUS'                // multiple orgs matched fuzzy search (MVP)
  // Autofill errors (P6, P7)
  | 'FORM_NOT_DETECTED'            // no form fields found
  | 'FIELD_WRITE_FAILED'           // fillField returned false
  | 'PROFILE_NOT_AVAILABLE'        // P7: /api/profile returned no data
  // Configuration errors
  | 'CONFIG_MISSING'               // expected config key not set
  | 'UNKNOWN_ERROR'                // catch-all (never intentionally emitted)
```

### Partial Extraction Rule

Required fields: `title` and `description` (→ `raw_text`). Everything else can be null.

If either required field is missing, reject the entire capture with `EXTRACTION_INCOMPLETE` and surface `raw_fields` in the error context for diagnosis. **No partial writes**: partial data is worse than no data because it pollutes the DB.

### Logging Destinations

1. **Browser console** — primary dev feedback. Every error logged with full context.
2. **`chrome.runtime` last-error surface** — Chrome's own lifecycle error reporting.
3. **Forge server logs** (MVP+) — `POST /api/extension/log` sends serialized `ExtensionError`. Gated by config flag. Prototype scope: console only.

### Toast Messages

Toasts are **short, actionable, never show the error code**. The code is for logs and dev mode.

| Code | Toast text | Action hint |
|------|-----------|-------------|
| `NO_PLUGIN_FOR_HOST` | "No plugin for this site yet" | "Try a supported site" |
| `EXTRACTION_INCOMPLETE` | "Couldn't extract job title and description" | "The page structure may have changed" |
| `EXTRACTION_EMPTY` | "No job found on this page" | "Make sure you're on a job detail page" |
| `API_UNREACHABLE` | "Forge is not running" | "Start Forge with `bun dev`" |
| `API_DUPLICATE` | "Job already captured" | "[View in Forge]" link |
| `PLUGIN_THREW` | "Plugin error — see console" | (dev mode: link to console) |
| `FIELD_WRITE_FAILED` | "Some fields couldn't be filled" | (dev mode: list failed field_kinds) |
| `PROFILE_NOT_AVAILABLE` | "Profile not loaded from Forge" | "Check Forge is running" |

### Dev Mode

`config.devMode: boolean` in `chrome.storage.local`:
- **On**: toasts show error code, popup has "Show last error" button opening a panel with full `ExtensionError` JSON
- **Off**: user-friendly messages only

Prototype ships with `devMode: true` by default. MVP defaults to `false` but keeps the toggle.

## 6. Configuration

### Prototype (chrome.storage.local)

```typescript
interface ExtensionConfig {
  baseUrl: string                  // 'http://localhost:3000'
  devMode: boolean                 // true in prototype
  enabledPlugins: string[]         // plugin names to activate
}
```

Simple, works offline, each browser install has its own config.

### MVP (Forge-hosted)

Extension fetches `GET /api/extension/config` on startup. Config lives in the Forge DB, UI for editing in webui. Migration from prototype: swap the storage layer, keep the `ExtensionConfig` interface identical. Single source of truth with the webui.

## 7. Testing Strategy

### Prototype (P1–P7)

1. **Plugin unit tests** — feed fixture HTML to jsdom, assert extraction output against expected `ExtractedJob`. Fast, isolated, run on every change.
2. **Error path tests** — feed malformed/empty fixtures, assert plugin returns null (not throws). Assert validator rejects incomplete extractions with correct error code.
3. **Smoke test** — P2+ includes a "does the extension actually talk to Forge" integration check against a running Forge server.
4. **Manual verification** — each prototype has a written verification checklist (in the phase plan doc). Run on real LinkedIn/Workday pages before declaring the prototype done.

### MVP adds

5. **Integration tests** — spin up Forge against a test DB, exercise full extension → API flow using Puppeteer/Playwright loading the extension. Capture a fixture, POST it, assert JD created, assert dedup on second capture.
6. **CORS regression tests** — verify preflight from `chrome-extension://*` origin.

### Post-MVP adds

7. **Cross-browser tests** — same integration suite against Firefox via `webextension-polyfill`.

### Fixture Strategy

```
tests/fixtures/linkedin/
├── job-detail-standard.html        # baseline
├── job-detail-no-salary.html       # missing optional field
├── job-detail-remote-only.html     # location edge case
├── job-detail-easy-apply.html      # Easy Apply UI variant
└── job-search-listing.html         # non-detail page (plugin returns null)
```

Capture: right-click job container in dev tools → "Copy outer HTML" → paste into fixture file. Trim `<script>` and `<style>`. Comment with capture date at top.

Recapture when: plugin tests start failing against production pages. The fixture is the contract.

### Verification Checklists

- **Location**: per-phase plan docs (as direction), per-phase commit messages (as attestation).
- **Format**: markdown checkboxes that must be signed off before merge.

### What's Explicitly NOT Tested (prototype)

- Live LinkedIn scraping — brittle, rate-limited, requires auth. Fixtures only.
- Real Forge DB mutations in unit tests — mocked SDK client. Integration tests exercise real DB.
- Browser UI snapshot tests — popup too small to warrant.

## 8. Worktree & Phasing Strategy

### Worktree Layout

```
/Users/adam/notes/job-hunting/.claude/worktrees/
├── forge-ext-p1-extraction/
├── forge-ext-p2-api-read/
├── forge-ext-p3-api-write/
├── forge-ext-p4-extract-write/
├── forge-ext-p5-dedup/
├── forge-ext-p6-fill-simple/
├── forge-ext-p7-fill-profile/
├── forge-core-profile-endpoint/      # dependency for P7
├── forge-core-jd-lookup-by-url/      # dependency for P5
├── forge-core-extension-cors/        # dependency for P2
├── forge-ext-mvp-healthcheck/
├── forge-ext-mvp-orgresolve/
├── forge-ext-mvp-workday/
├── forge-ext-mvp-firefox/
└── ...
```

### Branch Naming

```
feat/forge-ext/p1-extraction
feat/forge-ext/p2-api-read
...
feat/forge-core/profile-endpoint
feat/forge-core/jd-lookup-by-url
feat/forge-core/extension-cors
feat/forge-ext/mvp-healthcheck
```

### Dependency Graph

```
P1 ─┐
    ├─> P4 ─> P5 ─┐
P2 ─┤            ├─> (MVP phases)
    └─> P3 ──────┘

P6 ──> P7 ────────────> (MVP autofill phases)

Forge-side dependencies (must land before dependent extension phase):
- forge-core-extension-cors      before P2
- forge-core-jd-lookup-by-url    before P5
- forge-core-profile-endpoint    before P7
```

### Parallelization

- **P1 and P2 run in parallel** when possible — different worktrees, no shared code
- **P3 depends on P2** — can start once P2 proves SDK wiring
- **P6 is independent** of the P1–P5 chain — can start any time
- **P7 depends on P6** + `forge-core-profile-endpoint`

### Forge-Side Dependencies

Per-phase residents, documented in their dependent phase plan docs. When converting plans → beads, these become separate bead issues with blocking relationships.

| Endpoint/Change | Needed by | Worktree |
|----------------|-----------|----------|
| CORS allowlist for `chrome-extension://*` | P2 | `forge-core-extension-cors/` |
| `POST /api/job-descriptions/lookup-by-url` | P5 | `forge-core-jd-lookup-by-url/` |
| `GET /api/profile` | P7 | `forge-core-profile-endpoint/` |

### Commit Discipline per Worktree

Each worktree produces **one merge commit** back to main containing:
- All implementation
- Tests (unit fixtures + smoke tests where applicable)
- Documentation updates (README, spec amendments)
- **Filled verification checklist in commit body**

No force-pushes. No rebasing shared branches. Merge commits allow clean reverts of a single phase.

### Cleanup

After merge, worktree is removed (`git worktree remove`). Branch stays for history.

## 9. Phase Summary

### Prototype

| # | Phase | Dependencies | Key deliverable |
|---|-------|-------------|-----------------|
| P1 | Pure extraction | — | LinkedIn plugin + debug modal |
| P2 | API read-only sanity | `forge-core-extension-cors` | Health check + org list + smoke test |
| P3 | API write (simple) | P2 | Hardcoded org create |
| P4 | Extraction → write | P1 + P3 | Full capture, modal removed |
| P5 | Extraction + dedup → write | P4 + `forge-core-jd-lookup-by-url` | URL dedup + org resolve |
| P6 | Field filling (simple) | — | Workday plugin + hardcoded fill |
| P7 | Field filling + profile read | P6 + `forge-core-profile-endpoint` | Name/email/phone autofill |

### MVP

- Forge server health check in popup (persistent indicator)
- Org resolve/create with disambiguation UX (multiple-match handling)
- Page overlay with review-before-submit (editable extracted fields)
- Workday plugin (extraction + autofill)
- Firefox support (WebExtensions polyfill, manifest adaptations)
- Application answer bank (stored EEO, work auth, sponsorship — reusable across forms)
- Forge-hosted config migration (`GET /api/extension/config`)
- Forge server logging endpoint (`POST /api/extension/log`)

### Post-MVP

- Greenhouse plugin (validates generic plugin interface against a second site)
- LLM-assisted field matching (`POST /api/extension/match-fields`)
- Resume selection for autofill (profile → resume → work history → form)
- Side panel UI
- Safari support

## 10. Open Questions & Known Unknowns

1. **Workday framework interaction** — whether naive `element.value = x` triggers Workday's React change detection. May need synthetic event dispatch. Resolved in P6 research.
2. **LinkedIn auth walls** — does the plugin work on public job pages without login, or does the user need to be logged in? Resolved in P1 testing.
3. **URL normalization rules** — per-plugin `normalizeUrl()` — exact canonical forms TBD per plugin in implementation.
4. **CORS origin allowlist** — exact `chrome-extension://<id>` value depends on extension ID assigned at load time. Unpacked dev extensions get a stable ID from a `key` field in manifest. Resolved in `forge-core-extension-cors` worktree.

## 10a. Cross-Cutting Phase Requirement (Buildable & Installable)

**Every phase must produce a buildable, dev-installable extension.**

At the end of each phase — prototype, MVP, or post-MVP — the user must be able to:

1. Run `cd packages/extension && bun run build` with no errors
2. Open `chrome://extensions`, enable Developer mode, click "Load unpacked", select `packages/extension/dist/`
3. See the extension loaded with no manifest errors, no service worker errors, no content script errors in the extension's console
4. Click the extension icon on a relevant page and observe the phase's user-visible functionality working

This is a **non-negotiable gate**. A phase is not complete until the user has personally built and installed it and verified that the phase's stated deliverable works on real pages.

Phase plans MUST include explicit checklist items for:
- `bun run build` succeeds
- Extension is dev-installable (loads without errors)
- The phase's key deliverable works when clicked in the installed extension

This requirement is enforced at merge time — no phase merges until the user signs off on the build + install verification.

## 11. Success Definition

**Prototype phase complete** when:
- All 7 prototype checklists signed off
- P5 captures a real LinkedIn JD end-to-end in one click with dedup working
- P7 fills name/email/phone on a real Workday form from Forge profile data

**MVP phase complete** when:
- User can capture a JD from LinkedIn in one click with org resolution and review
- User can autofill a Workday application with profile data + answer bank
- Extension works on Firefox
- No data loss or duplicate orgs on happy path

**Post-MVP phase complete** when:
- Extension works on any job site via plugin or generic fallback
- Resume work history autofill works
- Safari parity with Chrome
