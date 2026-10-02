# P7: Field Filling + Profile Read — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fill Workday form fields with real profile data from Forge instead of hardcoded test values — completing the prototype series.

**Architecture:** Background handler loads the user's profile via `client.profile.get()`, maps profile fields to `FieldKind` values, then sends the value map to the Workday content script. Content script detects form fields (same as P6), fills fields that have matching profile values, and skips unknown/unmapped fields. No plugin imports in the background worker (avoids shared chunk issue from P5).

**Tech Stack:** Chrome MV3 extension, `@forge/sdk`, Bun test runner

**Worktree:** `.claude/worktrees/forge-ext-p7-fill-profile/` on branch `feat/forge-ext/p7-fill-profile`

**SPEC reference:** SPEC.md §4 P7

---

### File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `packages/extension/src/lib/profile-map.ts` | `buildProfileFieldMap()` — profile → FieldKind value map |
| Create | `packages/extension/tests/lib/profile-map.test.ts` | Tests for profile mapping |
| Modify | `packages/extension/src/plugin/plugins/workday.ts` | Add `email` to FIELD_NAME_MAP |
| Modify | `packages/extension/src/lib/messaging.ts` | Add `form.profileFill` command |
| Create | `packages/extension/src/background/handlers/autofill.ts` | `handleProfileFill` — load profile, build map, inject, fill |
| Modify | `packages/extension/src/background/index.ts` | Add `form.profileFill` case |
| Modify | `packages/extension/src/content/workday.ts` | Handle `profileFill` message with value map |
| Modify | `packages/extension/src/popup/Popup.svelte` | Replace "Test Fill" with "Autofill" |
| Modify | `packages/extension/tests/background/smoke.test.ts` | Smoke test for profile.get() |
| Modify | `packages/extension/manifest.json` | Version bump to 0.0.9 |

---

### Task 1: Profile → FieldKind mapping utility (TDD)

**Files:**
- Create: `packages/extension/tests/lib/profile-map.test.ts`
- Create: `packages/extension/src/lib/profile-map.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/extension/tests/lib/profile-map.test.ts`:

```typescript
import { describe, test, expect } from 'bun:test'
import { buildProfileFieldMap } from '../../src/lib/profile-map'

describe('buildProfileFieldMap', () => {
  test('maps all profile fields to field kinds', () => {
    const map = buildProfileFieldMap({
      name: 'Adam Smith',
      email: 'adam@example.com',
      phone: '+1 555-1234',
      location: 'Reston, VA',
      linkedin: 'https://linkedin.com/in/adamsmith',
      github: 'https://github.com/adamsmith',
      website: 'https://adamsmith.dev',
    })

    expect(map['name.full']).toBe('Adam Smith')
    expect(map['name.first']).toBe('Adam')
    expect(map['name.last']).toBe('Smith')
    expect(map['email']).toBe('adam@example.com')
    expect(map['phone']).toBe('+1 555-1234')
    expect(map['address.city']).toBe('Reston, VA')
    expect(map['profile.linkedin']).toBe('https://linkedin.com/in/adamsmith')
    expect(map['profile.github']).toBe('https://github.com/adamsmith')
    expect(map['profile.website']).toBe('https://adamsmith.dev')
  })

  test('splits multi-word last names correctly', () => {
    const map = buildProfileFieldMap({
      name: 'Jean Claude Van Damme',
      email: null, phone: null, location: null,
      linkedin: null, github: null, website: null,
    })

    expect(map['name.first']).toBe('Jean')
    expect(map['name.last']).toBe('Claude Van Damme')
  })

  test('handles single-word name (first only)', () => {
    const map = buildProfileFieldMap({
      name: 'Prince',
      email: null, phone: null, location: null,
      linkedin: null, github: null, website: null,
    })

    expect(map['name.full']).toBe('Prince')
    expect(map['name.first']).toBe('Prince')
    expect(map['name.last']).toBeUndefined()
  })

  test('skips null fields', () => {
    const map = buildProfileFieldMap({
      name: 'Test User',
      email: null, phone: null, location: null,
      linkedin: null, github: null, website: null,
    })

    expect(map['email']).toBeUndefined()
    expect(map['phone']).toBeUndefined()
    expect(map['address.city']).toBeUndefined()
    expect(map['profile.linkedin']).toBeUndefined()
    expect(map['profile.github']).toBeUndefined()
    expect(map['profile.website']).toBeUndefined()
  })

  test('returns empty object for empty name', () => {
    const map = buildProfileFieldMap({
      name: '',
      email: null, phone: null, location: null,
      linkedin: null, github: null, website: null,
    })

    expect(map['name.full']).toBeUndefined()
    expect(map['name.first']).toBeUndefined()
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/extension && bun test tests/lib/profile-map.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement buildProfileFieldMap**

Create `packages/extension/src/lib/profile-map.ts`:

```typescript
/**
 * Map user profile fields to canonical FieldKind values for form autofill.
 * Returns a plain Record (not a Map) so it serializes over chrome messaging.
 */
export interface ProfileFields {
  name: string
  email: string | null
  phone: string | null
  location: string | null
  linkedin: string | null
  github: string | null
  website: string | null
}

export function buildProfileFieldMap(profile: ProfileFields): Record<string, string> {
  const map: Record<string, string> = {}

  if (profile.name?.trim()) {
    map['name.full'] = profile.name
    const parts = profile.name.trim().split(/\s+/)
    if (parts.length >= 2) {
      map['name.first'] = parts[0]
      map['name.last'] = parts.slice(1).join(' ')
    } else {
      map['name.first'] = parts[0]
    }
  }

  if (profile.email) map['email'] = profile.email
  if (profile.phone) map['phone'] = profile.phone
  if (profile.location) map['address.city'] = profile.location
  if (profile.linkedin) map['profile.linkedin'] = profile.linkedin
  if (profile.github) map['profile.github'] = profile.github
  if (profile.website) map['profile.website'] = profile.website

  return map
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/extension && bun test tests/lib/profile-map.test.ts`
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/lib/profile-map.ts packages/extension/tests/lib/profile-map.test.ts
git commit -m "feat(ext): buildProfileFieldMap — profile to FieldKind value mapping"
```

---

### Task 2: Extend Workday FIELD_NAME_MAP

**Files:**
- Modify: `packages/extension/src/plugin/plugins/workday.ts`

- [ ] **Step 1: Add email to the field name map**

In `packages/extension/src/plugin/plugins/workday.ts`, find the `FIELD_NAME_MAP` and add the `email` entry:

```typescript
const FIELD_NAME_MAP: Record<string, FieldKind> = {
  'legalName--firstName': 'name.first',
  'legalName--lastName': 'name.last',
  'email': 'email',
  'city': 'address.city',
  'countryRegion': 'address.state',
  'country': 'address.country',
  'phoneNumber': 'phone',
}
```

- [ ] **Step 2: Run existing Workday tests**

Run: `cd packages/extension && bun test tests/plugins/workday.test.ts`
Expected: All existing tests still pass.

- [ ] **Step 3: Commit**

```bash
git add packages/extension/src/plugin/plugins/workday.ts
git commit -m "feat(ext): add email to Workday FIELD_NAME_MAP"
```

---

### Task 3: Content script — profileFill handler

**Files:**
- Modify: `packages/extension/src/content/workday.ts`

- [ ] **Step 1: Add profileFill message handling**

Replace the entire content of `packages/extension/src/content/workday.ts` with:

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

    if (msg.cmd === 'profileFill') {
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
          const ok = fill(field.element, value)
          if (ok) filled++
          else skipped++
        }

        sendResponse({ ok: true, data: { filled, skipped, total: fields.length } })
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

Key changes from P6:
- New `ProfileFillMessage` type with `values: Record<string, string>`
- New `profileFill` handler that fills with mapped values instead of hardcoded `FORGE-{kind}`
- Skips fields with `unknown` kind or no matching value
- Returns `{ filled, skipped, total }` instead of `{ filled, failed, total }`
- Existing `testFill` handler preserved for dev use

- [ ] **Step 2: Run existing Workday tests**

Run: `cd packages/extension && bun test tests/plugins/workday.test.ts`
Expected: All existing tests pass (tests exercise the plugin, not the content script).

- [ ] **Step 3: Commit**

```bash
git add packages/extension/src/content/workday.ts
git commit -m "feat(ext): profileFill content script handler — fill with mapped profile values"
```

---

### Task 4: Messaging + background handler for profileFill

**Files:**
- Modify: `packages/extension/src/lib/messaging.ts`
- Create: `packages/extension/src/background/handlers/autofill.ts`
- Modify: `packages/extension/src/background/index.ts`

- [ ] **Step 1: Add `form.profileFill` to Command union**

In `packages/extension/src/lib/messaging.ts`, add the new command:

```typescript
export type Command =
  | { cmd: 'health' }
  | { cmd: 'orgs.list'; limit?: number }
  | { cmd: 'orgs.create'; payload: { name: string } }
  | { cmd: 'form.testFill' }
  | { cmd: 'form.profileFill' }
  | { cmd: 'jd.captureActive' }
```

- [ ] **Step 2: Create the autofill handler**

Create `packages/extension/src/background/handlers/autofill.ts`:

```typescript
// packages/extension/src/background/handlers/autofill.ts

import type { Response } from '../../lib/messaging'
import { extError, mapSdkError, mapNetworkError } from '../../lib/errors'
import { buildProfileFieldMap } from '../../lib/profile-map'
import { getClient } from '../client'

export interface ProfileFillPayload {
  filled: number
  skipped: number
  total: number
}

export async function handleProfileFill(): Promise<Response<ProfileFillPayload>> {
  try {
    // 1. Get active tab
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
    if (!tab?.id) {
      return {
        ok: false,
        error: extError('UNKNOWN_ERROR', 'No active tab', { layer: 'background' }),
      }
    }

    // 2. Load profile from Forge
    const client = await getClient()
    const profileResult = await client.profile.get()
    if (!profileResult.ok) {
      return {
        ok: false,
        error: extError('PROFILE_NOT_AVAILABLE', 'Profile not loaded from Forge', {
          layer: 'sdk',
          context: { sdk_code: profileResult.error.code },
        }),
      }
    }

    // 3. Build FieldKind → value map
    const values = buildProfileFieldMap(profileResult.data)
    if (Object.keys(values).length === 0) {
      return {
        ok: false,
        error: extError('PROFILE_NOT_AVAILABLE', 'Profile has no fillable fields', {
          layer: 'background',
        }),
      }
    }

    // 4. Inject Workday content script
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ['content/workday.js'],
    })

    // 5. Send profile values to content script for filling
    const response = await chrome.tabs.sendMessage(tab.id, {
      cmd: 'profileFill',
      values,
    })

    if (response?.ok) {
      return { ok: true, data: response.data as ProfileFillPayload }
    }

    return {
      ok: false,
      error: response?.error ?? extError('FORM_NOT_DETECTED', 'Unexpected response from content script', {
        layer: 'content',
      }),
    }
  } catch (err) {
    return { ok: false, error: mapNetworkError(err, { url: '/api/profile' }) }
  }
}
```

- [ ] **Step 3: Wire into background router**

In `packages/extension/src/background/index.ts`, add the import:

```typescript
import { handleProfileFill } from './handlers/autofill'
```

Add the case in the switch before `default`:

```typescript
case 'form.profileFill':
  response = await handleProfileFill()
  break
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd packages/extension && npx tsc --noEmit`
Expected: No new errors.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/lib/messaging.ts packages/extension/src/background/handlers/autofill.ts packages/extension/src/background/index.ts
git commit -m "feat(ext): form.profileFill handler — load profile, map fields, autofill"
```

---

### Task 5: Popup — replace Test Fill with Autofill

**Files:**
- Modify: `packages/extension/src/popup/Popup.svelte`

- [ ] **Step 1: Add ProfileFillPayload import and autofill function**

Add the import alongside the existing type imports:

```typescript
import type { ProfileFillPayload } from '../background/handlers/autofill'
```

Replace the `testFill()` function with:

```typescript
async function autofill() {
  status = 'Loading profile...'
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

  const response = await sendCommand<ProfileFillPayload>({ cmd: 'form.profileFill' })
  if (response.ok) {
    const { filled, skipped, total } = response.data
    status = `Filled ${filled}/${total} fields` + (skipped > 0 ? ` (${skipped} need answer bank)` : '')
    statusKind = filled > 0 ? 'ok' : 'info'
  } else {
    const code = response.error.code
    if (code === 'FORM_NOT_DETECTED') {
      status = 'No form fields found on this page'
    } else if (code === 'PROFILE_NOT_AVAILABLE') {
      status = 'Profile not loaded from Forge'
    } else if (code === 'API_UNREACHABLE') {
      status = 'Forge is not running'
    } else {
      status = `Error: ${response.error.message}`
    }
    statusKind = 'err'
  }
}
```

- [ ] **Step 2: Update button in template**

Replace:
```svelte
<button onclick={testFill} class="secondary">Test Fill</button>
```

With:
```svelte
<button onclick={autofill} class="secondary">Autofill</button>
```

- [ ] **Step 3: Remove unused TestFillPayload import**

Remove the `TestFillPayload` import from the `import type { TestFillPayload } from '../background/handlers/form'` line. If this is the only import from that module, remove the entire line.

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd packages/extension && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/popup/Popup.svelte
git commit -m "feat(ext): Autofill button replaces Test Fill — fills from Forge profile"
```

---

### Task 6: Smoke test for profile.get()

**Files:**
- Modify: `packages/extension/tests/background/smoke.test.ts`

- [ ] **Step 1: Add profile smoke test**

Add this test inside the existing `describe('smoke: extension <-> Forge roundtrip', ...)` block:

```typescript
test('sdk.profile.get returns user profile', async () => {
  if (!serverReachable) return
  const client = await getClient()
  const result = await client.profile.get()
  expect(result.ok).toBe(true)
  if (result.ok) {
    expect(typeof result.data.name).toBe('string')
    expect(result.data.name.length).toBeGreaterThan(0)
  }
})
```

- [ ] **Step 2: Run smoke tests**

Run: `cd packages/extension && bun test tests/background/smoke.test.ts`
Expected: All smoke tests pass if Forge server is running.

- [ ] **Step 3: Commit**

```bash
git add packages/extension/tests/background/smoke.test.ts
git commit -m "test(ext): smoke test for profile.get (P7)"
```

---

### Task 7: Version bump + build verification

**Files:**
- Modify: `packages/extension/manifest.json`

- [ ] **Step 1: Bump version to 0.0.9**

In `packages/extension/manifest.json`, change:
```json
"version": "0.0.9",
```

- [ ] **Step 2: Full build**

Run: `cd packages/extension && bun run build`
Expected: Clean build, no errors. Content script `content/workday.js` is self-contained (no `import` statement — verify with `head -c 100 dist/content/workday.js`).

- [ ] **Step 3: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add packages/extension/manifest.json
git commit -m "chore(ext): bump version to 0.0.9 (P7)"
```

- [ ] **Step 5: Rebuild on main after merge**

After merging to main:
```bash
cd packages/extension && bun run build
```
Verify `dist/manifest.json` shows version `0.0.9`.

- [ ] **Step 6: Verification checklist (human sign-off required)**

- [ ] `bun run build` succeeds
- [ ] Extension loads in Chrome, shows v0.0.9
- [ ] On a Workday application form: click "Autofill"
- [ ] Name (first + last) fills with real profile values
- [ ] Email fills with real profile value
- [ ] Phone fills with real profile value
- [ ] Toast shows "Filled X/Y fields (Z need answer bank)"
- [ ] EEO/work-auth fields detected but NOT filled (correct — answer bank is MVP)
- [ ] Partial fill count is accurate
- [ ] "Capture Job" and other buttons still work
- [ ] Forge offline → "Autofill" → "Profile not loaded from Forge" or "Forge is not running"
