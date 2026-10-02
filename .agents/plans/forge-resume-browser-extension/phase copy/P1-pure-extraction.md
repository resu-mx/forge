# P1 — Pure Extraction (LinkedIn)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold `packages/extension/` with a working Chrome MV3 browser extension that extracts structured job data from LinkedIn job pages using a plugin-based architecture, and displays the extracted data in an in-page modal. Zero backend integration.

**Architecture:** Vite + TypeScript + Svelte 5 monorepo workspace package. A pure `JobBoardPlugin` TypeScript module reads the DOM and returns an `ExtractedJob` POJO. A content script injected into LinkedIn pages calls the plugin and mounts a debug modal with the extracted payload. A popup with one button triggers the content script via `chrome.tabs.sendMessage`.

**Tech Stack:** Bun workspace, Vite 5, `@crxjs/vite-plugin` (extension build), `@sveltejs/vite-plugin-svelte`, Svelte 5, TypeScript, `bun:test` + `jsdom` for plugin unit tests.

**Worktree:** `.claude/worktrees/forge-ext-p1-extraction/` on branch `feat/forge-ext/p1-extraction`.

**Depends on:** Nothing (can run in parallel with P2 and P0).

**Blocks:** P4 (Extraction → API Write).

---

## Context You Need

### What a Chrome MV3 extension is

A Chrome MV3 (Manifest V3) extension is a zip of web assets defined by a `manifest.json`:
- `action.default_popup` — HTML file shown when user clicks the extension icon
- `content_scripts[]` — TypeScript/JS files injected into pages matching URL patterns
- `background.service_worker` — long-running worker for lifecycle/network (not used in P1)
- `permissions` — capability requests (activeTab, storage, etc.)

**P1 uses only popup + content script.** No background worker, no storage, no network.

### How the extension will run during development

1. `bun run dev` in `packages/extension/` starts vite watch mode, emitting `dist/` continuously
2. User opens `chrome://extensions` → Enable "Developer mode" → "Load unpacked" → select `packages/extension/dist/`
3. Chrome loads the manifest from `dist/manifest.json` and assigns an extension ID
4. User navigates to a LinkedIn job page, clicks extension icon → popup opens → "Extract Job" button

### Plugin contract (from SPEC Section 2, reproduced verbatim for reference)

```typescript
export interface JobBoardPlugin {
  name: string
  matches: string[]  // hostname patterns
  capabilities: {
    extractJD?: (doc: Document, url: string) => ExtractedJob | null
    extractCompany?: (doc: Document, url: string) => ExtractedOrg | null
    normalizeUrl?: (url: string) => string
    detectFormFields?: (doc: Document) => DetectedField[]
    fillField?: (element: Element, value: string) => boolean
  }
}

export interface ExtractedJob {
  title: string | null
  company: string | null
  location: string | null
  salary_range: string | null
  description: string | null
  url: string
  extracted_at: string
  source_plugin: string
  raw_fields?: Record<string, unknown>
}
```

**P1 scope**: LinkedIn plugin implements `extractJD` only. Other capabilities are undefined on the plugin object.

### LinkedIn job page DOM research (caveat)

LinkedIn job page structure at the time of writing uses:
- Title: `h1.top-card-layout__title` or `h1.topcard__title` (public view) or `[data-test-id="job-title"]`
- Company: `a[data-tracking-control-name="public_jobs_topcard-org-name"]` or `.topcard__org-name-link`
- Location: `.topcard__flavor--bullet` (first child)
- Description: `div.show-more-less-html__markup` (public) or `.jobs-description__content` (logged in)
- Salary: not reliably present; when shown, inside `.compensation__salary-range` or similar

**Fixtures will be captured from the real site** rather than coded against assumed selectors. The plugin is written iteratively: capture a fixture, write a failing test asserting expected fields, make the test pass by finding the actual selectors in the fixture.

---

## File Structure

```
packages/extension/
├── package.json                         # @forge/extension, new workspace
├── tsconfig.json                        # TS config for vite
├── svelte.config.js                     # Svelte 5 config
├── vite.config.ts                       # Vite + CRXJS + svelte plugin
├── manifest.json                        # MV3 manifest (processed by CRXJS)
├── src/
│   ├── popup/
│   │   ├── index.html                   # popup entry HTML
│   │   ├── main.ts                      # Svelte mount
│   │   └── Popup.svelte                 # Single "Extract Job" button
│   ├── content/
│   │   ├── linkedin.ts                  # Content script: plugin + modal mount
│   │   └── shared/
│   │       └── modal.ts                 # Debug modal (removed in P4)
│   ├── plugin/
│   │   ├── types.ts                     # JobBoardPlugin, ExtractedJob, etc.
│   │   ├── registry.ts                  # matchPluginForHost(hostname)
│   │   └── plugins/
│   │       └── linkedin.ts              # LinkedIn plugin implementation
│   └── lib/
│       └── errors.ts                    # ExtensionError + extError() helper
├── tests/
│   ├── fixtures/
│   │   └── linkedin/
│   │       ├── job-detail-standard.html    # real capture
│   │       └── job-search-listing.html     # non-detail page (returns null)
│   └── plugins/
│       └── linkedin.test.ts             # jsdom-based extraction tests
└── dist/                                # vite build output, gitignored
```

**Files split by responsibility:**
- `plugin/plugins/linkedin.ts` — pure extraction logic, no DOM side effects, testable standalone
- `content/linkedin.ts` — glue that calls the plugin and mounts the modal, runs only in the browser
- `content/shared/modal.ts` — debug UI, isolated so it can be deleted cleanly in P4
- `lib/errors.ts` — error taxonomy, grows in later phases, starts with just the P1 codes

---

## Task 1: Worktree & Package Scaffold

- [ ] **Step 1.1: Create worktree**

```bash
cd /Users/adam/notes/job-hunting
git worktree add .claude/worktrees/forge-ext-p1-extraction -b feat/forge-ext/p1-extraction
cd .claude/worktrees/forge-ext-p1-extraction
```

- [ ] **Step 1.2: Create the package directory**

```bash
mkdir -p packages/extension/src/popup
mkdir -p packages/extension/src/content/shared
mkdir -p packages/extension/src/plugin/plugins
mkdir -p packages/extension/src/lib
mkdir -p packages/extension/tests/fixtures/linkedin
mkdir -p packages/extension/tests/plugins
```

- [ ] **Step 1.3: Create `packages/extension/package.json`**

```json
{
  "name": "@forge/extension",
  "version": "0.0.1",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "test": "bun test"
  },
  "dependencies": {
    "svelte": "^5.0.0"
  },
  "devDependencies": {
    "@crxjs/vite-plugin": "^2.0.0-beta.25",
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "@types/bun": "latest",
    "@types/chrome": "^0.0.280",
    "jsdom": "^25.0.0",
    "@types/jsdom": "^21.1.7",
    "svelte-check": "^4.0.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0"
  }
}
```

- [ ] **Step 1.4: Install dependencies**

From the worktree root:

```bash
bun install
```

Expected: new workspace discovered, deps installed. If `@crxjs/vite-plugin` fails to resolve, the version may need bumping — check the latest at https://www.npmjs.com/package/@crxjs/vite-plugin and update the plan file before continuing.

- [ ] **Step 1.5: Create `packages/extension/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "allowSyntheticDefaultImports": true,
    "types": ["chrome", "bun", "svelte"],
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "resolveJsonModule": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "noEmit": true
  },
  "include": ["src/**/*.ts", "src/**/*.svelte", "tests/**/*.ts"]
}
```

- [ ] **Step 1.6: Create `packages/extension/svelte.config.js`**

```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'

export default {
  preprocess: vitePreprocess(),
}
```

- [ ] **Step 1.7: Create `packages/extension/.gitignore`**

```
dist/
node_modules/
*.log
```

- [ ] **Step 1.8: Commit scaffold**

```bash
git add packages/extension/package.json packages/extension/tsconfig.json packages/extension/svelte.config.js packages/extension/.gitignore bun.lock
git commit -m "chore(ext): scaffold @forge/extension workspace package"
```

---

## Task 2: Plugin Types (types.ts)

**Files:**
- Create: `packages/extension/src/plugin/types.ts`

- [ ] **Step 2.1: Write the types file**

```typescript
// packages/extension/src/plugin/types.ts

/** Canonical field kinds for autofill (autofill kinds used in later phases, included now for stability) */
export type FieldKind =
  | 'name.full' | 'name.first' | 'name.last'
  | 'email' | 'phone'
  | 'address.city' | 'address.state' | 'address.country'
  | 'profile.linkedin' | 'profile.github' | 'profile.website'
  | 'work_auth.us' | 'work_auth.sponsorship'
  | 'eeo.gender' | 'eeo.race' | 'eeo.veteran' | 'eeo.disability'
  | 'unknown'

export interface ExtractedJob {
  title: string | null
  company: string | null
  location: string | null
  salary_range: string | null
  description: string | null       // maps to raw_text on Forge JD
  url: string
  extracted_at: string             // ISO timestamp
  source_plugin: string            // plugin name for traceability
  raw_fields?: Record<string, unknown>
}

export interface ExtractedOrg {
  name: string | null
  website: string | null
  location: string | null
  source_plugin: string
}

export interface DetectedField {
  element: Element
  label_text: string | null
  field_kind: FieldKind
  required: boolean
}

export interface JobBoardPlugin {
  /** Unique plugin identifier, used for logging and config */
  name: string

  /** Hostname patterns this plugin claims. Matched against window.location.hostname. */
  matches: string[]

  /** Capabilities are optional — plugin implements only what it supports */
  capabilities: {
    extractJD?: (doc: Document, url: string) => ExtractedJob | null
    extractCompany?: (doc: Document, url: string) => ExtractedOrg | null
    normalizeUrl?: (url: string) => string
    detectFormFields?: (doc: Document) => DetectedField[]
    fillField?: (element: Element, value: string) => boolean
  }
}
```

- [ ] **Step 2.2: Commit**

```bash
git add packages/extension/src/plugin/types.ts
git commit -m "feat(ext): define JobBoardPlugin contract and types"
```

---

## Task 3: Error Taxonomy (errors.ts)

**Files:**
- Create: `packages/extension/src/lib/errors.ts`

- [ ] **Step 3.1: Write the errors module**

```typescript
// packages/extension/src/lib/errors.ts

export type ExtensionErrorCode =
  // Extraction errors (P1)
  | 'NO_PLUGIN_FOR_HOST'
  | 'PLUGIN_THREW'
  | 'EXTRACTION_INCOMPLETE'
  | 'EXTRACTION_EMPTY'
  // API errors (added in P2+)
  | 'API_UNREACHABLE'
  | 'API_CORS_BLOCKED'
  | 'API_VALIDATION_FAILED'
  | 'API_DUPLICATE'
  | 'API_NOT_FOUND'
  | 'API_INTERNAL_ERROR'
  | 'API_TIMEOUT'
  // Dedup errors (P5+)
  | 'ORG_AMBIGUOUS'
  // Autofill errors (P6+)
  | 'FORM_NOT_DETECTED'
  | 'FIELD_WRITE_FAILED'
  | 'PROFILE_NOT_AVAILABLE'
  // Configuration errors
  | 'CONFIG_MISSING'
  | 'UNKNOWN_ERROR'

export interface ExtensionError {
  code: ExtensionErrorCode
  message: string
  layer: 'plugin' | 'content' | 'background' | 'popup' | 'sdk'
  plugin?: string
  url?: string
  context?: Record<string, unknown>
  cause?: ExtensionError
  timestamp: string
}

export function extError(
  code: ExtensionErrorCode,
  message: string,
  opts: Partial<Omit<ExtensionError, 'code' | 'message' | 'timestamp'>> = {},
): ExtensionError {
  return {
    code,
    message,
    layer: opts.layer ?? 'background',
    plugin: opts.plugin,
    url: opts.url,
    context: opts.context,
    cause: opts.cause,
    timestamp: new Date().toISOString(),
  }
}
```

- [ ] **Step 3.2: Commit**

```bash
git add packages/extension/src/lib/errors.ts
git commit -m "feat(ext): add ExtensionError taxonomy and extError helper"
```

---

## Task 4: Plugin Registry

**Files:**
- Create: `packages/extension/src/plugin/registry.ts`
- Test: `packages/extension/tests/plugins/registry.test.ts`

- [ ] **Step 4.1: Write the failing registry test**

```typescript
// packages/extension/tests/plugins/registry.test.ts

import { describe, test, expect } from 'bun:test'
import { matchPluginForHost } from '../../src/plugin/registry'
import type { JobBoardPlugin } from '../../src/plugin/types'

const fakePlugin: JobBoardPlugin = {
  name: 'test',
  matches: ['example.com', '*.example.com'],
  capabilities: {},
}

describe('matchPluginForHost', () => {
  test('matches exact hostname', () => {
    expect(matchPluginForHost('example.com', [fakePlugin])).toBe(fakePlugin)
  })

  test('matches subdomain via wildcard', () => {
    expect(matchPluginForHost('www.example.com', [fakePlugin])).toBe(fakePlugin)
    expect(matchPluginForHost('jobs.example.com', [fakePlugin])).toBe(fakePlugin)
  })

  test('returns null for unmatched host', () => {
    expect(matchPluginForHost('other.com', [fakePlugin])).toBeNull()
  })

  test('does not match wildcard as literal', () => {
    // "*.example.com" should not match "example.com" (bare domain)
    const subOnly: JobBoardPlugin = { ...fakePlugin, matches: ['*.example.com'] }
    expect(matchPluginForHost('example.com', [subOnly])).toBeNull()
    expect(matchPluginForHost('www.example.com', [subOnly])).toBe(subOnly)
  })

  test('returns first matching plugin', () => {
    const p1: JobBoardPlugin = { ...fakePlugin, name: 'first' }
    const p2: JobBoardPlugin = { ...fakePlugin, name: 'second' }
    expect(matchPluginForHost('example.com', [p1, p2])?.name).toBe('first')
  })
})
```

- [ ] **Step 4.2: Run the test to confirm it fails**

From `packages/extension/`:

```bash
bun test tests/plugins/registry.test.ts
```

Expected: all tests fail with "matchPluginForHost is not defined" or similar import error.

- [ ] **Step 4.3: Implement the registry**

```typescript
// packages/extension/src/plugin/registry.ts

import type { JobBoardPlugin } from './types'

/**
 * Find the first plugin whose `matches` patterns include the given hostname.
 * Supports exact matches ("example.com") and wildcard subdomains ("*.example.com").
 * Wildcards do NOT match the bare domain.
 */
export function matchPluginForHost(
  hostname: string,
  plugins: JobBoardPlugin[],
): JobBoardPlugin | null {
  for (const plugin of plugins) {
    for (const pattern of plugin.matches) {
      if (pattern === hostname) return plugin
      if (pattern.startsWith('*.')) {
        const suffix = pattern.slice(2)  // drop "*."
        if (hostname.endsWith(`.${suffix}`)) return plugin
      }
    }
  }
  return null
}
```

- [ ] **Step 4.4: Run the test to confirm it passes**

```bash
bun test tests/plugins/registry.test.ts
```

Expected: all 5 tests PASS.

- [ ] **Step 4.5: Commit**

```bash
git add packages/extension/src/plugin/registry.ts packages/extension/tests/plugins/registry.test.ts
git commit -m "feat(ext): plugin registry with hostname + wildcard matching"
```

---

## Task 5: Capture First LinkedIn Fixture

**Files:**
- Create: `packages/extension/tests/fixtures/linkedin/job-detail-standard.html`

- [ ] **Step 5.1: Capture a real LinkedIn job detail page**

In a real browser:
1. Open https://www.linkedin.com/jobs/view/ and pick any active job (doesn't need to be relevant to you)
2. Right-click anywhere on the page → Inspect → select the `<html>` element
3. Right-click → Copy → Copy outerHTML
4. Paste into `packages/extension/tests/fixtures/linkedin/job-detail-standard.html`

- [ ] **Step 5.2: Trim the fixture**

The raw capture will be 5-10MB. Trim it to ~200-500KB:
- Delete all `<script>` tags and contents (find/replace in editor)
- Delete all `<style>` tags (or reduce to empty `<style></style>`)
- Delete inline `style="..."` attributes
- Delete tracking beacon `<img>` tags
- Keep all text content and structural elements

At the top of the file, add an HTML comment:

```html
<!-- Captured: YYYY-MM-DD from linkedin.com/jobs/view/<id> (real job) -->
<!-- Trimmed: scripts, styles, tracking pixels removed -->
```

**Why trim**: large fixtures slow the test suite and make diff review hard when the fixture needs to be recaptured.

- [ ] **Step 5.3: Record the expected fields from the capture**

Look at the fixture by eye and note the **actual values** visible on the page. Create `packages/extension/tests/fixtures/linkedin/job-detail-standard.expected.json`:

```json
{
  "title": "ACTUAL_JOB_TITLE_FROM_FIXTURE",
  "company": "ACTUAL_COMPANY_NAME",
  "location": "ACTUAL_LOCATION_STRING",
  "url_suffix": "/jobs/view/ACTUAL_JOB_ID"
}
```

These values become the assertion targets in the plugin test. **Do not guess** — open the fixture HTML in a browser and copy the exact displayed values.

- [ ] **Step 5.4: Commit the fixture**

```bash
git add packages/extension/tests/fixtures/linkedin/
git commit -m "test(ext): capture first LinkedIn job detail fixture"
```

---

## Task 6: LinkedIn Plugin — Test First

**Files:**
- Create: `packages/extension/tests/plugins/linkedin.test.ts`

- [ ] **Step 6.1: Write the failing test**

```typescript
// packages/extension/tests/plugins/linkedin.test.ts

import { describe, test, expect, beforeAll } from 'bun:test'
import { JSDOM } from 'jsdom'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { linkedinPlugin } from '../../src/plugin/plugins/linkedin'

const fixtureDir = join(import.meta.dir, '..', 'fixtures', 'linkedin')

function loadFixture(name: string): { dom: JSDOM; expected: Record<string, string> } {
  const html = readFileSync(join(fixtureDir, `${name}.html`), 'utf-8')
  const expected = JSON.parse(
    readFileSync(join(fixtureDir, `${name}.expected.json`), 'utf-8'),
  )
  const dom = new JSDOM(html, { url: `https://www.linkedin.com${expected.url_suffix}` })
  return { dom, expected }
}

describe('linkedinPlugin.extractJD', () => {
  test('extracts fields from job detail fixture', () => {
    const { dom, expected } = loadFixture('job-detail-standard')
    const extract = linkedinPlugin.capabilities.extractJD
    expect(extract).toBeDefined()
    if (!extract) throw new Error('extractJD is required')

    const result = extract(dom.window.document, dom.window.location.href)
    expect(result).not.toBeNull()
    if (!result) return

    expect(result.title).toBe(expected.title)
    expect(result.company).toBe(expected.company)
    expect(result.location).toBe(expected.location)
    expect(result.url).toContain(expected.url_suffix)
    expect(result.description).not.toBeNull()
    expect(result.description!.length).toBeGreaterThan(100)
    expect(result.source_plugin).toBe('linkedin')
    expect(result.extracted_at).toMatch(/^\d{4}-\d{2}-\d{2}T/)
  })

  test('returns null on empty document', () => {
    const dom = new JSDOM('<html><body></body></html>', {
      url: 'https://www.linkedin.com/jobs/view/0',
    })
    const extract = linkedinPlugin.capabilities.extractJD!
    expect(extract(dom.window.document, dom.window.location.href)).toBeNull()
  })
})

describe('linkedinPlugin metadata', () => {
  test('has correct name and matches', () => {
    expect(linkedinPlugin.name).toBe('linkedin')
    expect(linkedinPlugin.matches).toContain('linkedin.com')
    expect(linkedinPlugin.matches).toContain('*.linkedin.com')
  })
})
```

- [ ] **Step 6.2: Run the test to confirm it fails**

```bash
bun test tests/plugins/linkedin.test.ts
```

Expected: import error (`linkedinPlugin` not defined).

---

## Task 7: LinkedIn Plugin — Implementation

**Files:**
- Create: `packages/extension/src/plugin/plugins/linkedin.ts`

- [ ] **Step 7.1: Write a skeleton implementation that will fail the assertions**

Open the fixture HTML in a browser (`open packages/extension/tests/fixtures/linkedin/job-detail-standard.html`) and inspect the DOM to find the current LinkedIn selectors for title, company, location, and description. Record them in the plugin file as comments.

```typescript
// packages/extension/src/plugin/plugins/linkedin.ts

import type { JobBoardPlugin, ExtractedJob } from '../types'

const PLUGIN_NAME = 'linkedin'

/**
 * Try multiple selectors in order; return the trimmed text content of the first match.
 * Returns null if none match.
 */
function pickText(doc: Document, selectors: string[]): string | null {
  for (const sel of selectors) {
    const el = doc.querySelector(sel)
    const text = el?.textContent?.trim()
    if (text) return text
  }
  return null
}

function extractJD(doc: Document, url: string): ExtractedJob | null {
  // Selector candidates — ordered by specificity. LinkedIn has multiple layouts
  // (public viewer, logged-in viewer, mobile). Try each until one matches.
  const titleSelectors = [
    'h1.top-card-layout__title',
    'h1.topcard__title',
    '[data-test-id="job-title"]',
    'h1.jobs-unified-top-card__job-title',
  ]
  const companySelectors = [
    'a.topcard__org-name-link',
    '[data-tracking-control-name="public_jobs_topcard-org-name"]',
    '.jobs-unified-top-card__company-name a',
    '.jobs-unified-top-card__company-name',
  ]
  const locationSelectors = [
    '.topcard__flavor--bullet',
    '.jobs-unified-top-card__bullet',
    '[data-test-id="job-location"]',
  ]
  const descriptionSelectors = [
    'div.show-more-less-html__markup',
    '.jobs-description__content',
    '.jobs-description-content__text',
    '[data-test-id="job-description"]',
  ]
  const salarySelectors = [
    '.compensation__salary-range',
    '.job-details-jobs-unified-top-card__job-insight span',
  ]

  const title = pickText(doc, titleSelectors)
  const company = pickText(doc, companySelectors)
  const location = pickText(doc, locationSelectors)
  const description = pickText(doc, descriptionSelectors)
  const salary = pickText(doc, salarySelectors)

  // If we have neither title nor description, the page isn't a job detail page
  if (!title && !description) return null

  const rawFields: Record<string, unknown> = {
    tried: { titleSelectors, companySelectors, locationSelectors, descriptionSelectors, salarySelectors },
    found: { title, company, location, description: description?.slice(0, 200), salary },
  }

  return {
    title,
    company,
    location,
    salary_range: salary,
    description,
    url,
    extracted_at: new Date().toISOString(),
    source_plugin: PLUGIN_NAME,
    raw_fields: rawFields,
  }
}

export const linkedinPlugin: JobBoardPlugin = {
  name: PLUGIN_NAME,
  matches: ['linkedin.com', '*.linkedin.com'],
  capabilities: {
    extractJD,
  },
}
```

- [ ] **Step 7.2: Run the tests**

```bash
bun test tests/plugins/linkedin.test.ts
```

Expected: one of two outcomes:
- **All pass** — the selectors happened to match the fixture. Move to Step 7.4.
- **Some fail** — the selectors don't match the captured fixture. Move to Step 7.3.

- [ ] **Step 7.3: Iterate selectors to match the fixture**

For each failing assertion:
1. Open the fixture HTML in a browser
2. Use DevTools "Copy → Copy selector" on the element with the expected value
3. Add that selector to the front of the appropriate selector list in `linkedin.ts`
4. Re-run the test

Repeat until all assertions pass.

If a selector works but captures additional whitespace or sibling text, tighten the scope (use `querySelector` on a narrower ancestor).

- [ ] **Step 7.4: Re-run to confirm green**

```bash
bun test tests/plugins/linkedin.test.ts
```

Expected: all tests PASS.

- [ ] **Step 7.5: Commit**

```bash
git add packages/extension/src/plugin/plugins/linkedin.ts packages/extension/tests/plugins/linkedin.test.ts
git commit -m "feat(ext): linkedin plugin extractJD with fallback selectors"
```

---

## Task 8: Capture Second Fixture (Non-Detail Page)

**Files:**
- Create: `packages/extension/tests/fixtures/linkedin/job-search-listing.html`
- Modify: `packages/extension/tests/plugins/linkedin.test.ts`

- [ ] **Step 8.1: Capture the LinkedIn job search results page**

Navigate to `https://www.linkedin.com/jobs/search/?keywords=software%20engineer` in a browser, copy the outerHTML, save to `tests/fixtures/linkedin/job-search-listing.html`. Trim the same way as Task 5.

- [ ] **Step 8.2: Add a test asserting the plugin returns null on this page**

Add to `tests/plugins/linkedin.test.ts`, inside `describe('linkedinPlugin.extractJD', ...)`:

```typescript
test('returns null on search listing page', () => {
  const html = readFileSync(join(fixtureDir, 'job-search-listing.html'), 'utf-8')
  const dom = new JSDOM(html, {
    url: 'https://www.linkedin.com/jobs/search/?keywords=software%20engineer',
  })
  const extract = linkedinPlugin.capabilities.extractJD!
  expect(extract(dom.window.document, dom.window.location.href)).toBeNull()
})
```

- [ ] **Step 8.3: Run**

```bash
bun test tests/plugins/linkedin.test.ts
```

Expected: all PASS. If the new test fails (plugin extracted something from the search page), tighten the "no title AND no description → null" check to also require that at least one of them is meaningfully long.

- [ ] **Step 8.4: Commit**

```bash
git add packages/extension/tests/fixtures/linkedin/job-search-listing.html packages/extension/tests/plugins/linkedin.test.ts
git commit -m "test(ext): linkedin plugin returns null on search listing"
```

---

## Task 9: Debug Modal

**Files:**
- Create: `packages/extension/src/content/shared/modal.ts`

- [ ] **Step 9.1: Write the modal module**

```typescript
// packages/extension/src/content/shared/modal.ts
// NOTE: Debug UI for P1. Removed in P4 when capture wires through to Forge.

import type { ExtractedJob } from '../../plugin/types'

const MODAL_ID = 'forge-ext-debug-modal'

function fieldRow(label: string, value: string | null): string {
  const display = value ?? '<em style="color:#a00">(not found)</em>'
  return `
    <div style="display:grid;grid-template-columns:140px 1fr;gap:8px;padding:8px 0;border-bottom:1px solid #333">
      <div style="font-weight:600;color:#88f">${label}</div>
      <div style="word-break:break-word">${display}</div>
    </div>
  `
}

/**
 * Mount a debug modal showing the extracted job payload.
 * Replaces any existing modal on the page.
 */
export function mountDebugModal(extracted: ExtractedJob): void {
  // Remove any existing modal first (re-capture case)
  document.getElementById(MODAL_ID)?.remove()

  const modal = document.createElement('div')
  modal.id = MODAL_ID
  modal.style.cssText = `
    position:fixed;top:20px;right:20px;width:480px;max-height:80vh;
    background:#1a1a1a;color:#eee;border:2px solid #555;border-radius:8px;
    padding:16px;z-index:2147483647;overflow:auto;
    font-family:ui-monospace,monospace;font-size:13px;line-height:1.4;
    box-shadow:0 8px 32px rgba(0,0,0,0.5);
  `

  const jsonString = JSON.stringify(extracted, null, 2)

  modal.innerHTML = `
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
      <h3 style="margin:0;font-size:14px;color:#88f">Forge: Extracted Job</h3>
      <div>
        <button id="forge-ext-copy" style="background:#335;color:#fff;border:1px solid #557;padding:4px 8px;border-radius:4px;cursor:pointer;margin-right:4px">Copy JSON</button>
        <button id="forge-ext-close" style="background:#533;color:#fff;border:1px solid #755;padding:4px 8px;border-radius:4px;cursor:pointer">Close</button>
      </div>
    </div>
    <div>
      ${fieldRow('Title', extracted.title)}
      ${fieldRow('Company', extracted.company)}
      ${fieldRow('Location', extracted.location)}
      ${fieldRow('Salary', extracted.salary_range)}
      ${fieldRow('URL', extracted.url)}
      ${fieldRow('Source', extracted.source_plugin)}
      ${fieldRow('Extracted At', extracted.extracted_at)}
    </div>
    <div style="margin-top:12px">
      <div style="font-weight:600;color:#88f;margin-bottom:4px">Description (preview)</div>
      <div style="max-height:150px;overflow:auto;padding:8px;background:#000;border-radius:4px">${
        extracted.description
          ? extracted.description.slice(0, 1000).replace(/</g, '&lt;')
          : '<em style="color:#a00">(not found)</em>'
      }</div>
    </div>
    <div style="margin-top:12px">
      <details>
        <summary style="cursor:pointer;color:#88f;font-weight:600">Raw fields (debug)</summary>
        <pre style="background:#000;padding:8px;border-radius:4px;font-size:11px;overflow:auto;max-height:200px">${
          jsonString.replace(/</g, '&lt;')
        }</pre>
      </details>
    </div>
  `

  document.body.appendChild(modal)

  // Wire up buttons (defer to next tick so DOM is attached)
  setTimeout(() => {
    document.getElementById('forge-ext-close')?.addEventListener('click', () => {
      modal.remove()
    })
    document.getElementById('forge-ext-copy')?.addEventListener('click', () => {
      navigator.clipboard.writeText(jsonString).catch(() => {
        // Fallback not needed in dev
      })
      const btn = document.getElementById('forge-ext-copy')
      if (btn) {
        btn.textContent = 'Copied!'
        setTimeout(() => { btn.textContent = 'Copy JSON' }, 1500)
      }
    })
  }, 0)
}

/**
 * Mount a minimal "extraction failed" indicator.
 */
export function mountEmptyModal(url: string): void {
  document.getElementById(MODAL_ID)?.remove()
  const modal = document.createElement('div')
  modal.id = MODAL_ID
  modal.style.cssText = `
    position:fixed;top:20px;right:20px;width:360px;
    background:#1a1a1a;color:#eee;border:2px solid #a55;border-radius:8px;
    padding:16px;z-index:2147483647;
    font-family:ui-monospace,monospace;font-size:13px;
  `
  modal.innerHTML = `
    <h3 style="margin:0 0 8px 0;font-size:14px;color:#f88">No job extracted</h3>
    <p style="margin:0 0 8px 0">The plugin returned null — this page doesn't look like a job detail page.</p>
    <p style="margin:0 0 8px 0;font-size:11px;color:#888;word-break:break-all">${url}</p>
    <button id="forge-ext-close-empty" style="background:#533;color:#fff;border:1px solid #755;padding:4px 8px;border-radius:4px;cursor:pointer">Close</button>
  `
  document.body.appendChild(modal)
  setTimeout(() => {
    document.getElementById('forge-ext-close-empty')?.addEventListener('click', () => modal.remove())
  }, 0)
}
```

- [ ] **Step 9.2: Commit**

```bash
git add packages/extension/src/content/shared/modal.ts
git commit -m "feat(ext): debug modal for P1 extraction display (removed in P4)"
```

---

## Task 10: LinkedIn Content Script

**Files:**
- Create: `packages/extension/src/content/linkedin.ts`

- [ ] **Step 10.1: Write the content script**

```typescript
// packages/extension/src/content/linkedin.ts

import { linkedinPlugin } from '../plugin/plugins/linkedin'
import { mountDebugModal, mountEmptyModal } from './shared/modal'

interface ExtractMessage {
  cmd: 'extract'
}

type IncomingMessage = ExtractMessage  // extend in later phases

chrome.runtime.onMessage.addListener((msg: IncomingMessage, _sender, sendResponse) => {
  if (msg.cmd === 'extract') {
    try {
      const extract = linkedinPlugin.capabilities.extractJD
      if (!extract) {
        sendResponse({ ok: false, error: { code: 'PLUGIN_THREW', message: 'extractJD not defined' } })
        return
      }
      const result = extract(document, location.href)
      if (!result) {
        mountEmptyModal(location.href)
        sendResponse({ ok: false, error: { code: 'EXTRACTION_EMPTY', message: 'Plugin returned null' } })
        return
      }
      mountDebugModal(result)
      sendResponse({ ok: true, data: result })
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      sendResponse({ ok: false, error: { code: 'PLUGIN_THREW', message } })
    }
    return true  // keep channel open for async sendResponse (even though sync here)
  }
  return false
})
```

- [ ] **Step 10.2: Commit**

```bash
git add packages/extension/src/content/linkedin.ts
git commit -m "feat(ext): LinkedIn content script calls plugin and mounts modal"
```

---

## Task 11: Popup

**Files:**
- Create: `packages/extension/src/popup/index.html`
- Create: `packages/extension/src/popup/main.ts`
- Create: `packages/extension/src/popup/Popup.svelte`

- [ ] **Step 11.1: Write the popup HTML**

```html
<!-- packages/extension/src/popup/index.html -->
<!doctype html>
<html>
  <head>
    <meta charset="UTF-8" />
    <title>Forge</title>
    <style>
      html, body { margin: 0; padding: 0; width: 280px; font-family: -apple-system, system-ui, sans-serif; background: #1a1a1a; color: #eee; }
    </style>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

- [ ] **Step 11.2: Write the Svelte mount**

```typescript
// packages/extension/src/popup/main.ts

import { mount } from 'svelte'
import Popup from './Popup.svelte'

const target = document.getElementById('app')!
mount(Popup, { target })
```

- [ ] **Step 11.3: Write the Svelte component**

```svelte
<!-- packages/extension/src/popup/Popup.svelte -->
<script lang="ts">
  let status = $state<string | null>(null)
  let statusKind = $state<'info' | 'ok' | 'err'>('info')

  async function extract() {
    status = 'Extracting...'
    statusKind = 'info'
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
    if (!tab?.id) {
      status = 'No active tab'
      statusKind = 'err'
      return
    }
    try {
      const response = await chrome.tabs.sendMessage(tab.id, { cmd: 'extract' })
      if (response?.ok) {
        status = 'Extracted — see modal on page'
        statusKind = 'ok'
        setTimeout(() => window.close(), 800)
      } else if (response?.error?.code === 'EXTRACTION_EMPTY') {
        status = 'No job found on this page'
        statusKind = 'err'
      } else if (response?.error?.code === 'PLUGIN_THREW') {
        status = 'Plugin error — see console'
        statusKind = 'err'
      } else {
        status = 'Unknown response'
        statusKind = 'err'
      }
    } catch (err) {
      // Most common cause: content script not injected (wrong URL pattern)
      status = 'No plugin for this site yet'
      statusKind = 'err'
    }
  }
</script>

<main>
  <h1>Forge</h1>
  <button onclick={extract}>Extract Job</button>
  {#if status}
    <p class="status {statusKind}">{status}</p>
  {/if}
</main>

<style>
  main {
    padding: 16px;
  }
  h1 {
    margin: 0 0 12px 0;
    font-size: 16px;
    color: #88f;
  }
  button {
    width: 100%;
    background: #335;
    color: #fff;
    border: 1px solid #557;
    padding: 10px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }
  button:hover {
    background: #446;
  }
  .status {
    margin: 12px 0 0 0;
    padding: 8px;
    border-radius: 4px;
    font-size: 12px;
  }
  .status.info {
    background: #223;
    color: #aac;
  }
  .status.ok {
    background: #232;
    color: #afa;
  }
  .status.err {
    background: #322;
    color: #faa;
  }
</style>
```

- [ ] **Step 11.4: Commit**

```bash
git add packages/extension/src/popup/
git commit -m "feat(ext): popup with Extract Job button (Svelte 5)"
```

---

## Task 12: Vite Config + Manifest

**Files:**
- Create: `packages/extension/manifest.json`
- Create: `packages/extension/vite.config.ts`

- [ ] **Step 12.1: Write the manifest**

```json
{
  "manifest_version": 3,
  "name": "Forge",
  "version": "0.0.1",
  "description": "Capture job descriptions and autofill applications into Forge.",
  "action": {
    "default_popup": "src/popup/index.html",
    "default_title": "Forge"
  },
  "permissions": ["activeTab", "clipboardWrite"],
  "content_scripts": [
    {
      "matches": ["*://*.linkedin.com/*"],
      "js": ["src/content/linkedin.ts"],
      "run_at": "document_idle"
    }
  ]
}
```

- [ ] **Step 12.2: Write the vite config**

```typescript
// packages/extension/vite.config.ts

import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { crx } from '@crxjs/vite-plugin'
import manifest from './manifest.json' with { type: 'json' }

export default defineConfig({
  plugins: [
    svelte(),
    crx({ manifest }),
  ],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})
```

- [ ] **Step 12.3: Build the extension**

From `packages/extension/`:

```bash
bun run build
```

Expected: vite builds successfully, outputs to `dist/`. Check that `dist/manifest.json`, `dist/src/popup/index.html`, `dist/src/content/linkedin.js` (or similar hashed names) all exist.

If the build fails, the most likely causes are:
1. `@crxjs/vite-plugin` version mismatch — check npm for the latest beta, update package.json
2. Svelte 5 + plugin version mismatch — check peer dep warnings from `bun install`
3. TypeScript errors in source — fix and retry

- [ ] **Step 12.4: Commit**

```bash
git add packages/extension/manifest.json packages/extension/vite.config.ts
git commit -m "build(ext): vite + crxjs + svelte config, MV3 manifest"
```

---

## Task 13: Manual Load & Verify Against Real LinkedIn

- [ ] **Step 13.1: Load the extension in Chrome**

1. Open `chrome://extensions` in Chrome
2. Enable "Developer mode" toggle (top right)
3. Click "Load unpacked"
4. Select `packages/extension/dist/` (the build output directory)
5. Extension appears in the list with ID `<some-id>` — note this ID for later phases
6. Pin the extension (puzzle icon → pin Forge) for easy access

- [ ] **Step 13.2: Verify on 5 different LinkedIn job pages**

For each of 5 different `linkedin.com/jobs/view/*` URLs:

1. Navigate to the URL
2. Wait for the page to fully load
3. Click the Forge extension icon
4. Click "Extract Job"
5. Verify:
   - Modal appears in the top-right of the page
   - Title field is correct (compare to the page heading)
   - Company field is correct
   - Location field is correct or `(not found)` if page doesn't show it
   - Description preview is not empty and matches the page text
   - Copy JSON button copies valid JSON to clipboard (paste into a text editor to verify)
   - Close button removes the modal
6. Record the URL and which fields were extracted correctly

If any field is wrong or consistently `(not found)` when it should have a value:
- Add the page as a new fixture: `tests/fixtures/linkedin/job-detail-<descriptor>.html`
- Add a test case for that fixture
- Update `linkedin.ts` selectors to match
- Re-run `bun test tests/plugins/linkedin.test.ts`
- Rebuild with `bun run build`
- Reload the extension in `chrome://extensions`
- Retry the page

- [ ] **Step 13.3: Verify the non-LinkedIn case**

1. Navigate to `https://github.com`
2. Click the Forge extension icon
3. Click "Extract Job"
4. Verify the popup shows "No plugin for this site yet" (because the content script isn't injected on github.com)

- [ ] **Step 13.4: Verify the search-listing case**

1. Navigate to `https://www.linkedin.com/jobs/search/?keywords=engineer`
2. Click Extract Job
3. Verify the **empty modal** appears (red border, "No job extracted") — the content script ran but the plugin returned null.

- [ ] **Step 13.5: Record results in a verification log**

Create `packages/extension/docs/p1-verification.md`:

```markdown
# P1 Verification Results

Captured: YYYY-MM-DD
Extension ID: <the-id>
Build: <git sha>

## Job detail pages tested

| URL | Title ✓ | Company ✓ | Location ✓ | Description ✓ | Salary ✓ | Notes |
|-----|---------|-----------|------------|---------------|----------|-------|
| linkedin.com/jobs/view/1 | ✓ | ✓ | ✓ | ✓ | n/a | |
| linkedin.com/jobs/view/2 | ✓ | ✓ | ✓ | ✓ | ✓ | |
| ... | | | | | | |

## Non-LinkedIn case
- github.com → popup shows "No plugin for this site yet" ✓

## Search listing case
- /jobs/search → empty modal shown ✓

## Copy JSON verified
- Pasted into text editor, valid JSON ✓
```

---

## Task 14: Update Root Docs and Commit Verification

**Files:**
- Create: `packages/extension/README.md`
- Create: `packages/extension/docs/p1-verification.md` (from Task 13)

- [ ] **Step 14.1: Write the README**

```markdown
# @forge/extension

Browser extension for Forge. Capture job descriptions and autofill applications
into the Forge resume builder.

## Status

**Prototype P1** — pure extraction proof. Works on LinkedIn job pages, displays
extracted data in an in-page debug modal. No backend integration yet.

See `SPEC.md` in `.claude/plans/forge-resume-browser-extension/` for full design.

## Development

```bash
bun install          # from monorepo root
cd packages/extension
bun test             # run unit tests
bun run build        # build to dist/
bun run dev          # build + watch
```

Then load `packages/extension/dist/` as an unpacked extension in
`chrome://extensions` (enable Developer mode first).

## Supported sites

| Site | Extract JD | Extract Org | Autofill |
|------|-----------|-------------|----------|
| LinkedIn | ✓ (P1) | — | — |

## Architecture

See `SPEC.md` Section 2 (Plugin Contract) and Section 3 (Project Layout).
```

- [ ] **Step 14.2: Commit the verification log and README**

```bash
git add packages/extension/README.md packages/extension/docs/p1-verification.md
git commit -m "$(cat <<'EOF'
docs(ext): P1 verification results and README

Verified extraction against N real LinkedIn job pages. All fields extracted
correctly on pages tested. Empty modal correctly shown for search listings.
Non-LinkedIn sites correctly show "no plugin" message.

Verification checklist (SPEC Section 11):
- [x] Click extension on 5+ LinkedIn job pages — modal shows correct fields
- [x] Missing fields render as "(not found)"
- [x] Plugin interface shape validated by real markup
- [x] Copy JSON button copies valid JSON to clipboard
- [x] Non-LinkedIn site shows NO_PLUGIN_FOR_HOST toast
- [x] Search listing page shows EXTRACTION_EMPTY modal
- [x] All unit tests pass: bun test plugins/linkedin

Blocks unblocked: P4 (extraction -> API write)

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 15: Merge Back to Main

**When to do this:** After user review.

- [ ] **Step 15.1: Switch to main and merge**

```bash
cd /Users/adam/notes/job-hunting
git merge --no-ff feat/forge-ext/p1-extraction -m "Merge P1 (pure extraction) prototype"
```

- [ ] **Step 15.2: Run tests from main**

```bash
cd packages/extension
bun test
```

Expected: all tests pass.

- [ ] **Step 15.3: Remove the worktree**

```bash
cd /Users/adam/notes/job-hunting
git worktree remove .claude/worktrees/forge-ext-p1-extraction
```

---

## Done When

- [ ] `packages/extension/` workspace package exists and builds
- [ ] Plugin contract types defined in `src/plugin/types.ts`
- [ ] Plugin registry with wildcard hostname matching, tested
- [ ] Error taxonomy in `src/lib/errors.ts`
- [ ] LinkedIn plugin extracts title/company/location/description/salary from fixture, tested
- [ ] LinkedIn plugin returns null on non-detail pages, tested
- [ ] Debug modal displays extracted data with Copy JSON and Close
- [ ] Popup with Extract Job button triggers content script via chrome.tabs.sendMessage
- [ ] **Extension compiles cleanly: `bun run build` succeeds with no errors**
- [ ] **Extension is dev-installable: user can load `packages/extension/dist/` as an unpacked extension in Chrome without errors**
- [ ] Extension loads unpacked in Chrome and works on real LinkedIn job pages (5+ verified)
- [ ] All unit tests pass
- [ ] Verification log committed with the merge commit
- [ ] Worktree removed; P4 is unblocked

## Cross-Cutting Phase Requirement

**Every phase of the forge-resume-browser-extension project must produce a buildable, dev-installable extension.**

At the end of each phase, the user must be able to:
1. `cd packages/extension && bun run build` → succeed with no errors
2. Open `chrome://extensions`, enable Developer mode, click "Load unpacked", select `packages/extension/dist/`
3. See the extension loaded with no manifest errors, no service worker errors, no content script errors
4. Click the extension icon on a relevant page and see the phase's functionality work

This is a **non-negotiable** gate. A phase is not complete until the user has personally built and installed it.
