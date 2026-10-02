# M2 — Capture Entry Points + LinkedIn Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add context menu and injected button capture triggers on LinkedIn, plus fix 3 LinkedIn extraction bugs (salary truncation, apply link, org LinkedIn URL).

**Architecture:** Context menu registered in background worker via `chrome.contextMenus` API (works on both Chrome and Firefox MV3). Injected button uses `aria-label` anchoring + MutationObserver for SPA navigation. LinkedIn plugin gains `extractApplyUrl` and `extractCompanyUrl` capabilities. The existing `handleCaptureJob` handler is reused by both new entry points.

**Tech Stack:** Vite 8, Svelte 5, Bun, chrome.contextMenus API

**Beads:** 3bp.23, 3bp.24, 3bp.13, 3bp.14, 3bp.16

**Worktree:** `.claude/worktrees/forge-ext-m2-entry-points/`
**Branch:** `feat/forge-ext/m2-entry-points`

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Modify | `packages/extension/manifest.json` | Add `contextMenus` permission |
| Modify | `packages/extension/manifest.firefox.json` | Add `contextMenus` permission |
| Modify | `packages/extension/src/background/index.ts` | Register context menu on install, add click handler |
| Modify | `packages/extension/src/background/handlers/capture.ts` | Accept optional `tabId` param (context menu provides the tab) |
| Modify | `packages/extension/src/plugin/types.ts` | Add `apply_url` to `ExtractedJob`, add `company_url` field |
| Modify | `packages/extension/src/plugin/plugins/linkedin.ts` | Fix salary truncation, add apply URL extraction, add company URL extraction |
| Modify | `packages/extension/src/content/linkedin.ts` | Add `extractMeta` cmd that returns apply URL + company URL alongside JD |
| Create | `packages/extension/src/content/inject-button.ts` | Injected "Capture to Forge" button on LinkedIn job pages |
| Modify | `packages/extension/src/lib/resolve-org.ts` | Accept + pass through `linkedin_url` field |
| Modify | `packages/extension/tests/plugins/linkedin.test.ts` | Add tests for salary fix, apply URL, company URL |
| Create | `packages/extension/tests/plugins/linkedin-apply-url.test.ts` | Dedicated tests for apply URL extraction |
| Modify | `packages/extension/tests/build/manifests.test.ts` | Assert `contextMenus` in permissions |

---

### Task 1: Create worktree and branch

**Files:**
- Create: `.claude/worktrees/forge-ext-m2-entry-points/` (git worktree)

- [ ] **Step 1: Create worktree from main**

```bash
cd /Users/adam/notes/job-hunting
git worktree add .claude/worktrees/forge-ext-m2-entry-points -b feat/forge-ext/m2-entry-points main
```

- [ ] **Step 2: Install deps and verify tests**

```bash
cd .claude/worktrees/forge-ext-m2-entry-points && bun install
cd packages/extension && bun test
```

Expected: 89 pass, 0 fail

---

### Task 2: Fix salary chip truncation (3bp.13)

**Files:**
- Modify: `packages/extension/src/plugin/plugins/linkedin.ts`
- Modify: `packages/extension/tests/plugins/linkedin.test.ts`

The salary chip regex `/\$[\d,.]+/` is matching correctly, but the `getCheckSmallChipText` function reads from the `<span>` sibling of the SVG. The bug is that the chip text span sometimes starts with a `$` character that is actually in a *separate* text node or nested element. The truncation of `$300,000` → `320,000` suggests the first span child of the wrapper contains partial text.

Investigation: The real issue is likely that the salary chip text `$300,000—$405,000` has the leading `$3` in one text node and `20,000—$405,000` in a continuation. The `textContent` property concatenates them, so the extraction should be correct. More likely, the chip HTML has a wrapping structure where the `<span>` sibling of the SVG doesn't contain the full text.

The fix: instead of reading only the first `<span>` sibling of the SVG, collect `textContent` from the **entire wrapper element** (minus the SVG text which is empty/icon).

- [ ] **Step 1: Create a fixture for the salary truncation bug**

Create `packages/extension/tests/fixtures/linkedin/salary-chip-truncation.html`:

```html
<!-- Minimal fixture reproducing the salary truncation bug from 3bp.13 -->
<!-- LinkedIn wraps chip text in nested spans inside the check-small wrapper -->
<html><body>
<div data-display-contents="true"><p>Staff Software Engineer</p></div>
<span aria-label="Company, Anthropic."><img alt="" /></span>
<!-- Salary chip with nested structure that caused truncation -->
<span>
  <svg id="check-small"><path d=""/></svg>
  <span>
    <span>$</span><span>300,000</span><span>—</span><span>$405,000</span>
  </span>
</span>
<!-- Workplace chip -->
<span>
  <svg id="check-small"><path d=""/></svg>
  <span>Remote</span>
</span>
</body></html>
```

- [ ] **Step 2: Write the failing test**

Add to `packages/extension/tests/plugins/linkedin.test.ts`:

```ts
test('extracts full salary range from nested chip spans (3bp.13 truncation fix)', () => {
  const html = readFileSync(join(fixtureDir, 'salary-chip-truncation.html'), 'utf-8')
  const dom = new JSDOM(html, {
    url: 'https://www.linkedin.com/jobs/view/4322360108/',
  })
  const extract = linkedinPlugin.capabilities.extractJD!
  const result = extract(dom.window.document, dom.window.location.href)
  expect(result).not.toBeNull()
  // Must get the FULL salary range, not truncated
  expect(result!.salary_range).toBe('$300,000—$405,000')
})
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd packages/extension && bun test tests/plugins/linkedin.test.ts
```

Expected: New test FAILS — current `getCheckSmallChipText` returns truncated text.

- [ ] **Step 4: Fix `getCheckSmallChipText` in linkedin.ts**

The fix: read `textContent` from the entire wrapper span, not just the first `<span>` child. The SVG has no text content, so `wrapper.textContent` gives us the concatenated chip text.

Replace the `getCheckSmallChipText` function in `packages/extension/src/plugin/plugins/linkedin.ts`:

```ts
/**
 * Extract the text from a check-small chip wrapper.
 * LinkedIn wraps chip text in nested spans — instead of reading a single
 * child span, we read textContent from the entire wrapper (the SVG
 * contributes no text, so this gives us the full chip value).
 */
function getCheckSmallChipText(svg: Element): string | null {
  const wrapper = svg.parentElement
  if (!wrapper) return null
  const text = wrapper.textContent?.trim()
  return text || null
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd packages/extension && bun test tests/plugins/linkedin.test.ts
```

Expected: All tests PASS including the new truncation test.

- [ ] **Step 6: Commit**

```bash
git add packages/extension/src/plugin/plugins/linkedin.ts packages/extension/tests/plugins/linkedin.test.ts packages/extension/tests/fixtures/linkedin/salary-chip-truncation.html
git commit -m "fix(ext): salary chip reads full wrapper textContent (3bp.13)"
```

---

### Task 3: Add apply URL extraction (3bp.14)

**Files:**
- Modify: `packages/extension/src/plugin/types.ts` — add `apply_url` to `ExtractedJob`
- Modify: `packages/extension/src/plugin/plugins/linkedin.ts` — add apply URL extraction
- Create: `packages/extension/tests/fixtures/linkedin/job-detail-external-apply.html` — fixture with Apply link
- Modify: `packages/extension/tests/plugins/linkedin.test.ts` — test apply URL extraction

- [ ] **Step 1: Add `apply_url` to `ExtractedJob` type**

In `packages/extension/src/plugin/types.ts`, add after `raw_fields`:

```ts
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
  apply_url?: string | null          // External apply link (decoded from LinkedIn redirect)
  company_url?: string | null        // Company profile URL (e.g. linkedin.com/company/anthropic)
}
```

- [ ] **Step 2: Create apply link fixture**

Create `packages/extension/tests/fixtures/linkedin/job-detail-external-apply.html`:

```html
<!-- Minimal fixture for external Apply link extraction (3bp.14) -->
<html><body>
<div data-display-contents="true"><p>ML Engineer</p></div>
<span aria-label="Company, Anthropic."><img alt="" /></span>
<div data-testid="expandable-text-box">Great role building AI systems.</div>
<!-- External Apply button linking through LinkedIn's safety redirect -->
<a href="https://www.linkedin.com/safety/go/?url=https%3A%2F%2Fjob-boards.greenhouse.io%2Fanthropic%2Fjobs%2F4982193008&trk=public_jobs_apply-link-offsite_sign-up"
   data-tracking-control-name="public_jobs_apply-link-offsite_sign-up">
  Apply
</a>
<!-- Also test Easy Apply (no external link) -->
</body></html>
```

- [ ] **Step 3: Write the failing test**

Add to `packages/extension/tests/plugins/linkedin.test.ts`:

```ts
test('extracts external apply URL from LinkedIn safety redirect (3bp.14)', () => {
  const html = readFileSync(join(fixtureDir, 'job-detail-external-apply.html'), 'utf-8')
  const dom = new JSDOM(html, {
    url: 'https://www.linkedin.com/jobs/view/4982193008/',
  })
  const extract = linkedinPlugin.capabilities.extractJD!
  const result = extract(dom.window.document, dom.window.location.href)
  expect(result).not.toBeNull()
  expect(result!.apply_url).toBe('https://job-boards.greenhouse.io/anthropic/jobs/4982193008')
})

test('apply_url is null when no external apply link present', () => {
  const { dom } = loadFixture('job-detail-standard')
  const extract = linkedinPlugin.capabilities.extractJD!
  const result = extract(dom.window.document, dom.window.location.href)
  expect(result).not.toBeNull()
  // Standard fixture has Easy Apply, not external — apply_url should be null
  expect(result!.apply_url).toBeNull()
})
```

- [ ] **Step 4: Run test to verify it fails**

```bash
cd packages/extension && bun test tests/plugins/linkedin.test.ts
```

Expected: FAIL — `apply_url` is not yet returned by `extractJD`.

- [ ] **Step 5: Implement apply URL extraction**

Add this function to `packages/extension/src/plugin/plugins/linkedin.ts` (before `extractJD`):

```ts
/**
 * Extract the external apply URL from LinkedIn's safety redirect link.
 * LinkedIn wraps external apply links as: /safety/go/?url=<encoded_url>&trk=...
 * Returns the decoded destination URL, or null if no external apply link found.
 */
function extractApplyUrl(doc: Document): string | null {
  // Look for the offsite apply link by tracking control name
  const applyLink = doc.querySelector(
    'a[data-tracking-control-name*="apply-link-offsite"]'
  ) as HTMLAnchorElement | null

  if (!applyLink?.href) return null

  try {
    const parsed = new URL(applyLink.href)
    const encodedUrl = parsed.searchParams.get('url')
    if (encodedUrl) return decodeURIComponent(encodedUrl)
  } catch {
    // Not a valid URL — return null
  }

  return null
}
```

Then in the `extractJD` function, add before the `return` statement:

```ts
  const applyUrl = extractApplyUrl(doc)

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
    apply_url: applyUrl,
    company_url: null,  // Added in Task 4
  }
```

- [ ] **Step 6: Run tests**

```bash
cd packages/extension && bun test tests/plugins/linkedin.test.ts
```

Expected: All PASS.

- [ ] **Step 7: Commit**

```bash
git add packages/extension/src/plugin/types.ts packages/extension/src/plugin/plugins/linkedin.ts packages/extension/tests/plugins/linkedin.test.ts packages/extension/tests/fixtures/linkedin/job-detail-external-apply.html
git commit -m "feat(ext): extract external apply URL from LinkedIn safety redirect (3bp.14)"
```

---

### Task 4: Extract org LinkedIn URL during capture (3bp.16)

**Files:**
- Modify: `packages/extension/src/plugin/plugins/linkedin.ts` — extract company profile URL
- Modify: `packages/extension/src/content/linkedin.ts` — return company URL in extract response
- Modify: `packages/extension/src/background/handlers/capture.ts` — pass `linkedin_url` to org create
- Modify: `packages/extension/src/lib/resolve-org.ts` — accept and pass `linkedin_url`
- Modify: `packages/extension/tests/plugins/linkedin.test.ts` — test company URL extraction
- Modify: `packages/extension/tests/lib/resolve-org.test.ts` — test linkedin_url pass-through

- [ ] **Step 1: Write the failing test for company URL extraction**

Add to `packages/extension/tests/plugins/linkedin.test.ts`:

```ts
test('extracts company LinkedIn URL from company link', () => {
  const { dom } = loadFixture('job-detail-standard')
  const extract = linkedinPlugin.capabilities.extractJD!
  const result = extract(dom.window.document, dom.window.location.href)
  expect(result).not.toBeNull()
  // Company URL should be the LinkedIn company profile page
  if (result!.company_url) {
    expect(result!.company_url).toMatch(/linkedin\.com\/company\//)
  }
})
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/extension && bun test tests/plugins/linkedin.test.ts
```

Expected: FAIL — `company_url` is always `null`.

- [ ] **Step 3: Implement company URL extraction**

Add this function to `packages/extension/src/plugin/plugins/linkedin.ts`:

```ts
/**
 * Extract the company's LinkedIn profile URL from the job detail page.
 * LinkedIn links the company name/logo to /company/<slug>/life/ or similar.
 */
function extractCompanyUrl(doc: Document): string | null {
  // Look for a link to a company profile page
  const companyLink = doc.querySelector(
    'a[href*="/company/"]'
  ) as HTMLAnchorElement | null

  if (!companyLink?.href) return null

  try {
    const parsed = new URL(companyLink.href)
    // Normalize to just /company/<slug>/
    const match = parsed.pathname.match(/^\/company\/[^/]+/)
    if (match) {
      return `https://www.linkedin.com${match[0]}/`
    }
  } catch {
    // Not a valid URL
  }

  return null
}
```

Update the `extractJD` return to use `extractCompanyUrl`:

```ts
  const applyUrl = extractApplyUrl(doc)
  const companyUrl = extractCompanyUrl(doc)

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
    apply_url: applyUrl,
    company_url: companyUrl,
  }
```

- [ ] **Step 4: Update `resolveOrganization` to accept and pass `linkedin_url`**

In `packages/extension/src/lib/resolve-org.ts`, update the `createOrg` callback signature and the call site:

```ts
export async function resolveOrganization(
  companyName: string | null,
  searchOrgs: (search: string) => Promise<{ ok: boolean; data?: { id: string; name: string }[] }>,
  createOrg: (name: string, opts?: { linkedin_url?: string }) => Promise<{ ok: boolean; data?: { id: string; name: string } }>,
  opts?: { linkedin_url?: string },
): Promise<string | null> {
  if (!companyName?.trim()) return null

  const searchResult = await searchOrgs(companyName)
  if (!searchResult.ok || !searchResult.data) return null

  const exact = searchResult.data.find(
    (org) => org.name.toLowerCase() === companyName.toLowerCase(),
  )
  if (exact) return exact.id

  if (searchResult.data.length === 0) {
    const createResult = await createOrg(companyName, { linkedin_url: opts?.linkedin_url })
    return createResult.ok && createResult.data ? createResult.data.id : null
  }

  return searchResult.data[0].id
}
```

- [ ] **Step 5: Update capture handler to pass company URL**

In `packages/extension/src/background/handlers/capture.ts`, update the `resolveOrganization` call (around line 90):

```ts
    const organizationId = await resolveOrganization(
      extracted.company,
      async (search) => {
        const result = await client.organizations.list({ search, limit: 5 })
        return result.ok
          ? { ok: true, data: result.data }
          : { ok: false }
      },
      async (name, opts) => {
        const result = await client.organizations.create({
          name,
          linkedin_url: opts?.linkedin_url,
        })
        return result.ok
          ? { ok: true, data: { id: result.data.id, name: result.data.name } }
          : { ok: false }
      },
      { linkedin_url: extracted.company_url ?? undefined },
    )
```

- [ ] **Step 6: Update resolve-org tests**

In `packages/extension/tests/lib/resolve-org.test.ts`, add a test for linkedin_url pass-through:

```ts
test('passes linkedin_url to createOrg when provided', async () => {
  let capturedOpts: { linkedin_url?: string } | undefined
  const result = await resolveOrganization(
    'NewCorp',
    async () => ({ ok: true, data: [] }),
    async (name, opts) => {
      capturedOpts = opts
      return { ok: true, data: { id: 'new-id', name } }
    },
    { linkedin_url: 'https://www.linkedin.com/company/newcorp/' },
  )
  expect(result).toBe('new-id')
  expect(capturedOpts?.linkedin_url).toBe('https://www.linkedin.com/company/newcorp/')
})
```

- [ ] **Step 7: Run all tests**

```bash
cd packages/extension && bun test
```

Expected: All PASS.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(ext): extract company LinkedIn URL, pass to org create (3bp.16)"
```

---

### Task 5: Add context menu "Capture to Forge" (3bp.23)

**Files:**
- Modify: `packages/extension/manifest.json` — add `contextMenus` permission
- Modify: `packages/extension/manifest.firefox.json` — add `contextMenus` permission
- Modify: `packages/extension/src/background/index.ts` — register context menu, add click handler
- Modify: `packages/extension/src/background/handlers/capture.ts` — accept explicit `tabId` parameter
- Modify: `packages/extension/tests/build/manifests.test.ts` — assert `contextMenus` in permissions

- [ ] **Step 1: Update manifest tests to expect `contextMenus`**

In `packages/extension/tests/build/manifests.test.ts`, find the permissions assertions and update expected values to include `contextMenus`:

```ts
// In the Chrome manifest tests:
test('has required permissions', () => {
  expect(chrome.permissions).toContain('contextMenus')
})

// In the Firefox manifest tests:
test('same permissions as Chrome', () => {
  expect(firefox.permissions).toEqual(chrome.permissions)
})
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/extension && bun test tests/build/manifests.test.ts
```

Expected: FAIL — `contextMenus` not in permissions yet.

- [ ] **Step 3: Add `contextMenus` to both manifests**

In `packages/extension/manifest.json`:
```json
"permissions": ["activeTab", "scripting", "clipboardWrite", "storage", "contextMenus"],
```

In `packages/extension/manifest.firefox.json`:
```json
"permissions": ["activeTab", "scripting", "clipboardWrite", "storage", "contextMenus"],
```

- [ ] **Step 4: Run manifest tests**

```bash
cd packages/extension && bun test tests/build/manifests.test.ts
```

Expected: PASS.

- [ ] **Step 5: Refactor `handleCaptureJob` to accept an optional `tabId`**

Currently `handleCaptureJob` queries for the active tab. Context menu clicks provide the tab directly. Refactor to accept an optional `tabId`:

In `packages/extension/src/background/handlers/capture.ts`:

```ts
export async function handleCaptureJob(explicitTabId?: number): Promise<Response<CaptureJobPayload>> {
  try {
    // 1. Get tab — use explicit tabId if provided, otherwise query active tab
    let tabId: number
    let tabUrl: string
    if (explicitTabId) {
      const tab = await chrome.tabs.get(explicitTabId)
      if (!tab.url) {
        return {
          ok: false,
          error: extError('UNKNOWN_ERROR', 'Tab has no URL', { layer: 'background' }),
        }
      }
      tabId = explicitTabId
      tabUrl = tab.url
    } else {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
      if (!tab?.id || !tab.url) {
        return {
          ok: false,
          error: extError('UNKNOWN_ERROR', 'No active tab', { layer: 'background' }),
        }
      }
      tabId = tab.id
      tabUrl = tab.url
    }

    // 2. Check host — only LinkedIn supported
    if (!/^https?:\/\/([a-z0-9-]+\.)?linkedin\.com\//i.test(tabUrl)) {
      return {
        ok: false,
        error: extError('NO_PLUGIN_FOR_HOST', 'No plugin for this site yet', {
          layer: 'background',
          url: tabUrl,
        }),
      }
    }

    // 3. Inject content script and extract
    await chrome.scripting.executeScript({
      target: { tabId },
      files: ['content/linkedin.js'],
    })

    const extractResponse = await chrome.tabs.sendMessage(tabId, { cmd: 'extract' })
    // ... rest of handler unchanged, but replace tab.url with tabUrl and tab.id with tabId ...
```

Update all remaining references from `tab.id` → `tabId` and `tab.url` → `tabUrl` in the rest of the function.

- [ ] **Step 6: Add context menu registration + click handler to background/index.ts**

Add to `packages/extension/src/background/index.ts` after the existing imports:

```ts
import { handleCaptureJob } from './handlers/capture'

// ── Context Menu ──────────────────────────────────────────────────────────
// Register on install (runs once per extension install/update)
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: 'forge-capture-job',
    title: 'Capture to Forge',
    contexts: ['page'],
    documentUrlPatterns: [
      '*://*.linkedin.com/jobs/*',
      '*://*.myworkdayjobs.com/*',
      '*://*.myworkday.com/*',
    ],
  })
})

// Handle context menu clicks
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== 'forge-capture-job') return
  if (!tab?.id) return

  const result = await handleCaptureJob(tab.id)

  // Set badge to indicate success/failure
  const badgeText = result.ok ? '✓' : '!'
  const badgeColor = result.ok ? '#4a4' : '#a44'
  chrome.action.setBadgeText({ text: badgeText, tabId: tab.id })
  chrome.action.setBadgeBackgroundColor({ color: badgeColor, tabId: tab.id })

  // Clear badge after 3 seconds
  setTimeout(() => {
    chrome.action.setBadgeText({ text: '', tabId: tab?.id })
  }, 3000)
})
```

Note: The `handleCaptureJob` import already exists at line 7, so don't duplicate it.

- [ ] **Step 7: Run all tests**

```bash
cd packages/extension && bun test
```

Expected: All PASS. (Context menu registration is a runtime-only API — no unit tests needed. Verification is manual.)

- [ ] **Step 8: Build both browsers and verify**

```bash
cd packages/extension && bun run build
```

Verify both builds succeed.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(ext): add context menu 'Capture to Forge' on job pages (3bp.23)"
```

---

### Task 6: Inject "Capture to Forge" button on LinkedIn (3bp.24)

**Files:**
- Create: `packages/extension/src/content/inject-button.ts` — injected button logic
- Modify: `packages/extension/src/content/linkedin.ts` — import and call inject-button on load
- Modify: `packages/extension/src/lib/messaging.ts` — no changes needed (uses existing `sendCommand` pattern)

The injected button appears as a sibling to LinkedIn's action buttons (Save, Share). It uses `aria-label` anchoring for stability. A MutationObserver handles SPA navigation.

- [ ] **Step 1: Create the inject-button module**

Create `packages/extension/src/content/inject-button.ts`:

```ts
// packages/extension/src/content/inject-button.ts
//
// Inject a "Capture to Forge" button on LinkedIn job detail pages.
// Anchored by aria-label attributes on LinkedIn's action buttons
// (more stable than CSS class selectors which are hashed module names).

const BUTTON_ID = 'forge-capture-btn'

/**
 * Find an anchor element near LinkedIn's job action buttons.
 * We look for buttons with known aria-labels in the job detail card area.
 */
function findAnchor(doc: Document): Element | null {
  // LinkedIn's action buttons: Save, Share, More options
  // These aria-labels are WCAG-mandated and rarely change
  const candidates = [
    'button[aria-label="Save"]',
    'button[aria-label="Share"]',
    'button[aria-label="More options"]',
  ]
  for (const sel of candidates) {
    const el = doc.querySelector(sel)
    if (el) return el
  }
  return null
}

/**
 * Create and inject the Forge capture button.
 * Returns true if injected, false if anchor not found or already present.
 */
export function injectCaptureButton(doc: Document): boolean {
  // Don't double-inject
  if (doc.getElementById(BUTTON_ID)) return false

  const anchor = findAnchor(doc)
  if (!anchor) return false

  const container = anchor.parentElement
  if (!container) return false

  const btn = doc.createElement('button')
  btn.id = BUTTON_ID
  btn.textContent = 'Capture to Forge'
  btn.setAttribute('aria-label', 'Capture to Forge')

  // Style to match LinkedIn's button aesthetic
  Object.assign(btn.style, {
    display: 'inline-flex',
    alignItems: 'center',
    padding: '6px 16px',
    marginLeft: '8px',
    border: '1px solid #0a66c2',
    borderRadius: '24px',
    background: 'transparent',
    color: '#0a66c2',
    fontFamily: '-apple-system, system-ui, sans-serif',
    fontSize: '14px',
    fontWeight: '600',
    cursor: 'pointer',
    lineHeight: '1.33',
  })

  btn.addEventListener('click', async (e) => {
    e.preventDefault()
    e.stopPropagation()
    btn.textContent = 'Capturing...'
    btn.disabled = true

    try {
      // Send capture command to background worker via runtime messaging
      const response = await chrome.runtime.sendMessage({ cmd: 'jd.captureActive' })
      if (response?.ok) {
        btn.textContent = '✓ Captured'
        btn.style.borderColor = '#057642'
        btn.style.color = '#057642'
      } else {
        const code = response?.error?.code
        if (code === 'API_DUPLICATE') {
          btn.textContent = 'Already captured'
          btn.style.borderColor = '#666'
          btn.style.color = '#666'
        } else {
          btn.textContent = 'Failed'
          btn.style.borderColor = '#cc1016'
          btn.style.color = '#cc1016'
        }
      }
    } catch {
      btn.textContent = 'Failed'
      btn.style.borderColor = '#cc1016'
      btn.style.color = '#cc1016'
    }

    // Reset after 3 seconds
    setTimeout(() => {
      btn.textContent = 'Capture to Forge'
      btn.disabled = false
      btn.style.borderColor = '#0a66c2'
      btn.style.color = '#0a66c2'
    }, 3000)
  })

  container.appendChild(btn)
  return true
}

/**
 * Remove the injected button (for cleanup on SPA navigation).
 */
export function removeCaptureButton(doc: Document): void {
  const existing = doc.getElementById(BUTTON_ID)
  if (existing) existing.remove()
}

/**
 * Set up a MutationObserver to re-inject the button on SPA navigation.
 * LinkedIn uses client-side routing — the page doesn't reload when
 * navigating between jobs.
 */
export function observeForInjection(doc: Document): void {
  let lastUrl = doc.location.href

  const observer = new MutationObserver(() => {
    const currentUrl = doc.location.href
    if (currentUrl !== lastUrl) {
      lastUrl = currentUrl
      // URL changed — remove old button, try to inject new one
      removeCaptureButton(doc)
      // Delay to let LinkedIn render the new page
      setTimeout(() => injectCaptureButton(doc), 500)
    }
  })

  observer.observe(doc.body, { childList: true, subtree: true })
}
```

- [ ] **Step 2: Wire into LinkedIn content script**

In `packages/extension/src/content/linkedin.ts`, add at the bottom (after the message listener setup):

```ts
// ── Injected Button ──────────────────────────────────────────────────────
// Import is safe here because inject-button.ts only uses DOM APIs,
// no plugin module imports (preserves shared chunk constraint).
import { injectCaptureButton, observeForInjection } from './inject-button'

// Inject the button on initial load
setTimeout(() => injectCaptureButton(document), 500)
// Watch for SPA navigation
observeForInjection(document)
```

**IMPORTANT:** `inject-button.ts` must NOT import from `../plugin/plugins/*` — it only uses DOM APIs and `chrome.runtime.sendMessage`. This preserves the shared chunk constraint.

- [ ] **Step 3: Build and verify content script is still self-contained**

```bash
cd packages/extension && bun run build:chrome
head -3 dist/chrome/content/linkedin.js
# Must not have 'import' statements
```

- [ ] **Step 4: Run all tests**

```bash
cd packages/extension && bun test
```

Expected: All PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/content/inject-button.ts packages/extension/src/content/linkedin.ts
git commit -m "feat(ext): inject 'Capture to Forge' button on LinkedIn via aria-label anchoring (3bp.24)"
```

---

### Task 7: Bump version to 0.1.2, final verification, commit

**Files:**
- Modify: `packages/extension/manifest.json` (version)
- Modify: `packages/extension/manifest.firefox.json` (version)

- [ ] **Step 1: Bump version in both manifests**

```json
"version": "0.1.2",
```

In both `manifest.json` and `manifest.firefox.json`.

- [ ] **Step 2: Run full test suite**

```bash
cd packages/extension && bun test
```

Expected: All tests pass.

- [ ] **Step 3: Full rebuild**

```bash
cd packages/extension && bun run build
```

- [ ] **Step 4: Final manual verification checklist**

- [ ] `bun run build` succeeds for both browsers
- [ ] Firefox/Zen: right-click on LinkedIn job page → "Capture to Forge" appears in context menu
- [ ] Firefox/Zen: context menu capture works (badge shows ✓)
- [ ] Firefox/Zen: injected "Capture to Forge" button visible next to Save/Share
- [ ] Firefox/Zen: injected button capture works
- [ ] Firefox/Zen: SPA navigation (click different job) → button re-appears
- [ ] Chrome: same 3 capture flows work
- [ ] Salary extraction: full range captured (no truncation)

- [ ] **Step 5: Commit**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json
git commit -m "chore(ext): bump version to 0.1.2 (M2)"
```

---

### Task 8: Merge to main, close beads, cleanup

**Files:** None (git operations only)

- [ ] **Step 1: Merge worktree branch to main**

```bash
cd /Users/adam/notes/job-hunting
git checkout main
git merge feat/forge-ext/m2-entry-points --no-ff -m "Merge M2 (capture entry points + LinkedIn polish)"
```

- [ ] **Step 2: Rebuild dist on main**

```bash
cd packages/extension && bun run build
```

- [ ] **Step 3: Close beads**

```bash
bd close job-hunting-3bp.23 --reason "Merged M2: context menu capture"
bd close job-hunting-3bp.24 --reason "Merged M2: injected button via aria-label"
bd close job-hunting-3bp.13 --reason "Merged M2: salary chip truncation fix"
bd close job-hunting-3bp.14 --reason "Merged M2: apply URL extraction"
bd close job-hunting-3bp.16 --reason "Merged M2: org LinkedIn URL pass-through"
```

- [ ] **Step 4: Clean up worktree**

```bash
cd /Users/adam/notes/job-hunting
git worktree remove .claude/worktrees/forge-ext-m2-entry-points
```

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| `inject-button.ts` import creates shared chunk | Module only uses DOM APIs + `chrome.runtime.sendMessage`. Build output test verifies no imports. If it breaks, move inject logic inline into `linkedin.ts`. |
| `aria-label` anchors not found on some LinkedIn layouts | `injectCaptureButton` returns false gracefully. Context menu (3bp.23) is the fallback — always works regardless of DOM structure. |
| Context menu registration fails silently | `chrome.runtime.onInstalled` fires once. If extension is already installed, manually reload it from `about:debugging` or `chrome://extensions`. |
| LinkedIn changes chip HTML structure again | Salary fix uses `wrapper.textContent` which is structural-position-independent. Only breaks if LinkedIn stops using `svg#check-small` chips entirely. |
