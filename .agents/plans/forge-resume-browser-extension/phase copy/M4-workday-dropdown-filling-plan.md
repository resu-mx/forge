# M4 — Workday Dropdown Filling: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend Workday autofill to handle custom dropdowns, radio buttons, and native selects — using robust semantic selectors instead of fragile CSS classes.

**Architecture:** Add `field_type` discriminator to `DetectedField`. Rewrite detection to use `aria-haspopup`, `input[type=radio]`, and structural sibling checks instead of CSS class names. Make `fillField` async to support click-wait-click dropdown interaction. Each control type gets its own fill helper.

**Tech Stack:** TypeScript, Bun test runner, JSDOM, Vite (IIFE content scripts)

**Spec:** `.claude/plans/forge-resume-browser-extension/phase/M4-workday-dropdown-filling.md`

---

### Task 1: Add FieldType to types and extend DetectedField

**Files:**
- Modify: `packages/extension/src/plugin/types.ts:1-56`

- [ ] **Step 1: Write failing test — DetectedField has field_type**

Create file `packages/extension/tests/plugins/workday-dropdown.test.ts`:

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

describe('Workday plugin: dropdown/radio detection', () => {
  test('all detected fields have a field_type property', () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    for (const field of fields) {
      expect(field.field_type).toBeDefined()
      expect(['text', 'select', 'custom-dropdown', 'radio', 'checkbox']).toContain(field.field_type)
    }
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/extension && bun test tests/plugins/workday-dropdown.test.ts`
Expected: FAIL — `field.field_type` is `undefined`

- [ ] **Step 3: Add FieldType and phone.type to types.ts**

In `packages/extension/src/plugin/types.ts`, add the `FieldType` type and `field_type` to `DetectedField`, and add `'phone.type'` to `FieldKind`:

```typescript
// packages/extension/src/plugin/types.ts

/** Canonical field kinds for autofill */
export type FieldKind =
  | 'name.full' | 'name.first' | 'name.last'
  | 'email' | 'phone' | 'phone.type'
  | 'address.city' | 'address.state' | 'address.country'
  | 'profile.linkedin' | 'profile.github' | 'profile.website'
  | 'work_auth.us' | 'work_auth.sponsorship'
  | 'eeo.gender' | 'eeo.race' | 'eeo.veteran' | 'eeo.disability'
  | 'unknown'

/** Control type discriminator for fill strategy dispatch */
export type FieldType = 'text' | 'select' | 'custom-dropdown' | 'radio' | 'checkbox'

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
  apply_url?: string | null
  company_url?: string | null
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
  field_type: FieldType
  required: boolean
}

export interface JobBoardPlugin {
  name: string
  matches: string[]
  capabilities: {
    extractJD?: (doc: Document, url: string) => ExtractedJob | null
    extractCompany?: (doc: Document, url: string) => ExtractedOrg | null
    normalizeUrl?: (url: string) => string
    detectFormFields?: (doc: Document) => DetectedField[]
    fillField?: (field: DetectedField, value: string) => Promise<boolean>
  }
}
```

- [ ] **Step 4: Fix workday.ts — add field_type: 'text' to existing detection, update fillField signature**

In `packages/extension/src/plugin/plugins/workday.ts`, update `detectFormFields` to set `field_type: 'text'` on each field, and update `fillField` to accept `DetectedField` and return `Promise<boolean>`:

```typescript
// packages/extension/src/plugin/plugins/workday.ts

import type { JobBoardPlugin, DetectedField, FieldKind } from '../types'

const PLUGIN_NAME = 'workday'

const FIELD_NAME_MAP: Record<string, FieldKind> = {
  'legalName--firstName': 'name.first',
  'legalName--lastName': 'name.last',
  'email': 'email',
  'city': 'address.city',
  'countryRegion': 'address.state',
  'country': 'address.country',
  'phoneNumber': 'phone',
}

/**
 * Check if an input element is a plain text input (fillable).
 * Excludes radios, checkboxes, hidden inputs, and custom widget inputs.
 * Uses semantic/structural selectors — NO CSS class names.
 */
function isFillableTextInput(el: HTMLInputElement, wrapper: Element): boolean {
  const type = el.type?.toLowerCase() ?? 'text'
  if (['radio', 'checkbox', 'hidden', 'submit', 'button'].includes(type)) return false
  // Exclude backing inputs for custom dropdowns (sibling of button[aria-haspopup="listbox"])
  if (el.parentElement?.querySelector('button[aria-haspopup="listbox"]')) return false
  // Exclude multiselect search inputs
  if (el.getAttribute('data-uxi-widget-type') === 'selectinput') return false
  return true
}

function detectFormFields(doc: Document): DetectedField[] {
  const fields: DetectedField[] = []
  const wrappers = doc.querySelectorAll('[data-automation-id^="formField-"]')

  for (const wrapper of Array.from(wrappers)) {
    const automationId = wrapper.getAttribute('data-automation-id') ?? ''
    const fieldName = automationId.replace('formField-', '')
    const fieldKind = FIELD_NAME_MAP[fieldName] ?? 'unknown'

    // Find label text from the wrapper's <label>
    const label = wrapper.querySelector('label')
    const labelText = label?.textContent?.trim()?.replace(/\*$/, '').trim() ?? null

    // Find text inputs inside this wrapper
    const inputs = wrapper.querySelectorAll('input')
    for (const input of Array.from(inputs)) {
      if (!isFillableTextInput(input as HTMLInputElement, wrapper)) continue

      const required = input.getAttribute('aria-required') === 'true'

      fields.push({
        element: input,
        label_text: labelText,
        field_kind: fieldKind,
        field_type: 'text',
        required,
      })
    }
  }

  return fields
}

/**
 * Fill a text input with a value using React-compatible event dispatch.
 */
async function fillField(field: DetectedField, value: string): Promise<boolean> {
  try {
    const element = field.element
    if (element.tagName !== 'INPUT') return false
    const input = element as HTMLInputElement
    const win = element.ownerDocument?.defaultView as Window | null

    const EventCtor = win?.Event ?? Event

    input.focus()
    input.dispatchEvent(new EventCtor('focusin', { bubbles: true }))

    const nativeSetter = Object.getOwnPropertyDescriptor(
      win?.HTMLInputElement?.prototype ?? HTMLInputElement.prototype,
      'value',
    )?.set
    if (nativeSetter) {
      nativeSetter.call(input, value)
    } else {
      input.value = value
    }

    const tracker = (input as any)._valueTracker
    if (tracker) tracker.setValue('')

    input.dispatchEvent(new EventCtor('input', { bubbles: true }))
    input.dispatchEvent(new EventCtor('change', { bubbles: true }))

    input.dispatchEvent(new EventCtor('blur', { bubbles: true }))
    input.dispatchEvent(new EventCtor('focusout', { bubbles: true }))
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

- [ ] **Step 5: Fix existing workday.test.ts to match new signature**

In `packages/extension/tests/plugins/workday.test.ts`, update the `fillField` call at line 85 to pass a `DetectedField` instead of `(element, value)`:

```typescript
// Change line 85 from:
//   const ok = fill(firstName.element, 'FORGE-name.first')
// to:
    const ok = await fill(firstName, 'FORGE-name.first')
```

Also update the `fill` call to be awaited. The test function needs to be async:

```typescript
  test('fills a text input and returns true', async () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fill = workdayPlugin.capabilities.fillField!
    const fields = detect(doc)
    const firstName = fields.find(f => f.field_kind === 'name.first')!
    const ok = await fill(firstName, 'FORGE-name.first')
    expect(ok).toBe(true)
    expect((firstName.element as HTMLInputElement).value).toBe('FORGE-name.first')
  })
```

- [ ] **Step 6: Fix content script to use new async fillField signature**

In `packages/extension/src/content/workday.ts`, update both `testFill` and `profileFill` handlers. The message listener must return a promise, so wrap in async IIFE. Pass `field` (not `field.element`) to `fillField`, and `await` the result:

```typescript
// packages/extension/src/content/workday.ts

import { workdayPlugin } from '../plugin/plugins/workday'

interface TestFillMessage {
  cmd: 'testFill'
}

interface ProfileFillMessage {
  cmd: 'profileFill'
  values: Record<string, string>
}

type IncomingMessage = TestFillMessage | ProfileFillMessage

declare global {
  interface Window {
    __forge_extension_workday_ready?: boolean
  }
}

if (!window.__forge_extension_workday_ready) {
  window.__forge_extension_workday_ready = true

  chrome.runtime.onMessage.addListener((msg: IncomingMessage, _sender, sendResponse) => {
    if (msg.cmd === 'testFill') {
      ;(async () => {
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
            const ok = await fill(field, testValue)
            if (ok) filled++
            else failed++
          }

          sendResponse({ ok: true, data: { filled, failed, total: fields.length } })
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err)
          sendResponse({ ok: false, error: { code: 'PLUGIN_THREW', message } })
        }
      })()
      return true
    }

    if (msg.cmd === 'profileFill') {
      ;(async () => {
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

          const values = msg.values
          let filled = 0
          let skipped = 0
          for (const field of fields) {
            const value = values[field.field_kind]
            if (!value || field.field_kind === 'unknown') {
              skipped++
              continue
            }
            const ok = await fill(field, value)
            if (ok) filled++
            else skipped++
          }

          sendResponse({ ok: true, data: { filled, skipped, total: fields.length } })
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err)
          sendResponse({ ok: false, error: { code: 'PLUGIN_THREW', message } })
        }
      })()
      return true
    }

    return false
  })
}
```

- [ ] **Step 7: Run all tests to verify**

Run: `cd packages/extension && bun test`
Expected: ALL PASS — existing tests pass with `field_type: 'text'`, new test passes too.

- [ ] **Step 8: Commit**

```bash
git add packages/extension/src/plugin/types.ts packages/extension/src/plugin/plugins/workday.ts packages/extension/src/content/workday.ts packages/extension/tests/plugins/workday.test.ts packages/extension/tests/plugins/workday-dropdown.test.ts
git commit -m "refactor(ext): add FieldType discriminator and async fillField

- Add FieldType union and field_type to DetectedField
- Add phone.type to FieldKind
- Make fillField async, accept DetectedField instead of Element
- Replace css-77hcv class filter with structural sibling check
- Update content script for async fill loop"
```

---

### Task 2: Detect custom dropdowns and radio buttons

**Files:**
- Modify: `packages/extension/src/plugin/plugins/workday.ts`
- Modify: `packages/extension/tests/plugins/workday-dropdown.test.ts`

- [ ] **Step 1: Write failing tests for dropdown and radio detection**

Append to `packages/extension/tests/plugins/workday-dropdown.test.ts`:

```typescript
  test('detects country as custom-dropdown', () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const country = fields.find(f => f.field_kind === 'address.country')
    expect(country).toBeDefined()
    expect(country!.field_type).toBe('custom-dropdown')
    expect(country!.element.tagName).toBe('BUTTON')
  })

  test('detects state (countryRegion) as custom-dropdown', () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const state = fields.find(f => f.field_kind === 'address.state')
    expect(state).toBeDefined()
    expect(state!.field_type).toBe('custom-dropdown')
    expect(state!.element.tagName).toBe('BUTTON')
  })

  test('detects phoneType as custom-dropdown', () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const phoneType = fields.find(f => f.field_kind === 'phone.type')
    expect(phoneType).toBeDefined()
    expect(phoneType!.field_type).toBe('custom-dropdown')
  })

  test('detects candidateIsPreviousWorker as radio', () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const radio = fields.find(f => f.field_type === 'radio')
    expect(radio).toBeDefined()
    expect(radio!.label_text).toContain('previously worked')
  })

  test('does not detect multiselect widgets (source, countryPhoneCode)', () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const multiselects = fields.filter(f =>
      f.field_kind === 'unknown' && f.label_text?.includes('How Did You Hear')
    )
    expect(multiselects).toHaveLength(0)
  })

  test('text inputs still detected correctly (7 total)', () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const textFields = fields.filter(f => f.field_type === 'text')
    expect(textFields).toHaveLength(7)
  })

  test('total detected fields: 7 text + 3 dropdown + 1 radio = 11', () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    expect(fields).toHaveLength(11)
  })
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/extension && bun test tests/plugins/workday-dropdown.test.ts`
Expected: FAIL — dropdowns and radios not detected yet

- [ ] **Step 3: Update FIELD_NAME_MAP and rewrite detectFormFields**

In `packages/extension/src/plugin/plugins/workday.ts`, add `phoneType` to the map and rewrite `detectFormFields` to detect all control types:

```typescript
const FIELD_NAME_MAP: Record<string, FieldKind> = {
  'legalName--firstName': 'name.first',
  'legalName--lastName': 'name.last',
  'email': 'email',
  'city': 'address.city',
  'countryRegion': 'address.state',
  'country': 'address.country',
  'phoneNumber': 'phone',
  'phoneType': 'phone.type',
}

function detectFormFields(doc: Document): DetectedField[] {
  const fields: DetectedField[] = []
  const wrappers = doc.querySelectorAll('[data-automation-id^="formField-"]')

  for (const wrapper of Array.from(wrappers)) {
    const automationId = wrapper.getAttribute('data-automation-id') ?? ''
    const fieldName = automationId.replace('formField-', '')
    const fieldKind = FIELD_NAME_MAP[fieldName] ?? 'unknown'

    // Find label text from the wrapper's <label> or <legend>
    const label = wrapper.querySelector('legend label, label')
    const labelText = label?.textContent?.trim()?.replace(/\*$/, '').trim() ?? null

    // Required: check aria-required on the primary interactive element
    const getRequired = (el: Element) => el.getAttribute('aria-required') === 'true'

    // 1. Custom dropdown? (button[aria-haspopup="listbox"] inside wrapper)
    const dropdownBtn = wrapper.querySelector('button[aria-haspopup="listbox"]')
    if (dropdownBtn) {
      fields.push({
        element: dropdownBtn,
        label_text: labelText,
        field_kind: fieldKind,
        field_type: 'custom-dropdown',
        required: getRequired(dropdownBtn) ||
          dropdownBtn.getAttribute('aria-label')?.includes('Required') === true,
      })
      continue
    }

    // 2. Radio group? (input[type="radio"] inside wrapper)
    const radios = wrapper.querySelectorAll('input[type="radio"]')
    if (radios.length > 0) {
      // The group container has the aria-required
      const groupDiv = wrapper.querySelector('[aria-required]')
      fields.push({
        element: wrapper,
        label_text: labelText,
        field_kind: fieldKind,
        field_type: 'radio',
        required: groupDiv ? getRequired(groupDiv) : false,
      })
      continue
    }

    // 3. Native select?
    const select = wrapper.querySelector('select')
    if (select) {
      fields.push({
        element: select,
        label_text: labelText,
        field_kind: fieldKind,
        field_type: 'select',
        required: getRequired(select),
      })
      continue
    }

    // 4. Text inputs (existing logic, semantic exclusions)
    const inputs = wrapper.querySelectorAll('input')
    for (const input of Array.from(inputs)) {
      if (!isFillableTextInput(input as HTMLInputElement, wrapper)) continue

      fields.push({
        element: input,
        label_text: labelText,
        field_kind: fieldKind,
        field_type: 'text',
        required: getRequired(input),
      })
    }
  }

  return fields
}
```

- [ ] **Step 4: Update existing workday.test.ts count expectation**

In `packages/extension/tests/plugins/workday.test.ts`, the test at line 71 ("detects exactly 7 text input fields") needs a description update since the total is now 11 but text inputs are still 7. Update the test:

```typescript
  test('detects exactly 7 text input fields from fixture', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    const textFields = fields.filter(f => f.field_type === 'text')
    // firstName, lastName, addressLine1, city, postalCode, phoneNumber, extension
    expect(textFields).toHaveLength(7)
  })
```

Also update the test at line 60 ("only returns fillable text inputs") since it now also returns dropdowns/radios:

```typescript
  test('text fields are all INPUT elements with text type', () => {
    const detect = workdayPlugin.capabilities.detectFormFields!
    const fields = detect(doc)
    const textFields = fields.filter(f => f.field_type === 'text')
    for (const field of textFields) {
      const el = field.element as HTMLInputElement
      expect(el.tagName).toBe('INPUT')
      const type = el.type?.toLowerCase() ?? 'text'
      expect(['text', ''].includes(type)).toBe(true)
    }
  })
```

- [ ] **Step 5: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add packages/extension/src/plugin/plugins/workday.ts packages/extension/tests/plugins/workday-dropdown.test.ts packages/extension/tests/plugins/workday.test.ts
git commit -m "feat(ext): detect custom dropdowns and radio buttons in Workday forms

- Detect button[aria-haspopup=listbox] as custom-dropdown
- Detect input[type=radio] groups as radio
- Add phoneType to FIELD_NAME_MAP
- Total detection: 7 text + 3 dropdown + 1 radio = 11 fields"
```

---

### Task 3: Implement fillRadio

**Files:**
- Modify: `packages/extension/src/plugin/plugins/workday.ts`
- Modify: `packages/extension/tests/plugins/workday-dropdown.test.ts`

- [ ] **Step 1: Write failing test for radio fill**

Append to `packages/extension/tests/plugins/workday-dropdown.test.ts`:

```typescript
describe('Workday plugin: fillRadio', () => {
  test('selects radio by value match (e.g. "false" for No)', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const radio = fields.find(f => f.field_type === 'radio')!
    const ok = await fill(radio, 'false')
    expect(ok).toBe(true)

    // Verify the "No" radio is checked
    const noInput = radio.element.querySelector('input[type="radio"][value="false"]') as HTMLInputElement
    expect(noInput.checked).toBe(true)
  })

  test('selects radio by label text match (case-insensitive)', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const radio = fields.find(f => f.field_type === 'radio')!
    const ok = await fill(radio, 'Yes')
    expect(ok).toBe(true)

    const yesInput = radio.element.querySelector('input[type="radio"][value="true"]') as HTMLInputElement
    expect(yesInput.checked).toBe(true)
  })

  test('returns false for radio with no matching value or label', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const radio = fields.find(f => f.field_type === 'radio')!
    const ok = await fill(radio, 'Maybe')
    expect(ok).toBe(false)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/extension && bun test tests/plugins/workday-dropdown.test.ts`
Expected: FAIL — radio fill not dispatched to yet

- [ ] **Step 3: Implement fillRadio and dispatch in fillField**

In `packages/extension/src/plugin/plugins/workday.ts`, add `fillRadio` and update `fillField` to dispatch based on `field_type`:

```typescript
/**
 * Fill a radio button group by matching value or label text.
 */
function fillRadio(wrapper: Element, value: string): boolean {
  const radios = wrapper.querySelectorAll('input[type="radio"]')
  const win = wrapper.ownerDocument?.defaultView as Window | null
  const EventCtor = win?.Event ?? Event

  for (const radio of Array.from(radios)) {
    const input = radio as HTMLInputElement

    // Match by value attribute
    if (input.value === value) {
      input.checked = true
      input.dispatchEvent(new EventCtor('click', { bubbles: true }))
      input.dispatchEvent(new EventCtor('change', { bubbles: true }))
      return true
    }

    // Match by sibling label text (case-insensitive)
    const label = input.id
      ? wrapper.querySelector(`label[for="${input.id}"]`)
      : null
    if (label && label.textContent?.trim().toLowerCase() === value.toLowerCase()) {
      input.checked = true
      input.dispatchEvent(new EventCtor('click', { bubbles: true }))
      input.dispatchEvent(new EventCtor('change', { bubbles: true }))
      return true
    }
  }

  return false
}

/**
 * Fill a text input with React-compatible event dispatch.
 */
function fillTextInput(element: Element, value: string): boolean {
  try {
    if (element.tagName !== 'INPUT') return false
    const input = element as HTMLInputElement
    const win = element.ownerDocument?.defaultView as Window | null

    const EventCtor = win?.Event ?? Event

    input.focus()
    input.dispatchEvent(new EventCtor('focusin', { bubbles: true }))

    const nativeSetter = Object.getOwnPropertyDescriptor(
      win?.HTMLInputElement?.prototype ?? HTMLInputElement.prototype,
      'value',
    )?.set
    if (nativeSetter) {
      nativeSetter.call(input, value)
    } else {
      input.value = value
    }

    const tracker = (input as any)._valueTracker
    if (tracker) tracker.setValue('')

    input.dispatchEvent(new EventCtor('input', { bubbles: true }))
    input.dispatchEvent(new EventCtor('change', { bubbles: true }))

    input.dispatchEvent(new EventCtor('blur', { bubbles: true }))
    input.dispatchEvent(new EventCtor('focusout', { bubbles: true }))
    return true
  } catch {
    return false
  }
}

/**
 * Fill a native <select> element.
 */
function fillNativeSelect(element: Element, value: string): boolean {
  try {
    if (element.tagName !== 'SELECT') return false
    const select = element as HTMLSelectElement
    const win = element.ownerDocument?.defaultView as Window | null
    const EventCtor = win?.Event ?? Event

    // Find option by value or text
    let matched = false
    for (const option of Array.from(select.options)) {
      if (option.value === value || option.textContent?.trim().toLowerCase() === value.toLowerCase()) {
        select.value = option.value
        matched = true
        break
      }
    }
    if (!matched) return false

    select.dispatchEvent(new EventCtor('change', { bubbles: true }))
    return true
  } catch {
    return false
  }
}

async function fillField(field: DetectedField, value: string): Promise<boolean> {
  switch (field.field_type) {
    case 'text':
      return fillTextInput(field.element, value)
    case 'radio':
      return fillRadio(field.element, value)
    case 'select':
      return fillNativeSelect(field.element, value)
    case 'custom-dropdown':
      // Placeholder — implemented in Task 4
      return false
    case 'checkbox':
      return false
  }
}
```

Remove the old standalone `fillField` function body — the dispatch switch replaces it entirely.

- [ ] **Step 4: Run tests**

Run: `cd packages/extension && bun test`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/plugin/plugins/workday.ts packages/extension/tests/plugins/workday-dropdown.test.ts
git commit -m "feat(ext): implement fillRadio and fillNativeSelect for Workday

- fillRadio matches by value attr or label text (case-insensitive)
- fillNativeSelect matches by value or option text
- fillField dispatches to per-type handlers
- Extract fillTextInput from old fillField"
```

---

### Task 4: Implement fillCustomDropdown

**Files:**
- Modify: `packages/extension/src/plugin/plugins/workday.ts`
- Modify: `packages/extension/tests/plugins/workday-dropdown.test.ts`

- [ ] **Step 1: Write failing test for custom dropdown fill**

Workday's custom dropdown opens a listbox dynamically. In JSDOM we need to simulate this by creating the listbox DOM after the button click. Append to `packages/extension/tests/plugins/workday-dropdown.test.ts`:

```typescript
describe('Workday plugin: fillCustomDropdown', () => {
  test('fills country dropdown via click-select interaction', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const country = fields.find(f => f.field_kind === 'address.country')!
    const btn = country.element as HTMLButtonElement

    // Simulate Workday's dropdown popup: when button is clicked, a listbox appears
    btn.addEventListener('click', () => {
      // Create the popup listbox (Workday renders this dynamically)
      const listbox = doc.createElement('div')
      listbox.setAttribute('role', 'listbox')
      listbox.id = 'test-country-listbox'

      const options = ['United States of America', 'Canada', 'United Kingdom']
      for (const text of options) {
        const opt = doc.createElement('div')
        opt.setAttribute('role', 'option')
        opt.textContent = text
        listbox.appendChild(opt)
      }

      doc.body.appendChild(listbox)
    })

    const ok = await fill(country, 'United States of America')
    expect(ok).toBe(true)

    // Clean up
    const listbox = doc.getElementById('test-country-listbox')
    listbox?.remove()
  })

  test('fills state dropdown with fuzzy match (e.g. "Colorado" matches "Colorado (CO)")', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const state = fields.find(f => f.field_kind === 'address.state')!
    const btn = state.element as HTMLButtonElement

    btn.addEventListener('click', () => {
      const listbox = doc.createElement('div')
      listbox.setAttribute('role', 'listbox')
      listbox.id = 'test-state-listbox'

      const options = ['California (CA)', 'Colorado (CO)', 'Connecticut (CT)']
      for (const text of options) {
        const opt = doc.createElement('div')
        opt.setAttribute('role', 'option')
        opt.textContent = text
        listbox.appendChild(opt)
      }

      doc.body.appendChild(listbox)
    })

    const ok = await fill(state, 'Colorado')
    expect(ok).toBe(true)

    const listbox = doc.getElementById('test-state-listbox')
    listbox?.remove()
  })

  test('returns false when no matching option found', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const state = fields.find(f => f.field_kind === 'address.state')!
    const btn = state.element as HTMLButtonElement

    btn.addEventListener('click', () => {
      const listbox = doc.createElement('div')
      listbox.setAttribute('role', 'listbox')
      listbox.id = 'test-state-listbox-2'

      const opt = doc.createElement('div')
      opt.setAttribute('role', 'option')
      opt.textContent = 'California (CA)'
      listbox.appendChild(opt)

      doc.body.appendChild(listbox)
    })

    const ok = await fill(state, 'Narnia')
    expect(ok).toBe(false)

    const listbox = doc.getElementById('test-state-listbox-2')
    listbox?.remove()
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/extension && bun test tests/plugins/workday-dropdown.test.ts`
Expected: FAIL — fillCustomDropdown returns `false` (placeholder)

- [ ] **Step 3: Implement fillCustomDropdown**

In `packages/extension/src/plugin/plugins/workday.ts`, add `fillCustomDropdown`:

```typescript
/**
 * Wait for an element matching a selector to appear in the document.
 * Polls every 50ms up to timeoutMs.
 */
function waitForElement(doc: Document, selector: string, timeoutMs: number): Promise<Element | null> {
  return new Promise((resolve) => {
    const el = doc.querySelector(selector)
    if (el) { resolve(el); return }

    const start = Date.now()
    const interval = setInterval(() => {
      const el = doc.querySelector(selector)
      if (el) { clearInterval(interval); resolve(el); return }
      if (Date.now() - start > timeoutMs) { clearInterval(interval); resolve(null) }
    }, 50)
  })
}

/**
 * Find the best matching option in a listbox.
 * Tries exact match first, then case-insensitive startsWith for partial matches
 * (e.g., "Colorado" matches "Colorado (CO)").
 */
function findMatchingOption(listbox: Element, value: string): Element | null {
  const options = listbox.querySelectorAll('[role="option"]')
  const valueLower = value.toLowerCase().trim()

  // Exact match
  for (const opt of Array.from(options)) {
    if (opt.textContent?.trim() === value) return opt
  }

  // Case-insensitive exact match
  for (const opt of Array.from(options)) {
    if (opt.textContent?.trim().toLowerCase() === valueLower) return opt
  }

  // startsWith match (for "Colorado" matching "Colorado (CO)")
  for (const opt of Array.from(options)) {
    const text = opt.textContent?.trim().toLowerCase() ?? ''
    if (text.startsWith(valueLower)) return opt
  }

  return null
}

/**
 * Fill a Workday custom dropdown widget.
 * Interaction: click button → wait for listbox → find option → click option.
 */
async function fillCustomDropdown(element: Element, value: string): Promise<boolean> {
  try {
    const btn = element as HTMLButtonElement
    const doc = element.ownerDocument
    const win = doc?.defaultView as Window | null
    const EventCtor = win?.Event ?? Event

    // 1. Click the button to open the dropdown popup
    btn.click()

    // 2. Wait for listbox to appear (Workday lazy-renders it)
    const listbox = await waitForElement(doc, '[role="listbox"]', 2000)
    if (!listbox) return false

    // 3. Find matching option
    const option = findMatchingOption(listbox, value)
    if (!option) {
      // Close the dropdown by clicking the button again
      btn.click()
      return false
    }

    // 4. Click the option to select it
    ;(option as HTMLElement).click()
    option.dispatchEvent(new EventCtor('click', { bubbles: true }))

    return true
  } catch {
    return false
  }
}
```

Then update the `fillField` switch to call `fillCustomDropdown`:

```typescript
    case 'custom-dropdown':
      return fillCustomDropdown(field.element, value)
```

- [ ] **Step 4: Run tests**

Run: `cd packages/extension && bun test`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/plugin/plugins/workday.ts packages/extension/tests/plugins/workday-dropdown.test.ts
git commit -m "feat(ext): implement fillCustomDropdown for Workday select widgets

- Click-wait-click interaction pattern
- waitForElement polls for listbox with 2s timeout
- findMatchingOption: exact → case-insensitive → startsWith fuzzy match
- Tests use dynamic listbox injection to simulate Workday popup"
```

---

### Task 5: Extend profile mapping (country codes, phone type)

**Files:**
- Modify: `packages/extension/src/lib/profile-map.ts`
- Modify: `packages/extension/tests/lib/profile-map.test.ts`

- [ ] **Step 1: Write failing tests for country name mapping and phone.type**

Append to `packages/extension/tests/lib/profile-map.test.ts`:

```typescript
  test('maps country code US to display name', () => {
    const map = buildProfileFieldMap(profile({
      address: { city: 'Reston', state: 'VA', country_code: 'US' },
    }))
    expect(map['address.country']).toBe('United States of America')
  })

  test('maps country code CA to Canada', () => {
    const map = buildProfileFieldMap(profile({
      address: { city: 'Toronto', state: 'ON', country_code: 'CA' },
    }))
    expect(map['address.country']).toBe('Canada')
  })

  test('passes through unknown country code as-is', () => {
    const map = buildProfileFieldMap(profile({
      address: { city: 'Berlin', state: null, country_code: 'DE' },
    }))
    expect(map['address.country']).toBe('DE')
  })

  test('sets phone.type to Mobile when phone exists', () => {
    const map = buildProfileFieldMap(profile({ phone: '555-1234' }))
    expect(map['phone.type']).toBe('Mobile')
  })

  test('does not set phone.type when phone is null', () => {
    const map = buildProfileFieldMap(profile({ phone: null }))
    expect(map['phone.type']).toBeUndefined()
  })
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/extension && bun test tests/lib/profile-map.test.ts`
Expected: FAIL — country code `'US'` is not mapped to `'United States of America'`, and `phone.type` is not set.

- [ ] **Step 3: Implement country code mapping and phone.type default**

Update `packages/extension/src/lib/profile-map.ts`:

```typescript
/**
 * Map ISO country codes to Workday display names.
 * Workday custom dropdowns show full country names, not ISO codes.
 * Extend as needed for additional countries.
 */
const COUNTRY_CODE_TO_NAME: Record<string, string> = {
  'US': 'United States of America',
  'CA': 'Canada',
  'GB': 'United Kingdom',
  'AU': 'Australia',
  'DE': 'Germany',
  'FR': 'France',
  'IN': 'India',
  'JP': 'Japan',
  'BR': 'Brazil',
  'MX': 'Mexico',
  'IL': 'Israel',
  'SG': 'Singapore',
  'IE': 'Ireland',
  'NL': 'Netherlands',
  'SE': 'Sweden',
  'CH': 'Switzerland',
  'NZ': 'New Zealand',
}
```

Then update the `buildProfileFieldMap` function — change the country mapping line and add phone.type:

```typescript
    if (profile.address.country_code) {
      map['address.country'] = COUNTRY_CODE_TO_NAME[profile.address.country_code] ?? profile.address.country_code
    }
```

And after the phone mapping block:

```typescript
  if (profile.phone) {
    map['phone'] = profile.phone.replace(/^\+\d+\s+/, '')
    map['phone.type'] = 'Mobile'
  }
```

- [ ] **Step 4: Update existing test that checks raw country code**

The existing test at line 28 (`expect(map['address.country']).toBe('US')`) will now fail because `US` maps to `United States of America`. Update it:

```typescript
    expect(map['address.country']).toBe('United States of America')
```

Also update the test at line 87 (`expect(map['address.country']).toBe('US')`) for the same reason:

```typescript
    expect(map['address.country']).toBe('United States of America')
```

- [ ] **Step 5: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add packages/extension/src/lib/profile-map.ts packages/extension/tests/lib/profile-map.test.ts
git commit -m "feat(ext): add country code→name mapping and phone.type default

- COUNTRY_CODE_TO_NAME lookup for 17 common countries
- Unknown codes pass through as-is
- phone.type defaults to 'Mobile' when phone exists"
```

---

### Task 6: Integration test — mixed field fill in one pass

**Files:**
- Create: `packages/extension/tests/plugins/workday-integration.test.ts`

- [ ] **Step 1: Write integration test**

Create `packages/extension/tests/plugins/workday-integration.test.ts`:

```typescript
import { describe, test, expect, beforeEach } from 'bun:test'
import { readFileSync } from 'fs'
import { resolve } from 'path'
import { JSDOM } from 'jsdom'
import { workdayPlugin } from '../../src/plugin/plugins/workday'
import { buildProfileFieldMap } from '../../src/lib/profile-map'
import type { ProfileFields } from '../../src/lib/profile-map'

const FIXTURE_PATH = resolve(import.meta.dir, '../fixtures/workday/application-form-my-info.html')

let doc: Document
let dom: JSDOM

beforeEach(() => {
  const html = readFileSync(FIXTURE_PATH, 'utf-8')
  dom = new JSDOM(html)
  doc = dom.window.document
})

const TEST_PROFILE: ProfileFields = {
  name: 'Ada Lovelace',
  email: 'ada@example.com',
  phone: '+1 555-0199',
  address: { city: 'Austin', state: 'Texas', country_code: 'US' },
  urls: [{ key: 'linkedin', url: 'https://linkedin.com/in/ada' }],
}

describe('Workday integration: profileFill flow', () => {
  test('fills text inputs from profile', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const values = buildProfileFieldMap(TEST_PROFILE)

    const firstName = fields.find(f => f.field_kind === 'name.first')!
    const ok = await fill(firstName, values['name.first'])
    expect(ok).toBe(true)
    expect((firstName.element as HTMLInputElement).value).toBe('Ada')
  })

  test('fills radio button from profile values', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!

    const radio = fields.find(f => f.field_type === 'radio')!
    const ok = await fill(radio, 'No')
    expect(ok).toBe(true)
  })

  test('fills custom dropdown with simulated popup', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const values = buildProfileFieldMap(TEST_PROFILE)

    const country = fields.find(f => f.field_kind === 'address.country')!
    const btn = country.element as HTMLButtonElement

    // Simulate Workday dropdown popup
    btn.addEventListener('click', () => {
      if (doc.querySelector('[role="listbox"]')) return
      const listbox = doc.createElement('div')
      listbox.setAttribute('role', 'listbox')
      for (const name of ['United States of America', 'Canada', 'United Kingdom']) {
        const opt = doc.createElement('div')
        opt.setAttribute('role', 'option')
        opt.textContent = name
        listbox.appendChild(opt)
      }
      doc.body.appendChild(listbox)
    })

    const ok = await fill(country, values['address.country'])
    expect(ok).toBe(true)
    expect(values['address.country']).toBe('United States of America')
  })

  test('full profile fill counts: fills text + skips dropdowns without popup', async () => {
    const fields = workdayPlugin.capabilities.detectFormFields!(doc)
    const fill = workdayPlugin.capabilities.fillField!
    const values = buildProfileFieldMap(TEST_PROFILE)

    let filled = 0
    let skipped = 0

    for (const field of fields) {
      const value = values[field.field_kind]
      if (!value || field.field_kind === 'unknown') {
        skipped++
        continue
      }
      const ok = await fill(field, value)
      if (ok) filled++
      else skipped++
    }

    // Text inputs that have profile values: firstName, lastName, city, phone = 4
    // Dropdowns without popup simulation will fail gracefully = skipped
    expect(filled).toBeGreaterThanOrEqual(4)
    expect(filled + skipped).toBe(fields.length)
  })
})
```

- [ ] **Step 2: Run integration test**

Run: `cd packages/extension && bun test tests/plugins/workday-integration.test.ts`
Expected: ALL PASS

- [ ] **Step 3: Run full extension test suite**

Run: `cd packages/extension && bun test`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add packages/extension/tests/plugins/workday-integration.test.ts
git commit -m "test(ext): add M4 integration tests for mixed field autofill

- Profile mapping → detect → fill loop for text, radio, dropdown
- Simulated Workday popup for dropdown tests
- Verifies graceful skip when popup doesn't appear"
```

---

### Task 7: Version bump and build validation

**Files:**
- Modify: `packages/extension/manifest.json`
- Modify: `packages/extension/manifest.firefox.json`

- [ ] **Step 1: Bump version in both manifests**

In `packages/extension/manifest.json`, change `"version": "0.1.3"` to `"version": "0.1.4"`.

In `packages/extension/manifest.firefox.json`, change `"version": "0.1.3"` to `"version": "0.1.4"`.

- [ ] **Step 2: Run manifest parity tests**

Run: `cd packages/extension && bun test tests/build/manifests.test.ts`
Expected: ALL PASS (versions match)

- [ ] **Step 3: Build both browsers**

Run: `cd packages/extension && bun run build`
Expected: Build succeeds for both Chrome and Firefox. Output in `dist/chrome/` and `dist/firefox/`.

- [ ] **Step 4: Run build output tests**

Run: `cd packages/extension && bun test tests/build/output.test.ts`
Expected: PASS — no `import` statements in content scripts.

- [ ] **Step 5: Run full test suite**

Run: `cd packages/extension && bun test`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json
git commit -m "chore(ext): bump version to 0.1.4 (M4)"
```
