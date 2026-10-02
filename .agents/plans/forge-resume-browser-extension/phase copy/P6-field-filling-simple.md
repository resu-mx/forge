# P6: Field Filling Simple — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove the extension can write to DOM form fields on a Workday application page. "Test Fill" button fills detected text inputs with hardcoded `FORGE-<field_kind>` strings.

**Architecture:** Workday plugin with `detectFormFields` and `fillField` capabilities. Field detection uses `data-automation-id="formField-<name>"` wrapper divs (NOT input-level attributes). P6 scope is text inputs only — custom dropdown/multiselect widgets are MVP. Content script (`content/workday.ts`) listens for `testFill` command. Background worker orchestrates injection + messaging. Popup gets a "Test Fill" button.

**Tech Stack:** Chrome MV3, Svelte 5, Workday DOM (`formField-*` wrappers), synthetic events

**Worktree:** `.claude/worktrees/forge-ext-p6-fill-simple/` on branch `feat/forge-ext/p6-fill-simple`

**SPEC reference:** SPEC.md §4 P6

**Fixture:** `tests/fixtures/workday/application-form-my-info.html` — captured 2026-04-14 from CrowdStrike.

---

## Workday DOM Discovery (from fixture capture)

### How Workday structures form fields

Workday does **not** put `data-automation-id` on `<input>` elements. Instead:

1. **Wrapper div** has `data-automation-id="formField-<fieldName>"`
2. **Input** inside uses standard `id`, `name`, `aria-required` attributes
3. **Label** is a `<label for="...">` inside the wrapper
4. **Required** indicated by `aria-required="true"` on the input (NOT `required` attribute)
5. **Dropdowns** are custom `<button aria-haspopup="listbox">` widgets, NOT `<select>`
6. **Multiselects** are custom `<div data-uxi-widget-type="multiselect">` widgets

### Field inventory (CrowdStrike "My Information" step)

| Wrapper `formField-*` | Input type | FieldKind | Fillable in P6? |
|------------------------|-----------|-----------|----------------|
| `legalName--firstName` | text input | `name.first` | Yes |
| `legalName--lastName` | text input | `name.last` | Yes |
| `addressLine1` | text input | `unknown` | Yes |
| `city` | text input | `address.city` | Yes |
| `postalCode` | text input | `unknown` | Yes |
| `phoneNumber` | text input | `phone` | Yes |
| `extension` | text input | `unknown` | Yes |
| `candidateIsPreviousWorker` | radio | `unknown` | No (radio) |
| `country` | custom dropdown | `address.country` | No (custom widget) |
| `countryRegion` | custom dropdown | `address.state` | No (custom widget) |
| `phoneType` | custom dropdown | `unknown` | No (custom widget) |
| `countryPhoneCode` | multiselect | `unknown` | No (custom widget) |
| `source` | multiselect | `unknown` | No (custom widget) |

Email is displayed as a static `<span>`, not an input — not fillable.

**P6 scope: 7 text inputs.** Custom widgets deferred to MVP.

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `src/plugin/plugins/workday.ts` | Workday plugin: `detectFormFields` + `fillField` |
| Create | `src/content/workday.ts` | Content script: testFill listener |
| Modify | `src/lib/messaging.ts` | Add `form.testFill` command |
| Create | `src/background/handlers/form.ts` | testFill handler: inject + message + collect results |
| Modify | `src/background/index.ts` | Add `form.testFill` case |
| Modify | `src/popup/Popup.svelte` | "Test Fill" button |
| Modify | `vite.config.ts` | Add `content-workday` entry |
| Modify | `manifest.json` | Add `*://*.myworkdayjobs.com/*` host permission |
| Create | `tests/plugins/workday.test.ts` | Plugin unit tests |

All paths relative to `packages/extension/`.

---

### Task 1: Workday plugin — `detectFormFields` + `fillField`

**Files:**
- Create: `packages/extension/src/plugin/plugins/workday.ts`
- Test: `packages/extension/tests/plugins/workday.test.ts`

- [ ] **Step 1: Write plugin tests**

Create `packages/extension/tests/plugins/workday.test.ts`:

```typescript
import { describe, test, expect, beforeAll } from 'bun:test'
import { readFileSync } from 'fs'
import { resolve } from 'path'
import { JSDOM } from 'jsdom'
import { workdayPlugin } from '../../src/plugin/plugins/workday'

const FIXTURE_PATH = resolve(import.meta.dir, '../fixtures/workday/application-form-my-info.html')

let doc: Document

beforeAll(() => {
  const html = readFileSync(FIXTURE_PATH, 'utf-8')
  const dom = new JSDOM(html)
  doc = dom.window.document
})

describe('Workday plugin: detectFormFields', () => {
  test('detects text input fields from fixture', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    expect(fields.length).toBeGreaterThan(0)
    for (const field of fields) {
      expect(field.element).toBeDefined()
      expect(field.field_kind).toBeDefined()
    }
  })

  test('detects firstName and lastName with correct kinds', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    const kinds = fields.map(f => f.field_kind)
    expect(kinds).toContain('name.first')
    expect(kinds).toContain('name.last')
  })

  test('detects city as address.city', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    const cityField = fields.find(f => f.field_kind === 'address.city')
    expect(cityField).toBeDefined()
    expect(cityField!.label_text).toContain('City')
  })

  test('detects phoneNumber as phone', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    const phoneField = fields.find(f => f.field_kind === 'phone')
    expect(phoneField).toBeDefined()
  })

  test('marks required fields correctly', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    const firstName = fields.find(f => f.field_kind === 'name.first')
    const extension = fields.find(f => f.label_text?.includes('Extension'))
    expect(firstName!.required).toBe(true)
    expect(extension!.required).toBe(false)
  })

  test('only returns fillable text inputs (not dropdowns or radios)', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    for (const field of fields) {
      const el = field.element as HTMLInputElement
      expect(el.tagName).toBe('INPUT')
      expect(['text', ''].includes(el.type?.toLowerCase() ?? 'text')).toBe(true)
    }
  })

  test('detects exactly 7 text input fields from fixture', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    // firstName, lastName, addressLine1, city, postalCode, phoneNumber, extension
    expect(fields).toHaveLength(7)
  })
})

describe('Workday plugin: fillField', () => {
  test('fills a text input and returns true', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fill = workdayPlugin.capabilities.fillField!
    const fields = detect(doc)
    const firstName = fields.find(f => f.field_kind === 'name.first')!
    const ok = fill(firstName.element, 'FORGE-name.first')
    expect(ok).toBe(true)
    expect((firstName.element as HTMLInputElement).value).toBe('FORGE-name.first')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/extension && bun test tests/plugins/workday.test.ts`
Expected: Fail — plugin doesn't exist yet.

- [ ] **Step 3: Implement the Workday plugin**

Create `packages/extension/src/plugin/plugins/workday.ts`:

```typescript
// packages/extension/src/plugin/plugins/workday.ts

import type { JobBoardPlugin, DetectedField, FieldKind } from '../types'

const PLUGIN_NAME = 'workday'

/**
 * Map Workday formField-<name> wrapper IDs to canonical FieldKind.
 * The wrapper div has data-automation-id="formField-<fieldName>".
 * We match the fieldName portion (after "formField-").
 */
const FIELD_NAME_MAP: Record<string, FieldKind> = {
  'legalName--firstName': 'name.first',
  'legalName--lastName': 'name.last',
  'city': 'address.city',
  'countryRegion': 'address.state',
  'country': 'address.country',
  'phoneNumber': 'phone',
}

/**
 * Check if an input element is a plain text input (fillable in P6).
 * Excludes radios, checkboxes, hidden inputs, and custom widget inputs.
 */
function isFillableTextInput(el: HTMLInputElement): boolean {
  const type = el.type?.toLowerCase() ?? 'text'
  // Exclude non-text types
  if (['radio', 'checkbox', 'hidden', 'submit', 'button'].includes(type)) return false
  // Exclude Workday's hidden inputs backing custom dropdowns (class="css-77hcv")
  if (el.classList.contains('css-77hcv')) return false
  // Exclude multiselect search inputs (they have data-uxi-widget-type="selectinput")
  if (el.getAttribute('data-uxi-widget-type') === 'selectinput') return false
  return true
}

function detectFormFields(doc: Document): DetectedField[] {
  const fields: DetectedField[] = []
  const wrappers = doc.querySelectorAll('[data-automation-id^="formField-"]')

  for (const wrapper of Array.from(wrappers)) {
    const automationId = wrapper.getAttribute('data-automation-id') ?? ''
    const fieldName = automationId.replace('formField-', '')

    // Find text inputs inside this wrapper
    const inputs = wrapper.querySelectorAll('input')
    for (const input of Array.from(inputs)) {
      if (!isFillableTextInput(input as HTMLInputElement)) continue

      const fieldKind = FIELD_NAME_MAP[fieldName] ?? 'unknown'

      // Find label text from the wrapper's <label>
      const label = wrapper.querySelector('label')
      const labelText = label?.textContent?.trim()?.replace(/\*$/, '').trim() ?? null

      // Required: check aria-required on the input
      const required = input.getAttribute('aria-required') === 'true'

      fields.push({
        element: input,
        label_text: labelText,
        field_kind: fieldKind,
        required,
      })
    }
  }

  return fields
}

/**
 * Fill a text input with a value.
 * Dispatches synthetic events to trigger Workday's framework change detection.
 */
function fillField(element: Element, value: string): boolean {
  try {
    if (element.tagName !== 'INPUT') return false
    const input = element as HTMLInputElement

    input.focus()
    input.value = value
    input.dispatchEvent(new Event('input', { bubbles: true }))
    input.dispatchEvent(new Event('change', { bubbles: true }))
    input.dispatchEvent(new Event('blur', { bubbles: true }))
    return true
  } catch {
    return false
  }
}

export const workdayPlugin: JobBoardPlugin = {
  name: PLUGIN_NAME,
  matches: ['myworkdayjobs.com', '*.myworkdayjobs.com', 'myworkday.com', '*.myworkday.com'],
  capabilities: {
    detectFormFields,
    fillField,
  },
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/extension && bun test tests/plugins/workday.test.ts`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/plugin/plugins/workday.ts packages/extension/tests/plugins/workday.test.ts packages/extension/tests/fixtures/workday/application-form-my-info.html
git commit -m "feat(ext): Workday plugin with detectFormFields + fillField + fixture"
```

---

### Task 2: Content script — `content/workday.ts`

**Files:**
- Create: `packages/extension/src/content/workday.ts`

- [ ] **Step 1: Create the content script**

Create `packages/extension/src/content/workday.ts` following the LinkedIn content script pattern:

```typescript
// packages/extension/src/content/workday.ts

import { workdayPlugin } from '../plugin/plugins/workday'

interface TestFillMessage {
  cmd: 'testFill'
}

type IncomingMessage = TestFillMessage

declare global {
  interface Window {
    __forge_extension_workday_ready?: boolean
  }
}

if (!window.__forge_extension_workday_ready) {
  window.__forge_extension_workday_ready = true

  chrome.runtime.onMessage.addListener((msg: IncomingMessage, _sender, sendResponse) => {
    if (msg.cmd === 'testFill') {
      try {
        const detect = workdayPlugin.capabilities.detectFormFields
        const fill = workdayPlugin.capabilities.fillField
        if (!detect || !fill) {
          sendResponse({
            ok: false,
            error: { code: 'PLUGIN_THREW', message: 'detectFormFields or fillField not defined' },
          })
          return
        }

        const fields = detect(document)
        if (fields.length === 0) {
          sendResponse({
            ok: false,
            error: { code: 'FORM_NOT_DETECTED', message: 'No fillable fields found on this page' },
          })
          return
        }

        let filled = 0
        let failed = 0
        for (const field of fields) {
          const testValue = `FORGE-${field.field_kind}`
          const ok = fill(field.element, testValue)
          if (ok) filled++
          else failed++
        }

        sendResponse({ ok: true, data: { filled, failed, total: fields.length } })
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

- [ ] **Step 2: Commit**

```bash
git add packages/extension/src/content/workday.ts
git commit -m "feat(ext): Workday content script with testFill handler"
```

---

### Task 3: Messaging + background handler + popup button

**Files:**
- Modify: `packages/extension/src/lib/messaging.ts`
- Create: `packages/extension/src/background/handlers/form.ts`
- Modify: `packages/extension/src/background/index.ts`
- Modify: `packages/extension/src/popup/Popup.svelte`

- [ ] **Step 1: Add `form.testFill` to Command union**

In `src/lib/messaging.ts`, add to the Command type:
```typescript
| { cmd: 'form.testFill' }
```

- [ ] **Step 2: Create the form handler**

Create `src/background/handlers/form.ts`:

```typescript
// packages/extension/src/background/handlers/form.ts

import type { Response } from '../../lib/messaging'
import type { ExtensionError } from '../../lib/errors'

export interface TestFillPayload {
  filled: number
  failed: number
  total: number
}

export async function handleTestFill(): Promise<Response<TestFillPayload>> {
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
    if (!tab?.id) {
      return {
        ok: false,
        error: {
          code: 'UNKNOWN_ERROR',
          message: 'No active tab',
          layer: 'background',
          timestamp: new Date().toISOString(),
        } as ExtensionError,
      }
    }

    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ['content/workday.js'],
    })

    const response = await chrome.tabs.sendMessage(tab.id, { cmd: 'testFill' })

    if (response?.ok) {
      return { ok: true, data: response.data as TestFillPayload }
    }

    return {
      ok: false,
      error: response?.error ?? {
        code: 'UNKNOWN_ERROR',
        message: 'Unexpected response from content script',
        layer: 'content',
        timestamp: new Date().toISOString(),
      },
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    return {
      ok: false,
      error: {
        code: 'FORM_NOT_DETECTED',
        message,
        layer: 'background',
        timestamp: new Date().toISOString(),
      } as ExtensionError,
    }
  }
}
```

- [ ] **Step 3: Add switch case to background/index.ts**

Import:
```typescript
import { handleTestFill } from './handlers/form'
```

Add case before `default`:
```typescript
case 'form.testFill':
  response = await handleTestFill()
  break
```

- [ ] **Step 4: Add "Test Fill" button to popup**

In `Popup.svelte`, add import:
```typescript
import type { TestFillPayload } from '../background/handlers/form'
```

Add function after `createTestOrg()`:
```typescript
async function testFill() {
  status = 'Detecting fields...'
  statusKind = 'info'

  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
  if (!tab?.id) {
    status = 'No active tab'
    statusKind = 'err'
    return
  }

  const url = tab.url ?? ''
  if (!/myworkday(jobs)?\.com/i.test(url)) {
    status = 'Not a Workday page'
    statusKind = 'err'
    return
  }

  const response = await sendCommand<TestFillPayload>({ cmd: 'form.testFill' })
  if (response.ok) {
    const { filled, failed, total } = response.data
    status = `Filled ${filled}/${total} fields` + (failed > 0 ? ` (${failed} failed)` : '')
    statusKind = failed > 0 ? 'info' : 'ok'
  } else {
    const code = response.error.code
    if (code === 'FORM_NOT_DETECTED') {
      status = 'No form fields found on this page'
    } else {
      status = `Error: ${response.error.message}`
    }
    statusKind = 'err'
  }
}
```

Add button in `<div class="buttons">`:
```svelte
<button onclick={testFill} class="secondary">Test Fill</button>
```

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/lib/messaging.ts packages/extension/src/background/handlers/form.ts packages/extension/src/background/index.ts packages/extension/src/popup/Popup.svelte
git commit -m "feat(ext): form.testFill command, handler, and popup button"
```

---

### Task 4: Build config — Vite entry + manifest host_permissions

**Files:**
- Modify: `packages/extension/vite.config.ts`
- Modify: `packages/extension/manifest.json`

- [ ] **Step 1: Add Workday content script entry to Vite**

In `vite.config.ts`, update `rollupOptions.input`:
```typescript
input: {
  'content-linkedin': 'src/content/linkedin.ts',
  'content-workday': 'src/content/workday.ts',
},
```

Update `entryFileNames`:
```typescript
entryFileNames: (chunkInfo) => {
  if (chunkInfo.name === 'content-linkedin') return 'content/linkedin.js'
  if (chunkInfo.name === 'content-workday') return 'content/workday.js'
  return 'assets/[name]-[hash].js'
},
```

- [ ] **Step 2: Add Workday host_permissions to manifest**

In `manifest.json`, add to `host_permissions`:
```json
"host_permissions": [
  "*://*.linkedin.com/*",
  "http://localhost:3000/*",
  "*://*.myworkdayjobs.com/*",
  "*://*.myworkday.com/*"
]
```

- [ ] **Step 3: Commit**

```bash
git add packages/extension/vite.config.ts packages/extension/manifest.json
git commit -m "build(ext): Workday content script entry + host permissions"
```

---

### Task 5: Full build + test verification

- [ ] **Step 1: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: All plugin tests (linkedin + workday) + client tests pass.

- [ ] **Step 2: Full build**

Run: `cd packages/extension && bun run build`
Expected: Clean build. `dist/` contains `content/workday.js`, `content/linkedin.js`, popup, background.

- [ ] **Step 3: Manual verification checklist (human sign-off required)**

- [ ] `bun run build` succeeds
- [ ] Extension loads in Chrome via "Load unpacked" → `dist/` (no errors)
- [ ] Navigate to a Workday application form ("My Information" step)
- [ ] Click "Test Fill" → toast shows "Filled 7/7 fields"
- [ ] Hardcoded `FORGE-<kind>` strings visible in form text inputs
- [ ] Values persist after clicking away (no ghost values from framework revert)
- [ ] On non-Workday page → "Not a Workday page" toast
- [ ] Existing LinkedIn extraction still works
- [ ] Health dot still correct

Branch `feat/forge-ext/p6-fill-simple` is ready for merge after manual verification.
