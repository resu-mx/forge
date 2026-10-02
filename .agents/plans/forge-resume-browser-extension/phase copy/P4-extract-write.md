# P4: Extraction to API Write — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the LinkedIn extraction flow through to Forge JD creation — full capture in one click, with validation and error feedback. Remove the P1 debug modal.

**Architecture:** Popup sends `jd.captureActive` to background worker. Background injects the LinkedIn content script, receives the extracted job, validates required fields (title + description), then creates a JD via `sdk.jobDescriptions.create()`. Content script is simplified to pure extraction (no modal side effects). Follows the same handler pattern as `handleTestFill` and `handleOrgsCreate`.

**Tech Stack:** Chrome MV3 extension, Svelte 5 popup, `@forge/sdk`, Bun test runner

**Worktree:** `.claude/worktrees/forge-ext-p4-extract-write/` on branch `feat/forge-ext/p4-extract-write`

**SPEC reference:** SPEC.md section 4 P4

---

### File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `src/lib/validate.ts` | `validateExtraction()` — checks title + description present |
| Create | `tests/lib/validate.test.ts` | Unit tests for extraction validation |
| Create | `src/background/handlers/capture.ts` | `handleCaptureJob` — orchestrates inject/extract/validate/create |
| Modify | `src/lib/messaging.ts` | Add `jd.captureActive` to Command union |
| Modify | `src/background/index.ts` | Add `jd.captureActive` case + import |
| Modify | `src/content/linkedin.ts` | Remove modal imports and calls |
| Delete | `src/content/shared/modal.ts` | P1 debug modal — no longer needed |
| Delete | `src/content/shared/` | Empty directory after modal removal |
| Modify | `src/popup/Popup.svelte` | Replace "Extract Job" with "Capture Job" |
| Modify | `tests/background/smoke.test.ts` | Add JD create smoke test |
| Modify | `manifest.json` | Version bump to 0.0.7 |

All paths relative to `packages/extension/`.

---

### Task 1: Extraction validator (TDD)

**Files:**
- Create: `packages/extension/tests/lib/validate.test.ts`
- Create: `packages/extension/src/lib/validate.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/extension/tests/lib/validate.test.ts`:

```typescript
import { describe, test, expect } from 'bun:test'
import { validateExtraction } from '../../src/lib/validate'
import type { ExtractedJob } from '../../src/plugin/types'

function makeJob(overrides: Partial<ExtractedJob> = {}): ExtractedJob {
  return {
    title: 'Software Engineer',
    company: 'Acme Corp',
    location: 'San Francisco, CA',
    salary_range: '$150k-$200k',
    description: 'We are looking for a software engineer...',
    url: 'https://www.linkedin.com/jobs/view/123',
    extracted_at: new Date().toISOString(),
    source_plugin: 'linkedin',
    ...overrides,
  }
}

describe('validateExtraction', () => {
  test('passes when title and description present', () => {
    const result = validateExtraction(makeJob())
    expect(result.valid).toBe(true)
  })

  test('fails when title is null', () => {
    const result = validateExtraction(makeJob({ title: null }))
    expect(result.valid).toBe(false)
    if (!result.valid) {
      expect(result.missing).toContain('title')
    }
  })

  test('fails when description is null', () => {
    const result = validateExtraction(makeJob({ description: null }))
    expect(result.valid).toBe(false)
    if (!result.valid) {
      expect(result.missing).toContain('description')
    }
  })

  test('fails when both title and description are null', () => {
    const result = validateExtraction(makeJob({ title: null, description: null }))
    expect(result.valid).toBe(false)
    if (!result.valid) {
      expect(result.missing).toEqual(['title', 'description'])
    }
  })

  test('passes when optional fields are null', () => {
    const result = validateExtraction(makeJob({
      company: null,
      location: null,
      salary_range: null,
    }))
    expect(result.valid).toBe(true)
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/extension && bun test tests/lib/validate.test.ts`
Expected: FAIL — module `../../src/lib/validate` not found.

- [ ] **Step 3: Write minimal implementation**

Create `packages/extension/src/lib/validate.ts`:

```typescript
import type { ExtractedJob } from '../plugin/types'

export type ValidationResult =
  | { valid: true }
  | { valid: false; missing: string[] }

/** Check that required fields (title, description) are present. */
export function validateExtraction(extracted: ExtractedJob): ValidationResult {
  const missing: string[] = []
  if (!extracted.title) missing.push('title')
  if (!extracted.description) missing.push('description')
  return missing.length === 0 ? { valid: true } : { valid: false, missing }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/extension && bun test tests/lib/validate.test.ts`
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/lib/validate.ts packages/extension/tests/lib/validate.test.ts
git commit -m "feat(ext): extraction validator with TDD tests"
```

---

### Task 2: Messaging contract + capture handler

**Files:**
- Modify: `packages/extension/src/lib/messaging.ts`
- Create: `packages/extension/src/background/handlers/capture.ts`
- Modify: `packages/extension/src/background/index.ts`

- [ ] **Step 1: Add `jd.captureActive` to the Command union**

In `packages/extension/src/lib/messaging.ts`, add the new command variant:

```typescript
export type Command =
  | { cmd: 'health' }
  | { cmd: 'orgs.list'; limit?: number }
  | { cmd: 'orgs.create'; payload: { name: string } }
  | { cmd: 'form.testFill' }
  | { cmd: 'jd.captureActive' }
```

- [ ] **Step 2: Create the capture handler**

Create `packages/extension/src/background/handlers/capture.ts`:

```typescript
// packages/extension/src/background/handlers/capture.ts

import type { Response } from '../../lib/messaging'
import { extError, mapSdkError, mapNetworkError } from '../../lib/errors'
import { validateExtraction } from '../../lib/validate'
import { getClient } from '../client'

export interface CaptureJobPayload {
  id: string
  title: string
}

export async function handleCaptureJob(): Promise<Response<CaptureJobPayload>> {
  try {
    // 1. Get active tab
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
    if (!tab?.id || !tab.url) {
      return {
        ok: false,
        error: extError('UNKNOWN_ERROR', 'No active tab', { layer: 'background' }),
      }
    }

    // 2. Check host — only LinkedIn supported in P4
    if (!/^https?:\/\/([a-z0-9-]+\.)?linkedin\.com\//i.test(tab.url)) {
      return {
        ok: false,
        error: extError('NO_PLUGIN_FOR_HOST', 'No plugin for this site yet', {
          layer: 'background',
          url: tab.url,
        }),
      }
    }

    // 3. Inject content script and extract
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ['content/linkedin.js'],
    })

    const extractResponse = await chrome.tabs.sendMessage(tab.id, { cmd: 'extract' })

    if (!extractResponse?.ok) {
      return {
        ok: false,
        error: extractResponse?.error ?? extError('EXTRACTION_EMPTY', 'No job found on this page', {
          layer: 'content',
          url: tab.url,
        }),
      }
    }

    const extracted = extractResponse.data

    // 4. Validate required fields (title + description)
    const validation = validateExtraction(extracted)
    if (!validation.valid) {
      return {
        ok: false,
        error: extError('EXTRACTION_INCOMPLETE', "Couldn't extract job title and description", {
          layer: 'background',
          url: tab.url,
          context: { missing: validation.missing, raw_fields: extracted.raw_fields },
        }),
      }
    }

    // 5. Create JD in Forge — no org dedup (that's P5)
    const client = await getClient()
    const result = await client.jobDescriptions.create({
      title: extracted.title,
      raw_text: extracted.description,
      url: extracted.url,
      location: extracted.location ?? undefined,
      salary_range: extracted.salary_range ?? undefined,
    })

    if (!result.ok) {
      return { ok: false, error: mapSdkError(result.error, { url: '/api/job-descriptions' }) }
    }

    return {
      ok: true,
      data: { id: result.data.id, title: result.data.title },
    }
  } catch (err) {
    return { ok: false, error: mapNetworkError(err, { url: '/api/job-descriptions' }) }
  }
}
```

- [ ] **Step 3: Wire into background router**

In `packages/extension/src/background/index.ts`, add the import:

```typescript
import { handleCaptureJob } from './handlers/capture'
```

Add the case in the switch before `default`:

```typescript
case 'jd.captureActive':
  response = await handleCaptureJob()
  break
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd packages/extension && npx tsc --noEmit`
Expected: No errors. The exhaustive switch check validates the new case is handled.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/lib/messaging.ts packages/extension/src/background/handlers/capture.ts packages/extension/src/background/index.ts
git commit -m "feat(ext): jd.captureActive handler — inject, extract, validate, create"
```

---

### Task 3: Remove P1 modal from content script

**Files:**
- Modify: `packages/extension/src/content/linkedin.ts`
- Delete: `packages/extension/src/content/shared/modal.ts`
- Delete: `packages/extension/src/content/shared/` (empty after modal removal)

- [ ] **Step 1: Strip modal calls from linkedin.ts**

Replace the entire content of `packages/extension/src/content/linkedin.ts` with:

```typescript
// packages/extension/src/content/linkedin.ts

import { linkedinPlugin } from '../plugin/plugins/linkedin'

interface ExtractMessage {
  cmd: 'extract'
}

type IncomingMessage = ExtractMessage

// Idempotent guard: chrome.scripting.executeScript may re-inject this script
// on every popup click. Without this guard, each injection registers a new
// onMessage listener, causing the same message to be handled multiple times
// and breaking the sendResponse channel.
declare global {
  interface Window {
    __forge_extension_linkedin_ready?: boolean
  }
}

if (!window.__forge_extension_linkedin_ready) {
  window.__forge_extension_linkedin_ready = true

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
          sendResponse({ ok: false, error: { code: 'EXTRACTION_EMPTY', message: 'Plugin returned null' } })
          return
        }
        sendResponse({ ok: true, data: result })
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        sendResponse({ ok: false, error: { code: 'PLUGIN_THREW', message } })
      }
      return true
    }
    return false
  })
}
```

The only changes vs. the original:
- Removed `import { mountDebugModal, mountEmptyModal } from './shared/modal'`
- Removed `mountEmptyModal(location.href)` call (line 35 original)
- Removed `mountDebugModal(result)` call (line 39 original)

- [ ] **Step 2: Delete the modal file and empty shared directory**

```bash
rm packages/extension/src/content/shared/modal.ts
rmdir packages/extension/src/content/shared
```

- [ ] **Step 3: Run all existing LinkedIn tests to verify no breakage**

Run: `cd packages/extension && bun test tests/plugins/linkedin.test.ts`
Expected: All 4 LinkedIn plugin tests pass — they test the plugin directly, not the content script.

- [ ] **Step 4: Run full test suite**

Run: `cd packages/extension && bun test`
Expected: All tests pass. No test imports `content/shared/modal.ts`.

- [ ] **Step 5: Commit**

```bash
git add -A packages/extension/src/content/
git commit -m "refactor(ext): remove P1 debug modal from LinkedIn content script

The content script now returns extraction data to the background worker
without mounting any on-page UI. The background handler (P4) creates the
JD in Forge instead."
```

---

### Task 4: Popup "Capture Job" button

**Files:**
- Modify: `packages/extension/src/popup/Popup.svelte`

- [ ] **Step 1: Replace extract() with captureJob()**

In `packages/extension/src/popup/Popup.svelte`, make the following changes:

Add the import for `CaptureJobPayload` (add to the existing import block):

```typescript
import type { CaptureJobPayload } from '../background/handlers/capture'
```

Replace the entire `extract()` function (lines 34-82) with:

```typescript
async function captureJob() {
  status = 'Capturing job...'
  statusKind = 'info'
  const response = await sendCommand<CaptureJobPayload>({ cmd: 'jd.captureActive' })
  if (response.ok) {
    status = `JD created: ${response.data.id.slice(0, 8)}...`
    statusKind = 'ok'
  } else {
    const code = response.error.code
    if (code === 'NO_PLUGIN_FOR_HOST') {
      status = 'No plugin for this site yet'
    } else if (code === 'EXTRACTION_INCOMPLETE') {
      status = "Couldn't extract job title and description"
    } else if (code === 'EXTRACTION_EMPTY') {
      status = 'No job found on this page'
    } else if (code === 'API_UNREACHABLE') {
      status = 'Forge is not running'
    } else if (code === 'API_DUPLICATE') {
      status = 'Job already captured'
    } else {
      status = `Error: ${response.error.message}`
    }
    statusKind = 'err'
  }
}
```

- [ ] **Step 2: Update the button in the template**

Replace the "Extract Job" button:

```svelte
<button onclick={captureJob}>Capture Job</button>
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd packages/extension && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add packages/extension/src/popup/Popup.svelte
git commit -m "feat(ext): Capture Job button replaces Extract Job in popup"
```

---

### Task 5: JD create smoke test

**Files:**
- Modify: `packages/extension/tests/background/smoke.test.ts`

- [ ] **Step 1: Add JD create smoke test**

In `packages/extension/tests/background/smoke.test.ts`, add the import for `getClient`:

```typescript
import { resetClient, getClient } from '../../src/background/client'
```

(Update the existing `import { resetClient } from '../../src/background/client'` line.)

Add the test inside the existing describe block:

```typescript
test('sdk.jobDescriptions.create creates a JD against running server', async () => {
  if (!serverReachable) return
  const client = await getClient()
  const result = await client.jobDescriptions.create({
    title: 'Smoke Test JD ' + Date.now(),
    raw_text: 'This is a test job description created by the P4 smoke test.',
    url: 'https://www.linkedin.com/jobs/view/smoke-test-' + Date.now(),
    location: 'Remote',
  })
  expect(result.ok).toBe(true)
  if (result.ok) {
    expect(result.data.title).toContain('Smoke Test JD')
    expect(typeof result.data.id).toBe('string')
    expect(result.data.id).toHaveLength(36)
  }
})
```

- [ ] **Step 2: Run smoke tests**

Run: `cd packages/extension && bun test tests/background/smoke.test.ts`
Expected: All 4 smoke tests pass if Forge server is running, skip gracefully if not.

- [ ] **Step 3: Commit**

```bash
git add packages/extension/tests/background/smoke.test.ts
git commit -m "test(ext): smoke test for JD create via SDK (P4 write path)"
```

---

### Task 6: Version bump + build verification

**Files:**
- Modify: `packages/extension/manifest.json`

- [ ] **Step 1: Bump version to 0.0.7**

In `packages/extension/manifest.json`, change:

```json
"version": "0.0.7",
```

- [ ] **Step 2: Full build**

Run: `cd packages/extension && bun run build`
Expected: Clean build, no errors, `dist/` contains updated popup and content scripts.

- [ ] **Step 3: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: All tests pass — validation (5), LinkedIn plugin (4), registry, client, workday, smoke.

- [ ] **Step 4: Commit**

```bash
git add packages/extension/manifest.json
git commit -m "chore(ext): bump version to 0.0.7 (P4)"
```

- [ ] **Step 5: Verification checklist (human sign-off required)**

These require manual testing with the installed extension:

- [ ] `bun run build` succeeds
- [ ] Extension loads in Chrome via "Load unpacked" -> `dist/` (no manifest/SW errors)
- [ ] On a LinkedIn job page: click "Capture Job" -> toast shows "JD created: <id>"
- [ ] Created JD appears in Forge webui at `/job-descriptions` with correct title, description, url, location
- [ ] On a LinkedIn job page with missing description: toast shows "Couldn't extract job title and description"
- [ ] On a non-LinkedIn page: toast shows "No plugin for this site yet"
- [ ] Stop Forge server -> click "Capture Job" on LinkedIn -> toast shows "Forge is not running"
- [ ] P1 debug modal does NOT appear on any interaction
- [ ] Health dot still works (green when Forge running, red when stopped)
- [ ] "List Organizations", "Create Test Org", "Test Fill" buttons still work

Branch `feat/forge-ext/p4-extract-write` is ready for merge after manual verification passes.
