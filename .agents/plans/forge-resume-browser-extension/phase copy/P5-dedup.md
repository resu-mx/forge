# P5: Extraction + Dedup → API Write — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add URL dedup and org resolution to the capture flow — prevent duplicate JDs and link captured jobs to their organizations.

**Architecture:** After P4's extraction+validation, the handler now normalizes the URL (via plugin), checks if the JD already exists (via `lookupByUrl`), resolves the org (search existing → exact match or create), then creates the JD with the org linked. LinkedIn plugin gets a `normalizeUrl` capability that strips tracking params.

**Tech Stack:** Chrome MV3 extension, `@forge/sdk`, Bun test runner

**Worktree:** `.claude/worktrees/forge-ext-p5-dedup/` on branch `feat/forge-ext/p5-dedup`

**SPEC reference:** SPEC.md §4 P5

---

### File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `packages/sdk/src/types.ts` | Add `search` to `OrganizationFilter` |
| Modify | `packages/extension/src/plugin/plugins/linkedin.ts` | Add `normalizeUrl` capability |
| Modify | `packages/extension/tests/plugins/linkedin.test.ts` | Tests for `normalizeUrl` |
| Create | `packages/extension/src/lib/resolve-org.ts` | `resolveOrganization()` — search/match/create |
| Create | `packages/extension/tests/lib/resolve-org.test.ts` | Unit tests for org resolution |
| Modify | `packages/extension/src/background/handlers/capture.ts` | Add dedup + org resolution steps |
| Modify | `packages/extension/src/popup/Popup.svelte` | Update dedup toast message |
| Modify | `packages/extension/tests/background/smoke.test.ts` | Smoke tests for dedup + org |
| Modify | `packages/extension/manifest.json` | Version bump to 0.0.8 |

---

### Task 1: SDK — add `search` to OrganizationFilter

**Files:**
- Modify: `packages/sdk/src/types.ts`

The Forge API supports `?search=` on `GET /api/organizations` (name + alias LIKE search), but the SDK's `OrganizationFilter` type doesn't include it.

- [ ] **Step 1: Add search field**

In `packages/sdk/src/types.ts`, find the `OrganizationFilter` interface and add `search`:

```typescript
export interface OrganizationFilter {
  org_type?: string
  tag?: string
  worked?: string
  status?: string
  search?: string
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/sdk/src/types.ts
git commit -m "feat(sdk): add search to OrganizationFilter type"
```

---

### Task 2: LinkedIn normalizeUrl (TDD)

**Files:**
- Modify: `packages/extension/src/plugin/plugins/linkedin.ts`
- Modify: `packages/extension/tests/plugins/linkedin.test.ts`

- [ ] **Step 1: Write the failing tests**

Add a new describe block to `packages/extension/tests/plugins/linkedin.test.ts`:

```typescript
describe('linkedinPlugin.normalizeUrl', () => {
  const normalize = linkedinPlugin.capabilities.normalizeUrl

  test('normalizeUrl capability exists', () => {
    expect(normalize).toBeDefined()
  })

  test('strips query params', () => {
    expect(normalize!('https://www.linkedin.com/jobs/view/4366558926/?utm_source=google&trackingId=abc'))
      .toBe('https://www.linkedin.com/jobs/view/4366558926/')
  })

  test('strips hash fragment', () => {
    expect(normalize!('https://www.linkedin.com/jobs/view/4366558926/#details'))
      .toBe('https://www.linkedin.com/jobs/view/4366558926/')
  })

  test('adds trailing slash if missing', () => {
    expect(normalize!('https://www.linkedin.com/jobs/view/4366558926'))
      .toBe('https://www.linkedin.com/jobs/view/4366558926/')
  })

  test('preserves already-clean URL', () => {
    expect(normalize!('https://www.linkedin.com/jobs/view/4366558926/'))
      .toBe('https://www.linkedin.com/jobs/view/4366558926/')
  })

  test('returns input unchanged for non-URL strings', () => {
    expect(normalize!('not-a-url')).toBe('not-a-url')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/extension && bun test tests/plugins/linkedin.test.ts`
Expected: FAIL — `normalize` is undefined (capability not defined yet).

- [ ] **Step 3: Implement normalizeUrl**

In `packages/extension/src/plugin/plugins/linkedin.ts`, add the function before the plugin export:

```typescript
function normalizeUrl(url: string): string {
  try {
    const parsed = new URL(url)
    parsed.search = ''
    parsed.hash = ''
    if (!parsed.pathname.endsWith('/')) {
      parsed.pathname += '/'
    }
    return parsed.toString()
  } catch {
    return url
  }
}
```

Then add it to the plugin's capabilities:

```typescript
export const linkedinPlugin: JobBoardPlugin = {
  name: PLUGIN_NAME,
  matches: ['linkedin.com', '*.linkedin.com'],
  capabilities: {
    extractJD,
    normalizeUrl,
  },
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/extension && bun test tests/plugins/linkedin.test.ts`
Expected: All tests pass (existing extraction tests + 6 new normalizeUrl tests).

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/plugin/plugins/linkedin.ts packages/extension/tests/plugins/linkedin.test.ts
git commit -m "feat(ext): LinkedIn normalizeUrl — strips tracking params, normalizes path"
```

---

### Task 3: Org resolution utility (TDD)

**Files:**
- Create: `packages/extension/src/lib/resolve-org.ts`
- Create: `packages/extension/tests/lib/resolve-org.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/extension/tests/lib/resolve-org.test.ts`:

```typescript
import { describe, test, expect } from 'bun:test'
import { resolveOrganization } from '../../src/lib/resolve-org'

// Minimal mock types matching SDK shape
interface MockOrg { id: string; name: string }
type MockListFn = (search: string) => Promise<{ ok: true; data: MockOrg[] } | { ok: false }>
type MockCreateFn = (name: string) => Promise<{ ok: true; data: MockOrg } | { ok: false }>

function mockList(orgs: MockOrg[]): MockListFn {
  return async () => ({ ok: true as const, data: orgs })
}

function mockListFail(): MockListFn {
  return async () => ({ ok: false as const })
}

function mockCreate(id: string): MockCreateFn {
  return async (name) => ({ ok: true as const, data: { id, name } })
}

function mockCreateFail(): MockCreateFn {
  return async () => ({ ok: false as const })
}

describe('resolveOrganization', () => {
  test('returns null when companyName is null', async () => {
    const result = await resolveOrganization(null, mockList([]), mockCreate('new-id'))
    expect(result).toBeNull()
  })

  test('returns null when companyName is empty', async () => {
    const result = await resolveOrganization('', mockList([]), mockCreate('new-id'))
    expect(result).toBeNull()
  })

  test('returns org_id on exact name match (case-insensitive)', async () => {
    const orgs = [{ id: 'org-123', name: 'Anthropic' }]
    const result = await resolveOrganization('anthropic', mockList(orgs), mockCreate('new-id'))
    expect(result).toBe('org-123')
  })

  test('creates org when search returns no results', async () => {
    const result = await resolveOrganization('NewCorp', mockList([]), mockCreate('created-id'))
    expect(result).toBe('created-id')
  })

  test('returns first result when no exact match (prototype picks first)', async () => {
    const orgs = [
      { id: 'org-1', name: 'Anthropic AI' },
      { id: 'org-2', name: 'Anthropic Inc' },
    ]
    const result = await resolveOrganization('Anthropic', mockList(orgs), mockCreate('new-id'))
    expect(result).toBe('org-1')
  })

  test('returns null when search fails', async () => {
    const result = await resolveOrganization('Acme', mockListFail(), mockCreate('new-id'))
    expect(result).toBeNull()
  })

  test('returns null when create fails', async () => {
    const result = await resolveOrganization('NewCorp', mockList([]), mockCreateFail())
    expect(result).toBeNull()
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/extension && bun test tests/lib/resolve-org.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement resolveOrganization**

Create `packages/extension/src/lib/resolve-org.ts`:

```typescript
/**
 * Resolve a company name to an organization ID.
 *
 * Strategy (SPEC §4 P5):
 * 1. Search existing orgs by name
 * 2. If exact name match (case-insensitive) → use that org
 * 3. If no results → create new org
 * 4. If partial matches but no exact → pick first (prototype; MVP disambiguates)
 *
 * Returns null if companyName is empty or if both search and create fail.
 */
export async function resolveOrganization(
  companyName: string | null,
  searchOrgs: (search: string) => Promise<{ ok: boolean; data?: { id: string; name: string }[] }>,
  createOrg: (name: string) => Promise<{ ok: boolean; data?: { id: string; name: string } }>,
): Promise<string | null> {
  if (!companyName?.trim()) return null

  const searchResult = await searchOrgs(companyName)
  if (!searchResult.ok || !searchResult.data) return null

  // Exact match (case-insensitive)
  const exact = searchResult.data.find(
    (org) => org.name.toLowerCase() === companyName.toLowerCase(),
  )
  if (exact) return exact.id

  // No results → create
  if (searchResult.data.length === 0) {
    const createResult = await createOrg(companyName)
    return createResult.ok && createResult.data ? createResult.data.id : null
  }

  // Partial matches → prototype picks first
  return searchResult.data[0].id
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/extension && bun test tests/lib/resolve-org.test.ts`
Expected: 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/src/lib/resolve-org.ts packages/extension/tests/lib/resolve-org.test.ts
git commit -m "feat(ext): resolveOrganization — search, match, or create org"
```

---

### Task 4: Update capture handler with dedup + org resolution

**Files:**
- Modify: `packages/extension/src/background/handlers/capture.ts`

- [ ] **Step 1: Replace the capture handler**

Replace the entire content of `packages/extension/src/background/handlers/capture.ts` with:

```typescript
// packages/extension/src/background/handlers/capture.ts

import type { Response } from '../../lib/messaging'
import { extError, mapSdkError, mapNetworkError } from '../../lib/errors'
import { validateExtraction } from '../../lib/validate'
import { resolveOrganization } from '../../lib/resolve-org'
import { getClient } from '../client'
import { linkedinPlugin } from '../../plugin/plugins/linkedin'

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

    // 2. Check host — only LinkedIn supported
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

    const client = await getClient()

    // 5. Normalize URL and check for existing JD (dedup)
    const normalize = linkedinPlugin.capabilities.normalizeUrl
    const canonicalUrl = normalize ? normalize(extracted.url) : extracted.url

    const lookupResult = await client.jobDescriptions.lookupByUrl(canonicalUrl)
    if (lookupResult.ok) {
      return {
        ok: false,
        error: extError('API_DUPLICATE', 'Job already captured', {
          layer: 'background',
          url: canonicalUrl,
          context: { existing_id: lookupResult.data.id, existing_title: lookupResult.data.title },
        }),
      }
    }
    // NOT_FOUND means no duplicate — continue with creation
    // Any other error (network, server) — skip dedup silently, try to create

    // 6. Resolve organization (search existing or create new)
    const organizationId = await resolveOrganization(
      extracted.company,
      async (search) => {
        const result = await client.organizations.list({ search, limit: 5 })
        return result.ok
          ? { ok: true, data: result.data }
          : { ok: false }
      },
      async (name) => {
        const result = await client.organizations.create({ name })
        return result.ok
          ? { ok: true, data: { id: result.data.id, name: result.data.name } }
          : { ok: false }
      },
    )

    // 7. Create JD in Forge
    const result = await client.jobDescriptions.create({
      title: extracted.title!,
      raw_text: extracted.description!,
      url: canonicalUrl,
      location: extracted.location ?? undefined,
      salary_range: extracted.salary_range ?? undefined,
      organization_id: organizationId ?? undefined,
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

Key changes from P4:
- Import `resolveOrganization` and `linkedinPlugin`
- Step 5: normalize URL and call `lookupByUrl` for dedup
- Step 6: resolve org via `resolveOrganization` with adapter callbacks
- Step 7: pass `organization_id` and `canonicalUrl` to create

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd packages/extension && npx tsc --noEmit`
Expected: No new errors.

- [ ] **Step 3: Run all tests**

Run: `cd packages/extension && bun test`
Expected: All existing tests pass (the handler isn't unit-tested directly — chrome APIs needed).

- [ ] **Step 4: Commit**

```bash
git add packages/extension/src/background/handlers/capture.ts
git commit -m "feat(ext): capture handler with URL dedup + org resolution (P5)"
```

---

### Task 5: Update popup toast for dedup

**Files:**
- Modify: `packages/extension/src/popup/Popup.svelte`

- [ ] **Step 1: Improve the API_DUPLICATE toast message**

In `packages/extension/src/popup/Popup.svelte`, find the `captureJob` function's error handling. The current `API_DUPLICATE` case (line ~52) shows "Job already captured". Update it to include the existing JD ID from the error context:

Replace:
```typescript
    } else if (code === 'API_DUPLICATE') {
      status = 'Job already captured'
```

With:
```typescript
    } else if (code === 'API_DUPLICATE') {
      const existingId = response.error.context?.existing_id
      status = existingId
        ? `Already captured (${String(existingId).slice(0, 8)}...)`
        : 'Job already captured'
```

- [ ] **Step 2: Commit**

```bash
git add packages/extension/src/popup/Popup.svelte
git commit -m "feat(ext): dedup toast shows existing JD ID"
```

---

### Task 6: Smoke tests for dedup + org resolution

**Files:**
- Modify: `packages/extension/tests/background/smoke.test.ts`

- [ ] **Step 1: Add dedup and org smoke tests**

Add these tests inside the existing `describe('smoke: extension <-> Forge roundtrip', ...)` block in `packages/extension/tests/background/smoke.test.ts`:

```typescript
test('sdk.jobDescriptions.lookupByUrl finds existing JD', async () => {
  if (!serverReachable) return
  const client = await getClient()
  const uniqueUrl = 'https://www.linkedin.com/jobs/view/smoke-dedup-' + Date.now() + '/'

  // Create a JD first
  const createResult = await client.jobDescriptions.create({
    title: 'Dedup Test JD',
    raw_text: 'Testing dedup flow.',
    url: uniqueUrl,
  })
  expect(createResult.ok).toBe(true)

  // Lookup should find it
  const lookupResult = await client.jobDescriptions.lookupByUrl(uniqueUrl)
  expect(lookupResult.ok).toBe(true)
  if (lookupResult.ok) {
    expect(lookupResult.data.title).toBe('Dedup Test JD')
  }
})

test('sdk.jobDescriptions.lookupByUrl returns not-found for unknown URL', async () => {
  if (!serverReachable) return
  const client = await getClient()
  const result = await client.jobDescriptions.lookupByUrl('https://example.com/nonexistent-' + Date.now())
  expect(result.ok).toBe(false)
})

test('sdk.organizations.list with search finds org by name', async () => {
  if (!serverReachable) return
  const client = await getClient()
  const uniqueName = 'SmokeCorp ' + Date.now()

  // Create an org first
  const createResult = await client.organizations.create({ name: uniqueName })
  expect(createResult.ok).toBe(true)

  // Search should find it
  const searchResult = await client.organizations.list({ search: uniqueName, limit: 5 })
  expect(searchResult.ok).toBe(true)
  if (searchResult.ok) {
    expect(searchResult.data.length).toBeGreaterThanOrEqual(1)
    expect(searchResult.data[0].name).toBe(uniqueName)
  }
})
```

- [ ] **Step 2: Run smoke tests**

Run: `cd packages/extension && bun test tests/background/smoke.test.ts`
Expected: All smoke tests pass if Forge server is running.

- [ ] **Step 3: Commit**

```bash
git add packages/extension/tests/background/smoke.test.ts
git commit -m "test(ext): smoke tests for dedup lookupByUrl + org search (P5)"
```

---

### Task 7: Version bump + build verification

**Files:**
- Modify: `packages/extension/manifest.json`

- [ ] **Step 1: Bump version to 0.0.8**

In `packages/extension/manifest.json`, change:
```json
"version": "0.0.8",
```

- [ ] **Step 2: Full build**

Run: `cd packages/extension && bun run build`
Expected: Clean build, no errors.

- [ ] **Step 3: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add packages/extension/manifest.json
git commit -m "chore(ext): bump version to 0.0.8 (P5)"
```

- [ ] **Step 5: Verification checklist (human sign-off required)**

- [ ] `bun run build` succeeds
- [ ] Extension loads in Chrome via "Load unpacked" → `dist/`
- [ ] On LinkedIn job page (first capture): "Capture Job" → "JD created: \<id\>"
- [ ] Created JD visible in Forge webui with correct title, description, url (query params stripped), location
- [ ] Created JD linked to correct organization (existing or newly created)
- [ ] On same LinkedIn page (second capture): "Capture Job" → "Already captured (\<id\>...)"
- [ ] On a different LinkedIn job with known company: org reused (not duplicated)
- [ ] On LinkedIn job with new company: new org created and linked
- [ ] URL normalization: `?utm_source=...` stripped from stored URL
- [ ] All other buttons still work
