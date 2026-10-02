# M6 — Page Overlay + Answer Bank + App Questions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a confidence-gated review overlay to the capture flow, an answer bank for EEO/work-auth fields with WebUI settings pages, and Workday app question filling from the answer bank.

**Architecture:** Three layers — (1) Forge backend adds `answer_bank` table + API + SDK resource, (2) extension adds confidence model + shadow DOM overlay + toast to the content script capture flow, (3) Workday plugin extends field detection to EEO/work-auth and fills from answer bank via background handler. The overlay is injected by the content script (not the popup) and all three capture entry points converge to the same flow.

**Tech Stack:** Bun/Hono (API), SQLite (DB), TypeScript SDK, Svelte (WebUI), Vite (extension build), shadow DOM (overlay), chrome.storage.local (confidence config)

**Spec:** `.claude/plans/forge-resume-browser-extension/phase/M6-overlay-answer-bank.md`

---

## File Map

### New files

| File | Responsibility |
|------|---------------|
| `packages/core/src/db/migrations/050_answer_bank.sql` | Create `answer_bank` table |
| `packages/core/src/services/answer-bank-service.ts` | CRUD + upsert for answer bank entries |
| `packages/core/src/routes/answer-bank.ts` | HTTP routes: GET/PUT/DELETE `/api/profile/answers` |
| `packages/core/src/routes/__tests__/answer-bank.test.ts` | Route integration tests |
| `packages/sdk/src/resources/answer-bank.ts` | SDK resource class |
| `packages/sdk/src/__tests__/answer-bank.test.ts` | SDK resource unit tests |
| `packages/webui/src/routes/settings/work-auth/+page.svelte` | Work authorization settings form |
| `packages/webui/src/routes/settings/eeo/+page.svelte` | EEO voluntary disclosure settings form |
| `packages/extension/src/lib/confidence.ts` | Confidence tier model, decision gate, mode config |
| `packages/extension/src/content/overlay.ts` | Shadow DOM side panel for extraction review |
| `packages/extension/src/content/toast.ts` | Toast notification for quiet-mode captures |
| `packages/extension/tests/lib/confidence.test.ts` | Confidence model unit tests |

### Modified files

| File | Changes |
|------|---------|
| `packages/core/src/services/index.ts` | Add `AnswerBankService` to `Services` interface + `createServices()` |
| `packages/core/src/routes/server.ts` | Import + mount `answerBankRoutes` |
| `packages/sdk/src/types.ts` | Add `AnswerBankEntry` type |
| `packages/sdk/src/client.ts` | Import + wire `AnswerBankResource`, add `answerBank` property |
| `packages/sdk/src/index.ts` | Re-export answer bank types |
| `packages/extension/src/plugin/types.ts` | Add `ConfidenceTier`, `FieldConfidence`, `EnrichedExtraction` |
| `packages/extension/src/lib/enrich-extraction.ts` | Return `EnrichedExtraction` with confidence scores |
| `packages/extension/src/content/linkedin.ts` | Decision gate → overlay or toast after extraction |
| `packages/extension/src/content/inject-button.ts` | Support Shift+click for manual overlay |
| `packages/extension/src/background/handlers/capture.ts` | Add `handleSubmitExtracted` for pre-edited payloads |
| `packages/extension/src/background/handlers/autofill.ts` | Merge answer bank entries into fill values |
| `packages/extension/src/background/index.ts` | New commands: `jd.submitExtracted`, `answers.list` |
| `packages/extension/src/lib/messaging.ts` | New Command variants |
| `packages/extension/src/plugin/plugins/workday.ts` | EEO/work-auth entries in `FIELD_NAME_MAP` |
| `packages/extension/src/popup/Popup.svelte` | Update capture to pass forceManual flag |
| `packages/extension/vite.config.ts` | Add overlay entry point |
| `packages/extension/manifest.json` | Version bump to 0.1.6 |
| `packages/extension/manifest.firefox.json` | Version bump to 0.1.6 |

---

## Task 1: Answer Bank Migration

**Files:**
- Create: `packages/core/src/db/migrations/050_answer_bank.sql`

- [ ] **Step 1: Write the migration**

```sql
-- 050_answer_bank.sql
-- Answer bank: reusable answers for EEO/work-auth form fields.
-- field_kind is UNIQUE — one answer per FieldKind.

CREATE TABLE IF NOT EXISTS answer_bank (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6)))),
  field_kind TEXT NOT NULL UNIQUE,
  label TEXT NOT NULL,
  value TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

- [ ] **Step 2: Verify migration applies**

Run: `cd packages/core && bun test src/db/__tests__/migrate.test.ts`
Expected: PASS (migration runner picks up new file)

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/db/migrations/050_answer_bank.sql
git commit -m "feat(core): add answer_bank migration (M6)"
```

---

## Task 2: Answer Bank Service

**Files:**
- Create: `packages/core/src/services/answer-bank-service.ts`
- Modify: `packages/core/src/services/index.ts`

- [ ] **Step 1: Write AnswerBankService**

```ts
// packages/core/src/services/answer-bank-service.ts

import { storageErrorToForgeError } from '../storage/error-mapper'
import type { EntityLifecycleManager } from '../storage/lifecycle-manager'

export interface AnswerBankEntry {
  id: string
  field_kind: string
  label: string
  value: string
  created_at: string
  updated_at: string
}

interface UpsertInput {
  field_kind: string
  label: string
  value: string
}

type Result<T> = { ok: true; data: T } | { ok: false; error: { code: string; message: string } }

export class AnswerBankService {
  constructor(private readonly elm: EntityLifecycleManager) {}

  async list(): Promise<Result<AnswerBankEntry[]>> {
    const result = await this.elm.list('answer_bank', {
      orderBy: [{ field: 'field_kind', direction: 'asc' }],
    })
    if (!result.ok) {
      return { ok: false, error: storageErrorToForgeError(result.error) }
    }
    return { ok: true, data: result.value.rows as unknown as AnswerBankEntry[] }
  }

  async upsert(input: UpsertInput): Promise<Result<AnswerBankEntry>> {
    if (!input.field_kind?.trim()) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'field_kind is required' } }
    }
    if (!input.label?.trim()) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'label is required' } }
    }
    if (input.value == null) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'value is required' } }
    }

    // Check if entry exists for this field_kind
    const existing = await this.elm.list('answer_bank', {
      where: { field_kind: input.field_kind },
      limit: 1,
    })

    if (existing.ok && existing.value.rows.length > 0) {
      const row = existing.value.rows[0] as unknown as AnswerBankEntry
      const updateResult = await this.elm.update('answer_bank', row.id, {
        label: input.label,
        value: input.value,
        updated_at: new Date().toISOString().replace('T', ' ').replace('Z', ''),
      })
      if (!updateResult.ok) {
        return { ok: false, error: storageErrorToForgeError(updateResult.error) }
      }
      const refreshed = await this.elm.get('answer_bank', row.id)
      if (!refreshed.ok) {
        return { ok: false, error: storageErrorToForgeError(refreshed.error) }
      }
      return { ok: true, data: refreshed.value as unknown as AnswerBankEntry }
    }

    // Create new entry
    const id = crypto.randomUUID()
    const createResult = await this.elm.create('answer_bank', {
      id,
      field_kind: input.field_kind,
      label: input.label,
      value: input.value,
    })
    if (!createResult.ok) {
      return { ok: false, error: storageErrorToForgeError(createResult.error) }
    }
    const created = await this.elm.get('answer_bank', id)
    if (!created.ok) {
      return { ok: false, error: storageErrorToForgeError(created.error) }
    }
    return { ok: true, data: created.value as unknown as AnswerBankEntry }
  }

  async delete(fieldKind: string): Promise<Result<void>> {
    const existing = await this.elm.list('answer_bank', {
      where: { field_kind: fieldKind },
      limit: 1,
    })
    if (!existing.ok || existing.value.rows.length === 0) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `No answer for field_kind: ${fieldKind}` } }
    }
    const row = existing.value.rows[0] as unknown as AnswerBankEntry
    const deleteResult = await this.elm.delete('answer_bank', row.id)
    if (!deleteResult.ok) {
      return { ok: false, error: storageErrorToForgeError(deleteResult.error) }
    }
    return { ok: true, data: undefined }
  }
}
```

- [ ] **Step 2: Register in services index**

In `packages/core/src/services/index.ts`, add the import and wire it:

Add import:
```ts
import { AnswerBankService } from './answer-bank-service'
```

Add to `Services` interface:
```ts
answerBank: AnswerBankService
```

Add to `createServices()` return:
```ts
answerBank: new AnswerBankService(elm),
```

Add re-export at bottom:
```ts
export { AnswerBankService } from './answer-bank-service'
```

- [ ] **Step 3: Add answer_bank to entity map**

The ELM needs `answer_bank` registered. Check `packages/core/src/storage/entity-map.ts` for the pattern and add an entry for `answer_bank` with columns: `id`, `field_kind`, `label`, `value`, `created_at`, `updated_at`. The `field_kind` column should have `unique: true`.

- [ ] **Step 4: Run existing tests to verify no breakage**

Run: `cd packages/core && bun test`
Expected: All existing tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/services/answer-bank-service.ts packages/core/src/services/index.ts packages/core/src/storage/entity-map.ts
git commit -m "feat(core): add AnswerBankService with list/upsert/delete (M6)"
```

---

## Task 3: Answer Bank API Routes + Tests

**Files:**
- Create: `packages/core/src/routes/answer-bank.ts`
- Create: `packages/core/src/routes/__tests__/answer-bank.test.ts`
- Modify: `packages/core/src/routes/server.ts`

- [ ] **Step 1: Write route tests**

```ts
// packages/core/src/routes/__tests__/answer-bank.test.ts

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'

describe('Answer Bank routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // -- GET /profile/answers ---------------------------------------------------

  test('GET /profile/answers returns empty array initially', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/profile/answers')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data).toEqual([])
  })

  // -- PUT /profile/answers ---------------------------------------------------

  test('PUT /profile/answers creates a new entry', async () => {
    const res = await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'work_auth.us',
      label: 'Authorized to work in US',
      value: 'Yes',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.field_kind).toBe('work_auth.us')
    expect(body.data.label).toBe('Authorized to work in US')
    expect(body.data.value).toBe('Yes')
    expect(body.data.id).toBeDefined()
  })

  test('PUT /profile/answers upserts existing entry', async () => {
    await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'work_auth.us',
      label: 'Authorized to work in US',
      value: 'Yes',
    })

    const res = await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'work_auth.us',
      label: 'Authorized to work in US',
      value: 'No',
    })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.value).toBe('No')

    // Verify only one entry exists
    const listRes = await apiRequest(ctx.app, 'GET', '/profile/answers')
    const listBody = await listRes.json()
    expect(listBody.data).toHaveLength(1)
  })

  test('PUT /profile/answers returns 400 for missing field_kind', async () => {
    const res = await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      label: 'Test',
      value: 'Yes',
    })
    expect(res.status).toBe(400)
  })

  test('PUT /profile/answers returns 400 for missing label', async () => {
    const res = await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'work_auth.us',
      value: 'Yes',
    })
    expect(res.status).toBe(400)
  })

  // -- DELETE /profile/answers/:field_kind ------------------------------------

  test('DELETE /profile/answers/:field_kind removes entry', async () => {
    await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'eeo.gender',
      label: 'Gender',
      value: 'Male',
    })

    const res = await apiRequest(ctx.app, 'DELETE', '/profile/answers/eeo.gender')
    expect(res.status).toBe(204)

    const listRes = await apiRequest(ctx.app, 'GET', '/profile/answers')
    const listBody = await listRes.json()
    expect(listBody.data).toHaveLength(0)
  })

  test('DELETE /profile/answers/:field_kind returns 404 for unknown kind', async () => {
    const res = await apiRequest(ctx.app, 'DELETE', '/profile/answers/nonexistent')
    expect(res.status).toBe(404)
  })

  // -- Integration: multiple entries ------------------------------------------

  test('full CRUD lifecycle', async () => {
    // Create several entries
    await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'work_auth.us', label: 'US Work Auth', value: 'Yes',
    })
    await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'work_auth.sponsorship', label: 'Sponsorship', value: 'No',
    })
    await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'eeo.gender', label: 'Gender', value: 'Male',
    })

    // List all
    const listRes = await apiRequest(ctx.app, 'GET', '/profile/answers')
    const listBody = await listRes.json()
    expect(listBody.data).toHaveLength(3)
    // Sorted by field_kind
    expect(listBody.data[0].field_kind).toBe('eeo.gender')
    expect(listBody.data[1].field_kind).toBe('work_auth.sponsorship')
    expect(listBody.data[2].field_kind).toBe('work_auth.us')

    // Update one
    await apiRequest(ctx.app, 'PUT', '/profile/answers', {
      field_kind: 'eeo.gender', label: 'Gender', value: 'Non-binary',
    })

    // Delete one
    await apiRequest(ctx.app, 'DELETE', '/profile/answers/work_auth.sponsorship')

    // Final list
    const finalRes = await apiRequest(ctx.app, 'GET', '/profile/answers')
    const finalBody = await finalRes.json()
    expect(finalBody.data).toHaveLength(2)
    expect(finalBody.data[0].field_kind).toBe('eeo.gender')
    expect(finalBody.data[0].value).toBe('Non-binary')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/routes/__tests__/answer-bank.test.ts`
Expected: FAIL (routes not mounted yet)

- [ ] **Step 3: Write the route handler**

```ts
// packages/core/src/routes/answer-bank.ts

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './status-codes'

export function answerBankRoutes(services: Services) {
  const app = new Hono()

  app.get('/profile/answers', async (c) => {
    const result = await services.answerBank.list()
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.put('/profile/answers', async (c) => {
    const body = await c.req.json()
    const result = await services.answerBank.upsert(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/profile/answers/:field_kind', async (c) => {
    const fieldKind = c.req.param('field_kind')
    const result = await services.answerBank.delete(fieldKind)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

- [ ] **Step 4: Mount routes in server.ts**

In `packages/core/src/routes/server.ts`:

Add import:
```ts
import { answerBankRoutes } from './answer-bank'
```

Add route mount after the `addressRoutes` line:
```ts
app.route('/', answerBankRoutes(services))
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd packages/core && bun test src/routes/__tests__/answer-bank.test.ts`
Expected: All PASS

- [ ] **Step 6: Run full core test suite**

Run: `cd packages/core && bun test`
Expected: All PASS

- [ ] **Step 7: Commit**

```bash
git add packages/core/src/routes/answer-bank.ts packages/core/src/routes/__tests__/answer-bank.test.ts packages/core/src/routes/server.ts
git commit -m "feat(core): add answer bank API routes with tests (M6)"
```

---

## Task 4: SDK Answer Bank Resource

**Files:**
- Create: `packages/sdk/src/resources/answer-bank.ts`
- Create: `packages/sdk/src/__tests__/answer-bank.test.ts`
- Modify: `packages/sdk/src/types.ts`
- Modify: `packages/sdk/src/client.ts`
- Modify: `packages/sdk/src/index.ts`

- [ ] **Step 1: Add AnswerBankEntry type to SDK types**

In `packages/sdk/src/types.ts`, add at the end:

```ts
// ---------------------------------------------------------------------------
// Answer Bank
// ---------------------------------------------------------------------------

export interface AnswerBankEntry {
  id: string
  field_kind: string
  label: string
  value: string
  created_at: string
  updated_at: string
}

export interface UpsertAnswer {
  field_kind: string
  label: string
  value: string
}
```

- [ ] **Step 2: Write SDK resource**

```ts
// packages/sdk/src/resources/answer-bank.ts

import type { AnswerBankEntry, UpsertAnswer, RequestFn, Result } from '../types'

export class AnswerBankResource {
  constructor(private request: RequestFn) {}

  /** List all answer bank entries. */
  list(): Promise<Result<AnswerBankEntry[]>> {
    return this.request<AnswerBankEntry[]>('GET', '/api/profile/answers')
  }

  /** Upsert an answer bank entry by field_kind. */
  upsert(data: UpsertAnswer): Promise<Result<AnswerBankEntry>> {
    return this.request<AnswerBankEntry>('PUT', '/api/profile/answers', data)
  }

  /** Delete an answer bank entry by field_kind. */
  delete(fieldKind: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/profile/answers/${encodeURIComponent(fieldKind)}`)
  }
}
```

- [ ] **Step 3: Wire into ForgeClient**

In `packages/sdk/src/client.ts`:

Add import:
```ts
import { AnswerBankResource } from './resources/answer-bank'
```

Add property declaration:
```ts
/** Answer bank for EEO/work-auth autofill values. */
public answerBank: AnswerBankResource
```

Add initialization in constructor:
```ts
this.answerBank = new AnswerBankResource(req)
```

- [ ] **Step 4: Re-export from index.ts**

In `packages/sdk/src/index.ts`, add to the re-exports:
```ts
export type { AnswerBankEntry, UpsertAnswer } from './types'
```

- [ ] **Step 5: Write SDK integration test**

```ts
// packages/sdk/src/__tests__/answer-bank.test.ts

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { ForgeClient } from '../client'
import { startTestServer, type TestServer } from './helpers'

describe('AnswerBankResource', () => {
  let server: TestServer
  let client: ForgeClient

  beforeEach(async () => {
    server = await startTestServer()
    client = new ForgeClient({ baseUrl: server.url })
  })

  afterEach(() => {
    server.close()
  })

  test('list returns empty initially', async () => {
    const result = await client.answerBank.list()
    expect(result.ok).toBe(true)
    if (result.ok) expect(result.data).toEqual([])
  })

  test('upsert creates and updates', async () => {
    const createResult = await client.answerBank.upsert({
      field_kind: 'work_auth.us',
      label: 'US Work Auth',
      value: 'Yes',
    })
    expect(createResult.ok).toBe(true)
    if (createResult.ok) {
      expect(createResult.data.field_kind).toBe('work_auth.us')
      expect(createResult.data.value).toBe('Yes')
    }

    const updateResult = await client.answerBank.upsert({
      field_kind: 'work_auth.us',
      label: 'US Work Auth',
      value: 'No',
    })
    expect(updateResult.ok).toBe(true)
    if (updateResult.ok) {
      expect(updateResult.data.value).toBe('No')
    }
  })

  test('delete removes entry', async () => {
    await client.answerBank.upsert({
      field_kind: 'eeo.gender',
      label: 'Gender',
      value: 'Male',
    })

    const deleteResult = await client.answerBank.delete('eeo.gender')
    expect(deleteResult.ok).toBe(true)

    const listResult = await client.answerBank.list()
    expect(listResult.ok).toBe(true)
    if (listResult.ok) expect(listResult.data).toHaveLength(0)
  })
})
```

- [ ] **Step 6: Run SDK tests**

Run: `cd packages/sdk && bun test`
Expected: All PASS

- [ ] **Step 7: Commit**

```bash
git add packages/sdk/src/resources/answer-bank.ts packages/sdk/src/__tests__/answer-bank.test.ts packages/sdk/src/types.ts packages/sdk/src/client.ts packages/sdk/src/index.ts
git commit -m "feat(sdk): add AnswerBankResource with list/upsert/delete (M6)"
```

---

## Task 5: Extension Confidence Model

**Files:**
- Create: `packages/extension/src/lib/confidence.ts`
- Create: `packages/extension/tests/lib/confidence.test.ts`
- Modify: `packages/extension/src/plugin/types.ts`

- [ ] **Step 1: Add confidence types to plugin/types.ts**

In `packages/extension/src/plugin/types.ts`, add at the end:

```ts
/** Confidence tiers for extracted field values. */
export type ConfidenceTier = 'high' | 'medium' | 'low' | 'absent'

/** Confidence metadata for a single extracted field. */
export interface FieldConfidence {
  field: string
  tier: ConfidenceTier
  source: string   // 'chip', 'selector', 'parser-body', 'missing'
}

/** An extraction enriched with confidence scores. */
export interface EnrichedExtraction {
  extracted: ExtractedJob
  confidence: FieldConfidence[]
}
```

- [ ] **Step 2: Write confidence test**

```ts
// packages/extension/tests/lib/confidence.test.ts

import { describe, test, expect } from 'bun:test'
import {
  CONFIDENCE_ORDER,
  DEFAULT_FLOORS,
  CONFIDENCE_MODES,
  shouldShowOverlay,
  assignConfidence,
} from '../../src/lib/confidence'
import type { ExtractedJob } from '../../src/plugin/types'

function makeExtracted(overrides: Partial<ExtractedJob> = {}): ExtractedJob {
  return {
    title: 'Senior Engineer',
    company: 'Acme',
    location: 'San Francisco, CA',
    salary_range: '$150k-$200k',
    description: 'Build things.',
    url: 'https://linkedin.com/jobs/123',
    extracted_at: new Date().toISOString(),
    source_plugin: 'linkedin',
    ...overrides,
  }
}

describe('CONFIDENCE_ORDER', () => {
  test('high > medium > low > absent', () => {
    expect(CONFIDENCE_ORDER.high).toBeGreaterThan(CONFIDENCE_ORDER.medium)
    expect(CONFIDENCE_ORDER.medium).toBeGreaterThan(CONFIDENCE_ORDER.low)
    expect(CONFIDENCE_ORDER.low).toBeGreaterThan(CONFIDENCE_ORDER.absent)
  })
})

describe('assignConfidence', () => {
  test('marks present chip-sourced fields as high', () => {
    const extracted = makeExtracted()
    const conf = assignConfidence(extracted, {
      title: { tier: 'high', source: 'selector' },
      company: { tier: 'high', source: 'selector' },
    })
    const titleConf = conf.find(c => c.field === 'title')
    expect(titleConf?.tier).toBe('high')
    expect(titleConf?.source).toBe('selector')
  })

  test('marks absent fields correctly', () => {
    const extracted = makeExtracted({ salary_range: null, salary_min: null, salary_max: null })
    const conf = assignConfidence(extracted, {})
    const salaryConf = conf.find(c => c.field === 'salary_min')
    expect(salaryConf?.tier).toBe('absent')
    expect(salaryConf?.source).toBe('missing')
  })

  test('marks parser-derived fields as medium', () => {
    const extracted = makeExtracted({
      salary_range: null,
      salary_min: 150000,
      salary_max: 200000,
    })
    const conf = assignConfidence(extracted, {
      salary_min: { tier: 'medium', source: 'parser-body' },
      salary_max: { tier: 'medium', source: 'parser-body' },
    })
    const salMinConf = conf.find(c => c.field === 'salary_min')
    expect(salMinConf?.tier).toBe('medium')
    expect(salMinConf?.source).toBe('parser-body')
  })
})

describe('shouldShowOverlay', () => {
  test('returns false when all fields meet default floors', () => {
    const confidence = [
      { field: 'title', tier: 'high' as const, source: 'selector' },
      { field: 'company', tier: 'high' as const, source: 'selector' },
      { field: 'salary_min', tier: 'medium' as const, source: 'parser-body' },
      { field: 'url', tier: 'high' as const, source: 'selector' },
      { field: 'description', tier: 'high' as const, source: 'selector' },
      { field: 'source_plugin', tier: 'high' as const, source: 'selector' },
    ]
    expect(shouldShowOverlay(confidence, DEFAULT_FLOORS, false)).toBe(false)
  })

  test('returns true when a high-priority field is absent', () => {
    const confidence = [
      { field: 'title', tier: 'absent' as const, source: 'missing' },
      { field: 'company', tier: 'high' as const, source: 'selector' },
    ]
    expect(shouldShowOverlay(confidence, DEFAULT_FLOORS, false)).toBe(true)
  })

  test('returns true when salary is absent and floor is medium', () => {
    const confidence = [
      { field: 'salary_min', tier: 'absent' as const, source: 'missing' },
    ]
    expect(shouldShowOverlay(confidence, DEFAULT_FLOORS, false)).toBe(true)
  })

  test('returns true when forceManual is true regardless of confidence', () => {
    const confidence = [
      { field: 'title', tier: 'high' as const, source: 'selector' },
    ]
    expect(shouldShowOverlay(confidence, DEFAULT_FLOORS, true)).toBe(true)
  })

  test('dev mode floors never trigger overlay', () => {
    const confidence = [
      { field: 'title', tier: 'absent' as const, source: 'missing' },
    ]
    expect(shouldShowOverlay(confidence, CONFIDENCE_MODES.dev, false)).toBe(false)
  })

  test('debug mode triggers on any non-high field', () => {
    const confidence = [
      { field: 'salary_min', tier: 'medium' as const, source: 'parser-body' },
    ]
    expect(shouldShowOverlay(confidence, CONFIDENCE_MODES.debug, false)).toBe(true)
  })
})
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd packages/extension && bun test tests/lib/confidence.test.ts`
Expected: FAIL (module not found)

- [ ] **Step 4: Write confidence module**

```ts
// packages/extension/src/lib/confidence.ts

import type { ExtractedJob, ConfidenceTier, FieldConfidence } from '../plugin/types'

/** Numeric ordering for tier comparisons. Higher = more confident. */
export const CONFIDENCE_ORDER: Record<ConfidenceTier, number> = {
  high: 3,
  medium: 2,
  low: 1,
  absent: 0,
}

/** Fields tracked by the confidence model and their default floors. */
export const DEFAULT_FLOORS: Record<string, ConfidenceTier> = {
  title: 'high',
  company: 'high',
  salary_min: 'medium',
  salary_max: 'medium',
  work_posture: 'medium',
  location: 'medium',
  company_url: 'high',
  url: 'high',
  apply_url: 'low',
  description: 'high',
  source_plugin: 'high',
  // Low-priority parsed sections — floor of 'absent' means never trigger
  parsed_requirements: 'absent',
  parsed_responsibilities: 'absent',
  parsed_preferred: 'absent',
}

/** Preset confidence modes. */
export const CONFIDENCE_MODES: Record<string, Record<string, ConfidenceTier>> = {
  user: DEFAULT_FLOORS,
  debug: Object.fromEntries(
    Object.keys(DEFAULT_FLOORS).map(k => [k, 'high' as ConfidenceTier])
  ),
  dev: Object.fromEntries(
    Object.keys(DEFAULT_FLOORS).map(k => [k, 'absent' as ConfidenceTier])
  ),
}

/** Tracked fields and how to check if they're present. */
const FIELD_CHECKS: Array<{ field: string; check: (e: ExtractedJob) => boolean }> = [
  { field: 'title', check: e => !!e.title?.trim() },
  { field: 'company', check: e => !!e.company?.trim() },
  { field: 'salary_min', check: e => e.salary_min != null },
  { field: 'salary_max', check: e => e.salary_max != null },
  { field: 'work_posture', check: e => !!e.work_posture?.trim() },
  { field: 'location', check: e => !!e.location?.trim() },
  { field: 'company_url', check: e => !!e.company_url?.trim() },
  { field: 'url', check: e => !!e.url?.trim() },
  { field: 'apply_url', check: e => !!e.apply_url?.trim() },
  { field: 'description', check: e => !!e.description?.trim() },
  { field: 'source_plugin', check: e => !!e.source_plugin?.trim() },
]

/**
 * Assign confidence tiers to each tracked field.
 *
 * @param extracted The extraction result
 * @param overrides Explicit tier+source for fields where the extraction
 *   source is known (e.g. chip vs. parser). Fields without overrides
 *   get 'high' if present, 'absent' if not.
 */
export function assignConfidence(
  extracted: ExtractedJob,
  overrides: Record<string, { tier: ConfidenceTier; source: string }>,
): FieldConfidence[] {
  return FIELD_CHECKS.map(({ field, check }) => {
    const present = check(extracted)
    const override = overrides[field]

    if (override) {
      return { field, tier: override.tier, source: override.source }
    }

    return {
      field,
      tier: present ? 'high' : 'absent',
      source: present ? 'selector' : 'missing',
    }
  })
}

/**
 * Determine whether the overlay should be shown.
 *
 * Returns true if any field's confidence tier is strictly below its
 * configured floor, OR if forceManual is true.
 */
export function shouldShowOverlay(
  confidence: FieldConfidence[],
  floors: Record<string, ConfidenceTier>,
  forceManual: boolean,
): boolean {
  if (forceManual) return true

  for (const entry of confidence) {
    const floor = floors[entry.field]
    if (!floor) continue // unknown field, skip
    if (CONFIDENCE_ORDER[entry.tier] < CONFIDENCE_ORDER[floor]) {
      return true
    }
  }

  return false
}

/** Storage key for confidence mode in chrome.storage.local. */
export const CONFIDENCE_MODE_KEY = 'forge_confidence_mode'

/** Load the current confidence mode floors. Falls back to 'user' mode. */
export async function loadConfidenceFloors(): Promise<Record<string, ConfidenceTier>> {
  try {
    const stored = await chrome.storage.local.get(CONFIDENCE_MODE_KEY)
    const mode = stored[CONFIDENCE_MODE_KEY] as string | undefined
    return CONFIDENCE_MODES[mode ?? 'user'] ?? DEFAULT_FLOORS
  } catch {
    return DEFAULT_FLOORS
  }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd packages/extension && bun test tests/lib/confidence.test.ts`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add packages/extension/src/plugin/types.ts packages/extension/src/lib/confidence.ts packages/extension/tests/lib/confidence.test.ts
git commit -m "feat(ext): add confidence tier model with decision gate (M6)"
```

---

## Task 6: Enrich Extraction with Confidence

**Files:**
- Modify: `packages/extension/src/lib/enrich-extraction.ts`
- Modify: `packages/extension/tests/lib/enrich-extraction.test.ts`

- [ ] **Step 1: Update enrich-extraction.ts to return EnrichedExtraction**

Replace the existing `enrichWithParser` with a version that also returns confidence:

```ts
// packages/extension/src/lib/enrich-extraction.ts

import { parseJobDescription } from '@forge/core/src/parser'
import type { ExtractedJob, ConfidenceTier, FieldConfidence, EnrichedExtraction } from '../plugin/types'
import { assignConfidence } from './confidence'

/**
 * Enrich an extraction with parser-derived fields AND confidence scores.
 *
 * Confidence overrides track which fields came from the parser (medium)
 * vs. which were already present from DOM selectors (high).
 */
export function enrichWithParser(extracted: ExtractedJob): EnrichedExtraction {
  const overrides: Record<string, { tier: ConfidenceTier; source: string }> = {}

  // Fields already present from DOM extraction get 'high' confidence
  if (extracted.title) overrides.title = { tier: 'high', source: 'selector' }
  if (extracted.company) overrides.company = { tier: 'high', source: 'selector' }
  if (extracted.url) overrides.url = { tier: 'high', source: 'selector' }
  if (extracted.description) overrides.description = { tier: 'high', source: 'selector' }
  if (extracted.source_plugin) overrides.source_plugin = { tier: 'high', source: 'selector' }
  if (extracted.company_url) overrides.company_url = { tier: 'high', source: 'selector' }
  if (extracted.apply_url) overrides.apply_url = { tier: 'medium', source: 'selector' }

  // Chip-sourced fields are high confidence
  if (extracted.salary_range) {
    overrides.salary_min = { tier: 'high', source: 'chip' }
    overrides.salary_max = { tier: 'high', source: 'chip' }
  }
  if (extracted.location) overrides.location = { tier: 'high', source: 'chip' }

  if (!extracted.description) {
    const confidence = assignConfidence(extracted, overrides)
    return { extracted, confidence }
  }

  const parsed = parseJobDescription(extracted.description)

  const enriched: ExtractedJob = {
    ...extracted,
    salary_min: parsed.salary?.min != null ? Math.round(parsed.salary.min) : null,
    salary_max: parsed.salary?.max != null ? Math.round(parsed.salary.max) : null,
    salary_period: parsed.salary?.period ?? null,
    work_posture: parsed.workPosture,
    parsed_locations: parsed.locations,
    parsed_sections: JSON.stringify(parsed.sections),
  }

  // If salary wasn't from chip but parser found it, mark as medium
  if (!extracted.salary_range && parsed.salary?.min != null) {
    overrides.salary_min = { tier: 'medium', source: 'parser-body' }
    overrides.salary_max = { tier: 'medium', source: 'parser-body' }
  }
  // If work_posture came from parser
  if (parsed.workPosture) {
    overrides.work_posture = { tier: 'medium', source: 'parser-body' }
  }
  // If location came from parser (not chip)
  if (!extracted.location && parsed.locations?.length) {
    overrides.location = { tier: 'medium', source: 'parser-body' }
    enriched.location = parsed.locations[0]
  }

  const confidence = assignConfidence(enriched, overrides)
  return { extracted: enriched, confidence }
}
```

- [ ] **Step 2: Update existing enrichment tests**

Update `packages/extension/tests/lib/enrich-extraction.test.ts` to handle the new return type (`EnrichedExtraction` with `.extracted` and `.confidence`):

- Change all references from `result.salary_min` to `result.extracted.salary_min` etc.
- Add new tests verifying confidence scores are included:

```ts
test('includes confidence scores in result', () => {
  const input = makeExtracted({
    description: '## Compensation\nSalary: $150,000 - $200,000 per year.',
  })
  const result = enrichWithParser(input)
  expect(result.confidence).toBeDefined()
  expect(result.confidence.length).toBeGreaterThan(0)

  const salaryConf = result.confidence.find(c => c.field === 'salary_min')
  expect(salaryConf?.tier).toBe('medium')
  expect(salaryConf?.source).toBe('parser-body')
})

test('chip-sourced salary gets high confidence', () => {
  const input = makeExtracted({
    salary_range: '$150k-$200k',
    description: '## About\nGreat company.',
  })
  const result = enrichWithParser(input)
  const salaryConf = result.confidence.find(c => c.field === 'salary_min')
  expect(salaryConf?.tier).toBe('high')
  expect(salaryConf?.source).toBe('chip')
})
```

- [ ] **Step 3: Run tests**

Run: `cd packages/extension && bun test tests/lib/enrich-extraction.test.ts`
Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git add packages/extension/src/lib/enrich-extraction.ts packages/extension/tests/lib/enrich-extraction.test.ts
git commit -m "feat(ext): enrichWithParser returns confidence scores (M6)"
```

---

## Task 7: Toast Notification

**Files:**
- Create: `packages/extension/src/content/toast.ts`

- [ ] **Step 1: Write toast module**

```ts
// packages/extension/src/content/toast.ts
//
// Shadow DOM toast notification for quiet-mode captures.
// Self-contained — no host CSS leakage.

const TOAST_HOST_ID = 'forge-toast-host'

export function showToast(message: string, durationMs = 3000): void {
  // Remove any existing toast
  const existing = document.getElementById(TOAST_HOST_ID)
  if (existing) existing.remove()

  const host = document.createElement('div')
  host.id = TOAST_HOST_ID
  const shadow = host.attachShadow({ mode: 'closed' })

  shadow.innerHTML = `
    <style>
      .toast {
        position: fixed;
        bottom: 20px;
        right: 20px;
        z-index: 2147483647;
        background: #1a1a2e;
        color: #e0e0e0;
        border: 1px solid #333;
        border-left: 3px solid #4ade80;
        border-radius: 6px;
        padding: 12px 16px;
        font-family: -apple-system, system-ui, sans-serif;
        font-size: 13px;
        line-height: 1.4;
        max-width: 360px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.3);
        opacity: 1;
        transition: opacity 0.3s ease-out;
      }
      .toast.fade-out { opacity: 0; }
      .toast-header {
        display: flex;
        align-items: center;
        gap: 6px;
        margin-bottom: 4px;
        font-weight: 600;
        color: #4ade80;
        font-size: 12px;
      }
      .toast-body { color: #ccc; }
    </style>
    <div class="toast">
      <div class="toast-header">
        <span>\u2713</span>
        <span>Forge</span>
      </div>
      <div class="toast-body">${escapeHtml(message)}</div>
    </div>
  `

  document.body.appendChild(host)

  // Auto-dismiss with fade
  setTimeout(() => {
    const toast = shadow.querySelector('.toast')
    if (toast) toast.classList.add('fade-out')
    setTimeout(() => host.remove(), 300)
  }, durationMs)
}

function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/extension/src/content/toast.ts
git commit -m "feat(ext): add shadow DOM toast notification (M6)"
```

---

## Task 8: Side Panel Overlay

**Files:**
- Create: `packages/extension/src/content/overlay.ts`

- [ ] **Step 1: Write overlay module**

```ts
// packages/extension/src/content/overlay.ts
//
// Shadow DOM side panel for reviewing/editing extracted fields.
// Injected by content script when confidence gate triggers.
// Self-contained — no host CSS leakage.

import type { ExtractedJob, FieldConfidence, ConfidenceTier } from '../plugin/types'

const OVERLAY_HOST_ID = 'forge-overlay-host'

interface OverlayCallbacks {
  onSubmit: (edited: ExtractedJob) => void
  onCancel: () => void
}

/** Field display config for the overlay form. */
interface OverlayField {
  key: string
  label: string
  getValue: (e: ExtractedJob) => string
  setValue: (e: ExtractedJob, v: string) => ExtractedJob
  type: 'text' | 'textarea'
}

const OVERLAY_FIELDS: OverlayField[] = [
  {
    key: 'title',
    label: 'Job Title',
    getValue: e => e.title ?? '',
    setValue: (e, v) => ({ ...e, title: v || null }),
    type: 'text',
  },
  {
    key: 'company',
    label: 'Company',
    getValue: e => e.company ?? '',
    setValue: (e, v) => ({ ...e, company: v || null }),
    type: 'text',
  },
  {
    key: 'location',
    label: 'Location',
    getValue: e => e.location ?? '',
    setValue: (e, v) => ({ ...e, location: v || null }),
    type: 'text',
  },
  {
    key: 'salary_min',
    label: 'Salary Min',
    getValue: e => e.salary_min != null ? String(e.salary_min) : '',
    setValue: (e, v) => ({ ...e, salary_min: v ? Number(v) : null }),
    type: 'text',
  },
  {
    key: 'salary_max',
    label: 'Salary Max',
    getValue: e => e.salary_max != null ? String(e.salary_max) : '',
    setValue: (e, v) => ({ ...e, salary_max: v ? Number(v) : null }),
    type: 'text',
  },
  {
    key: 'work_posture',
    label: 'Work Posture',
    getValue: e => e.work_posture ?? '',
    setValue: (e, v) => ({ ...e, work_posture: v || null }),
    type: 'text',
  },
  {
    key: 'company_url',
    label: 'Company URL',
    getValue: e => e.company_url ?? '',
    setValue: (e, v) => ({ ...e, company_url: v || null }),
    type: 'text',
  },
  {
    key: 'apply_url',
    label: 'Apply URL',
    getValue: e => e.apply_url ?? '',
    setValue: (e, v) => ({ ...e, apply_url: v || null }),
    type: 'text',
  },
  {
    key: 'url',
    label: 'Job Post URL',
    getValue: e => e.url ?? '',
    setValue: (e, v) => ({ ...e, url: v }),
    type: 'text',
  },
]

function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

function tierColor(tier: ConfidenceTier): string {
  switch (tier) {
    case 'high': return '#4ade80'
    case 'medium': return '#fbbf24'
    case 'low': return '#f87171'
    case 'absent': return '#f87171'
  }
}

function tierIcon(tier: ConfidenceTier): string {
  switch (tier) {
    case 'high': return '\u2713'
    case 'medium': return '\u26a0'
    case 'low': return '\u2717'
    case 'absent': return '\u2717'
  }
}

export function showOverlay(
  extracted: ExtractedJob,
  confidence: FieldConfidence[],
  callbacks: OverlayCallbacks,
): void {
  // Remove any existing overlay
  removeOverlay()

  const host = document.createElement('div')
  host.id = OVERLAY_HOST_ID
  const shadow = host.attachShadow({ mode: 'closed' })

  const confMap = new Map(confidence.map(c => [c.field, c]))
  const needsReview = confidence.filter(c =>
    c.tier === 'absent' || c.tier === 'low' || c.tier === 'medium'
  ).length

  // Build field HTML
  const fieldsHtml = OVERLAY_FIELDS.map(f => {
    const conf = confMap.get(f.key)
    const tier = conf?.tier ?? 'absent'
    const color = tierColor(tier)
    const icon = tierIcon(tier)
    const value = escapeHtml(f.getValue(extracted))
    const inputTag = f.type === 'textarea'
      ? `<textarea data-field="${f.key}" rows="3">${value}</textarea>`
      : `<input data-field="${f.key}" type="text" value="${value}" />`

    return `
      <div class="field" style="border-left-color: ${color}">
        <div class="field-header">
          <label>${escapeHtml(f.label)}</label>
          <span class="confidence-badge" style="color: ${color}">${icon} ${tier}</span>
        </div>
        ${inputTag}
      </div>
    `
  }).join('')

  shadow.innerHTML = `
    <style>
      :host {
        all: initial;
      }
      .overlay {
        position: fixed;
        right: 0;
        top: 0;
        width: 360px;
        height: 100vh;
        z-index: 2147483647;
        background: #1a1a2e;
        color: #e0e0e0;
        font-family: -apple-system, system-ui, sans-serif;
        font-size: 13px;
        display: flex;
        flex-direction: column;
        box-shadow: -4px 0 16px rgba(0,0,0,0.4);
        border-left: 1px solid #333;
      }
      .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 12px 16px;
        border-bottom: 1px solid #333;
        flex-shrink: 0;
      }
      .header h2 {
        margin: 0;
        font-size: 14px;
        color: #a0a0ff;
      }
      .close-btn {
        background: none;
        border: none;
        color: #888;
        font-size: 18px;
        cursor: pointer;
        padding: 4px;
      }
      .close-btn:hover { color: #fff; }
      .review-badge {
        padding: 6px 16px;
        background: #2a1a3e;
        border-bottom: 1px solid #333;
        font-size: 12px;
        color: #fbbf24;
        flex-shrink: 0;
      }
      .fields {
        flex: 1;
        overflow-y: auto;
        padding: 12px 16px;
      }
      .field {
        margin-bottom: 12px;
        padding-left: 8px;
        border-left: 3px solid #333;
      }
      .field-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 4px;
      }
      .field-header label {
        font-weight: 600;
        font-size: 12px;
        color: #ccc;
      }
      .confidence-badge {
        font-size: 11px;
        font-weight: 500;
      }
      input, textarea {
        width: 100%;
        background: #12121e;
        border: 1px solid #333;
        border-radius: 4px;
        color: #e0e0e0;
        padding: 6px 8px;
        font-size: 13px;
        font-family: inherit;
        box-sizing: border-box;
      }
      input:focus, textarea:focus {
        outline: none;
        border-color: #6366f1;
        box-shadow: 0 0 0 2px rgba(99,102,241,0.2);
      }
      textarea { resize: vertical; }
      .footer {
        display: flex;
        gap: 8px;
        padding: 12px 16px;
        border-top: 1px solid #333;
        flex-shrink: 0;
      }
      .btn {
        flex: 1;
        padding: 8px 12px;
        border-radius: 4px;
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
        border: none;
      }
      .btn-cancel {
        background: transparent;
        border: 1px solid #444;
        color: #aaa;
      }
      .btn-cancel:hover { background: #222; }
      .btn-submit {
        background: #6366f1;
        color: #fff;
      }
      .btn-submit:hover { background: #5558e6; }
    </style>
    <div class="overlay">
      <div class="header">
        <h2>Forge \u2014 Review Extraction</h2>
        <button class="close-btn" title="Close">\u2715</button>
      </div>
      ${needsReview > 0 ? `<div class="review-badge">${needsReview} field${needsReview > 1 ? 's' : ''} need review</div>` : ''}
      <div class="fields">
        ${fieldsHtml}
      </div>
      <div class="footer">
        <button class="btn btn-cancel">Cancel</button>
        <button class="btn btn-submit">Submit to Forge</button>
      </div>
    </div>
  `

  // Wire up event handlers
  const cancelBtn = shadow.querySelector('.btn-cancel') as HTMLButtonElement
  const submitBtn = shadow.querySelector('.btn-submit') as HTMLButtonElement
  const closeBtn = shadow.querySelector('.close-btn') as HTMLButtonElement

  cancelBtn.addEventListener('click', () => {
    removeOverlay()
    callbacks.onCancel()
  })

  closeBtn.addEventListener('click', () => {
    removeOverlay()
    callbacks.onCancel()
  })

  submitBtn.addEventListener('click', () => {
    // Read edited values from inputs
    let edited = { ...extracted }
    for (const f of OVERLAY_FIELDS) {
      const input = shadow.querySelector(`[data-field="${f.key}"]`) as HTMLInputElement | HTMLTextAreaElement | null
      if (input) {
        edited = f.setValue(edited, input.value)
      }
    }
    removeOverlay()
    callbacks.onSubmit(edited)
  })

  document.body.appendChild(host)
}

export function removeOverlay(): void {
  const existing = document.getElementById(OVERLAY_HOST_ID)
  if (existing) existing.remove()
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/extension/src/content/overlay.ts
git commit -m "feat(ext): add shadow DOM review overlay panel (M6)"
```

---

## Task 9: Background Submit Handler + Messaging

**Files:**
- Modify: `packages/extension/src/lib/messaging.ts`
- Modify: `packages/extension/src/background/handlers/capture.ts`
- Modify: `packages/extension/src/background/index.ts`

- [ ] **Step 1: Add new Command variants to messaging.ts**

In `packages/extension/src/lib/messaging.ts`, extend the `Command` union:

```ts
export type Command =
  | { cmd: 'health' }
  | { cmd: 'orgs.list'; limit?: number }
  | { cmd: 'orgs.create'; payload: { name: string } }
  | { cmd: 'form.testFill' }
  | { cmd: 'form.profileFill' }
  | { cmd: 'jd.captureActive' }
  | { cmd: 'jd.captureActive'; forceManual?: boolean }
  | { cmd: 'jd.submitExtracted'; data: unknown }
  | { cmd: 'answers.list' }
```

Note: The `jd.captureActive` variant gains an optional `forceManual` flag. Merge the two into a single variant:

```ts
| { cmd: 'jd.captureActive'; forceManual?: boolean }
```

Remove the duplicate `| { cmd: 'jd.captureActive' }` line.

- [ ] **Step 2: Add handleSubmitExtracted to capture.ts**

In `packages/extension/src/background/handlers/capture.ts`, add a new exported function after `handleCaptureJob`:

```ts
/**
 * Submit a pre-extracted (potentially user-edited) job from the overlay.
 * Runs steps 4-7 of the normal capture flow: validate → dedup → org resolve → API create.
 */
export async function handleSubmitExtracted(extracted: Record<string, unknown>): Promise<Response<CaptureJobPayload>> {
  try {
    const validation = validateExtraction(extracted as any)
    if (!validation.valid) {
      return {
        ok: false,
        error: extError('EXTRACTION_INCOMPLETE', "Couldn't extract job title and description", {
          layer: 'background',
          context: { missing: validation.missing },
        }),
      }
    }

    const client = await getClient()

    // Normalize URL and dedup check
    const canonicalUrl = normalizeJobUrl(extracted.url as string)
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

    // Resolve organization
    const organizationId = await resolveOrganization(
      extracted.company as string | null,
      async (search) => {
        const result = await client.organizations.list({ search, limit: 5 })
        return result.ok ? { ok: true, data: result.data } : { ok: false }
      },
      async (name, opts) => {
        const result = await client.organizations.create({
          name,
          linkedin_url: opts?.linkedin_url,
        })
        return result.ok ? { ok: true, data: { id: result.data.id, name: result.data.name } } : { ok: false }
      },
      { linkedin_url: (extracted.company_url as string) ?? undefined },
    )

    // Create JD
    const result = await client.jobDescriptions.create({
      title: extracted.title as string,
      raw_text: extracted.description as string,
      url: canonicalUrl,
      location: (extracted.location as string) ?? undefined,
      salary_range: (extracted.salary_range as string) ?? undefined,
      organization_id: organizationId ?? undefined,
      salary_min: (extracted.salary_min as number) ?? undefined,
      salary_max: (extracted.salary_max as number) ?? undefined,
      salary_period: (extracted.salary_period as string) ?? undefined,
      work_posture: (extracted.work_posture as string) ?? undefined,
      parsed_locations: Array.isArray(extracted.parsed_locations) && extracted.parsed_locations.length
        ? JSON.stringify(extracted.parsed_locations)
        : undefined,
      parsed_sections: (extracted.parsed_sections as string) ?? undefined,
    })

    if (!result.ok) {
      return { ok: false, error: mapSdkError(result.error, { url: '/api/job-descriptions' }) }
    }

    return { ok: true, data: { id: result.data.id, title: result.data.title } }
  } catch (err) {
    return { ok: false, error: mapNetworkError(err, { url: '/api/job-descriptions' }) }
  }
}
```

- [ ] **Step 3: Add handleAnswersList to a new handler file or inline in background/index.ts**

Add an answers list handler in `packages/extension/src/background/handlers/answers.ts`:

```ts
// packages/extension/src/background/handlers/answers.ts

import type { Response } from '../../lib/messaging'
import { extError, mapNetworkError } from '../../lib/errors'
import { getClient } from '../client'

export interface AnswersListPayload {
  answers: Record<string, string>
}

export async function handleAnswersList(): Promise<Response<AnswersListPayload>> {
  try {
    const client = await getClient()
    const result = await client.answerBank.list()
    if (!result.ok) {
      return {
        ok: false,
        error: extError('API_UNREACHABLE', 'Could not load answer bank', { layer: 'sdk' }),
      }
    }

    // Convert array to field_kind → value map for easy lookup
    const answers: Record<string, string> = {}
    for (const entry of result.data) {
      answers[entry.field_kind] = entry.value
    }
    return { ok: true, data: { answers } }
  } catch (err) {
    return { ok: false, error: mapNetworkError(err, { url: '/api/profile/answers' }) }
  }
}
```

- [ ] **Step 4: Wire new commands into background/index.ts**

In `packages/extension/src/background/index.ts`:

Add imports:
```ts
import { handleSubmitExtracted } from './handlers/capture'
import { handleAnswersList } from './handlers/answers'
```

Add cases to the switch:
```ts
case 'jd.submitExtracted':
  response = await handleSubmitExtracted(msg.data as Record<string, unknown>)
  break
case 'answers.list':
  response = await handleAnswersList()
  break
```

Update the `jd.captureActive` case to pass `forceManual`:
```ts
case 'jd.captureActive':
  response = await handleCaptureJob(undefined, (msg as any).forceManual)
  break
```

- [ ] **Step 5: Update handleCaptureJob to pass forceManual to content script**

In `packages/extension/src/background/handlers/capture.ts`, update the function signature:
```ts
export async function handleCaptureJob(explicitTabId?: number, forceManual?: boolean): Promise<Response<CaptureJobPayload>>
```

Update the `sendMessage` call to pass forceManual:
```ts
const extractResponse = await chrome.tabs.sendMessage(tabId, { cmd: 'extract', forceManual: !!forceManual })
```

- [ ] **Step 6: Commit**

```bash
git add packages/extension/src/lib/messaging.ts packages/extension/src/background/handlers/capture.ts packages/extension/src/background/handlers/answers.ts packages/extension/src/background/index.ts
git commit -m "feat(ext): add submitExtracted + answers.list background handlers (M6)"
```

---

## Task 10: Rewire LinkedIn Content Script (Overlay Integration)

**Files:**
- Modify: `packages/extension/src/content/linkedin.ts`

- [ ] **Step 1: Update linkedin.ts to use decision gate**

Replace the current message handler with the overlay-integrated version:

```ts
// packages/extension/src/content/linkedin.ts

import { linkedinPlugin } from '../plugin/plugins/linkedin'
import { enrichWithParser } from '../lib/enrich-extraction'
import { shouldShowOverlay, loadConfidenceFloors } from '../lib/confidence'
import { showOverlay } from './overlay'
import { showToast } from './toast'

interface ExtractMessage {
  cmd: 'extract'
  forceManual?: boolean
}

type IncomingMessage = ExtractMessage

declare global {
  interface Window {
    __forge_extension_linkedin_ready?: boolean
  }
}

import { injectCaptureButton, observeForInjection } from './inject-button'

if (!window.__forge_extension_linkedin_ready) {
  window.__forge_extension_linkedin_ready = true

  setTimeout(() => injectCaptureButton(document), 500)
  observeForInjection(document)

  chrome.runtime.onMessage.addListener((msg: IncomingMessage, _sender, sendResponse) => {
    if (msg.cmd === 'extract') {
      ;(async () => {
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

          const enriched = enrichWithParser(result)
          const floors = await loadConfidenceFloors()
          const forceManual = !!msg.forceManual

          if (shouldShowOverlay(enriched.confidence, floors, forceManual)) {
            // Show overlay for review — don't sendResponse yet
            // The overlay will submit via a separate message
            showOverlay(enriched.extracted, enriched.confidence, {
              onSubmit: async (edited) => {
                const submitResponse = await chrome.runtime.sendMessage({
                  cmd: 'jd.submitExtracted',
                  data: edited,
                })
                if (submitResponse?.ok) {
                  showToast(`Captured: ${edited.title} at ${edited.company}`)
                } else {
                  const code = submitResponse?.error?.code
                  if (code === 'API_DUPLICATE') {
                    showToast('Job already captured')
                  } else {
                    showToast(`Error: ${submitResponse?.error?.message ?? 'Unknown error'}`)
                  }
                }
              },
              onCancel: () => {
                // User cancelled — do nothing
              },
            })
            // Respond to background that overlay is handling it
            sendResponse({ ok: true, data: { overlayShown: true } })
          } else {
            // Quiet mode — submit directly
            const submitResponse = await chrome.runtime.sendMessage({
              cmd: 'jd.submitExtracted',
              data: enriched.extracted,
            })
            if (submitResponse?.ok) {
              showToast(`Captured: ${enriched.extracted.title} at ${enriched.extracted.company}`)
            }
            sendResponse(submitResponse)
          }
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

- [ ] **Step 2: Update handleCaptureJob to handle overlayShown response**

In `packages/extension/src/background/handlers/capture.ts`, after receiving `extractResponse`, add handling for the overlay case:

```ts
// After: const extractResponse = await chrome.tabs.sendMessage(tabId, { cmd: 'extract', forceManual: !!forceManual })

if (!extractResponse?.ok) {
  // ... existing error handling
}

// If overlay is handling the submission, return early with success
if (extractResponse.data?.overlayShown) {
  return {
    ok: true,
    data: { id: 'pending-overlay', title: 'Review in overlay' },
  }
}
```

The rest of the original direct-to-API code path in `handleCaptureJob` can remain as fallback but won't be reached in the normal flow anymore since the content script now handles both paths.

- [ ] **Step 3: Commit**

```bash
git add packages/extension/src/content/linkedin.ts packages/extension/src/background/handlers/capture.ts
git commit -m "feat(ext): wire overlay + toast into LinkedIn capture flow (M6)"
```

---

## Task 11: Injected Button Shift+Click Support

**Files:**
- Modify: `packages/extension/src/content/inject-button.ts`

- [ ] **Step 1: Update inject-button click handler to detect Shift key**

In `packages/extension/src/content/inject-button.ts`, update the click handler:

```ts
btn.addEventListener('click', async (e) => {
  e.preventDefault()
  e.stopPropagation()
  btn.textContent = 'Capturing...'
  btn.disabled = true

  const forceManual = e.shiftKey

  try {
    const response = await chrome.runtime.sendMessage({
      cmd: 'jd.captureActive',
      forceManual,
    })
    if (response?.ok) {
      if (response.data?.overlayShown) {
        btn.textContent = 'Review in panel \u2192'
        btn.style.borderColor = '#6366f1'
        btn.style.color = '#6366f1'
      } else {
        btn.textContent = '\u2713 Captured'
        btn.style.borderColor = '#057642'
        btn.style.color = '#057642'
      }
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

  setTimeout(() => {
    btn.textContent = 'Capture to Forge'
    btn.disabled = false
    btn.style.borderColor = '#0a66c2'
    btn.style.color = '#0a66c2'
  }, 3000)
})
```

- [ ] **Step 2: Update popup to support forceManual**

In `packages/extension/src/popup/Popup.svelte`, update the `captureJob` function to accept a forceManual parameter and update the button to pass Shift state. The simplest approach: detect Shift key on the capture button click event:

Update the button onclick:
```svelte
<button onclick={(e) => captureJob(e.shiftKey)}>Capture Job</button>
```

Update the function signature:
```ts
async function captureJob(forceManual = false) {
```

Update the sendCommand call:
```ts
const response = await sendCommand<CaptureJobPayload>({ cmd: 'jd.captureActive', forceManual })
```

Add handling for overlayShown response:
```ts
if (response.ok) {
  if ((response.data as any).overlayShown) {
    status = 'Review in page overlay'
    statusKind = 'info'
  } else {
    status = `JD created: ${response.data.id.slice(0, 8)}...`
    statusKind = 'ok'
  }
}
```

- [ ] **Step 3: Commit**

```bash
git add packages/extension/src/content/inject-button.ts packages/extension/src/popup/Popup.svelte
git commit -m "feat(ext): Shift+click forces overlay, handle overlayShown in UI (M6)"
```

---

## Task 12: Workday EEO/Work-Auth Field Detection

**Files:**
- Modify: `packages/extension/src/plugin/plugins/workday.ts`

- [ ] **Step 1: Add EEO/work-auth entries to FIELD_NAME_MAP**

In `packages/extension/src/plugin/plugins/workday.ts`, extend the `FIELD_NAME_MAP`:

```ts
const FIELD_NAME_MAP: Record<string, FieldKind> = {
  // Existing personal info
  'legalName--firstName': 'name.first',
  'legalName--lastName': 'name.last',
  'email': 'email',
  'city': 'address.city',
  'countryRegion': 'address.state',
  'country': 'address.country',
  'phoneNumber': 'phone',
  'phoneType': 'phone.type',
  // Application Questions (M6)
  'workAuthorizationStatus': 'work_auth.us',
  'sponsorshipRequired': 'work_auth.sponsorship',
  'authorized': 'work_auth.us',
  'sponsorship': 'work_auth.sponsorship',
  // Voluntary Disclosures / EEO (M6)
  'gender': 'eeo.gender',
  'race': 'eeo.race',
  'ethnicity': 'eeo.race',
  'veteranStatus': 'eeo.veteran',
  'disabilityStatus': 'eeo.disability',
  'disability': 'eeo.disability',
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/extension/src/plugin/plugins/workday.ts
git commit -m "feat(ext): detect EEO/work-auth fields in Workday forms (M6)"
```

---

## Task 13: Answer Bank Merge in Autofill Handler

**Files:**
- Modify: `packages/extension/src/background/handlers/autofill.ts`

- [ ] **Step 1: Merge answer bank into profile fill values**

In `packages/extension/src/background/handlers/autofill.ts`, update `handleProfileFill` to also fetch answer bank entries:

After the profile field map is built (`const values = buildProfileFieldMap(profileResult.data)`), add:

```ts
// Merge answer bank entries (EEO/work-auth fields)
try {
  const answersResult = await client.answerBank.list()
  if (answersResult.ok) {
    for (const entry of answersResult.data) {
      // Answer bank values take precedence for their field_kinds
      values[entry.field_kind] = entry.value
    }
  }
} catch {
  // Answer bank fetch failure is non-fatal — continue with profile-only values
}
```

This goes between the `buildProfileFieldMap` call and the "check if values is empty" guard.

- [ ] **Step 2: Run existing autofill tests**

Run: `cd packages/extension && bun test`
Expected: All PASS (answer bank merge is additive, doesn't break existing behavior)

- [ ] **Step 3: Commit**

```bash
git add packages/extension/src/background/handlers/autofill.ts
git commit -m "feat(ext): merge answer bank into Workday autofill values (M6)"
```

---

## Task 14: Vite Config + Build Entry Points

**Files:**
- Modify: `packages/extension/vite.config.ts`

- [ ] **Step 1: Add overlay as inlined content in linkedin bundle**

The overlay and toast are imported by `linkedin.ts` content script. Since content scripts must be self-contained IIFE bundles, Vite will inline these imports automatically (same as how `enrich-extraction.ts` is inlined today). No new entry point needed — they're already imported by the content scripts.

However, verify the build output doesn't create shared chunks. If overlay/toast are also needed by other content scripts in the future, they'd need separate entry points. For M6, they're only used by linkedin.ts, so no vite.config.ts change is needed.

- [ ] **Step 2: Build both browsers**

Run: `cd packages/extension && bun run build`
Expected: Both `dist/chrome/` and `dist/firefox/` produced without errors

- [ ] **Step 3: Verify no import statements in content scripts**

Run: `cd packages/extension && bun test tests/build/`
Expected: All build output tests PASS

- [ ] **Step 4: Commit (if any vite.config changes were needed)**

Only commit if changes were made.

---

## Task 15: WebUI Settings Pages

**Files:**
- Create: `packages/webui/src/routes/settings/work-auth/+page.svelte`
- Create: `packages/webui/src/routes/settings/eeo/+page.svelte`

- [ ] **Step 1: Check existing WebUI settings structure**

Check if `packages/webui/src/routes/settings/` exists and what pattern to follow. If it doesn't exist, create the directory.

- [ ] **Step 2: Write work-auth settings page**

```svelte
<!-- packages/webui/src/routes/settings/work-auth/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte'
  import { PageWrapper, PageHeader } from '$lib/components'

  const API_BASE = 'http://localhost:3000/api'

  interface AnswerEntry {
    field_kind: string
    label: string
    value: string
  }

  let workAuthUs = $state('')
  let sponsorship = $state('')
  let saving = $state(false)
  let status = $state<string | null>(null)

  onMount(async () => {
    const res = await fetch(`${API_BASE}/profile/answers`)
    if (res.ok) {
      const body = await res.json()
      const entries = body.data as AnswerEntry[]
      const usEntry = entries.find(e => e.field_kind === 'work_auth.us')
      const sponsorEntry = entries.find(e => e.field_kind === 'work_auth.sponsorship')
      if (usEntry) workAuthUs = usEntry.value
      if (sponsorEntry) sponsorship = sponsorEntry.value
    }
  })

  async function save() {
    saving = true
    status = null

    const upsert = async (field_kind: string, label: string, value: string) => {
      await fetch(`${API_BASE}/profile/answers`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ field_kind, label, value }),
      })
    }

    try {
      await upsert('work_auth.us', 'Authorized to work in US', workAuthUs)
      await upsert('work_auth.sponsorship', 'Require sponsorship', sponsorship)
      status = 'Saved'
    } catch {
      status = 'Error saving'
    }
    saving = false
  }
</script>

<PageWrapper>
  <PageHeader title="Work Authorization" />

  <div class="settings-form">
    <div class="field-group">
      <label>Are you authorized to work in the United States?</label>
      <div class="radio-group">
        <label><input type="radio" bind:group={workAuthUs} value="Yes" /> Yes</label>
        <label><input type="radio" bind:group={workAuthUs} value="No" /> No</label>
      </div>
    </div>

    <div class="field-group">
      <label>Will you now or in the future require sponsorship?</label>
      <div class="radio-group">
        <label><input type="radio" bind:group={sponsorship} value="Yes" /> Yes</label>
        <label><input type="radio" bind:group={sponsorship} value="No" /> No</label>
      </div>
    </div>

    <div class="actions">
      <button class="btn btn-primary" onclick={save} disabled={saving}>
        {saving ? 'Saving...' : 'Save'}
      </button>
      {#if status}
        <span class="status" class:ok={status === 'Saved'}>{status}</span>
      {/if}
    </div>
  </div>
</PageWrapper>

<style>
  .settings-form {
    max-width: 500px;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  .field-group label:first-child {
    display: block;
    font-weight: 600;
    margin-bottom: 0.5rem;
    color: var(--color-text);
  }
  .radio-group {
    display: flex;
    gap: 1.5rem;
  }
  .radio-group label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-weight: 400;
    cursor: pointer;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .status {
    font-size: 0.85rem;
    color: var(--color-danger);
  }
  .status.ok {
    color: var(--color-success);
  }
</style>
```

- [ ] **Step 3: Write EEO settings page**

```svelte
<!-- packages/webui/src/routes/settings/eeo/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte'
  import { PageWrapper, PageHeader } from '$lib/components'

  const API_BASE = 'http://localhost:3000/api'

  interface AnswerEntry {
    field_kind: string
    label: string
    value: string
  }

  let gender = $state('')
  let race = $state('')
  let veteran = $state('')
  let disability = $state('')
  let saving = $state(false)
  let status = $state<string | null>(null)

  const genderOptions = ['Male', 'Female', 'Non-binary', 'Prefer not to say', 'Decline to self-identify']
  const raceOptions = [
    'American Indian or Alaska Native',
    'Asian',
    'Black or African American',
    'Hispanic or Latino',
    'Native Hawaiian or Other Pacific Islander',
    'White',
    'Two or more races',
    'Prefer not to say',
    'Decline to self-identify',
  ]
  const veteranOptions = ['I am a protected veteran', 'I am not a protected veteran', 'Prefer not to say']
  const disabilityOptions = ['Yes, I have a disability', 'No, I do not have a disability', 'Prefer not to say']

  onMount(async () => {
    const res = await fetch(`${API_BASE}/profile/answers`)
    if (res.ok) {
      const body = await res.json()
      const entries = body.data as AnswerEntry[]
      for (const e of entries) {
        if (e.field_kind === 'eeo.gender') gender = e.value
        if (e.field_kind === 'eeo.race') race = e.value
        if (e.field_kind === 'eeo.veteran') veteran = e.value
        if (e.field_kind === 'eeo.disability') disability = e.value
      }
    }
  })

  async function save() {
    saving = true
    status = null

    const upsert = async (field_kind: string, label: string, value: string) => {
      if (!value) return
      await fetch(`${API_BASE}/profile/answers`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ field_kind, label, value }),
      })
    }

    try {
      await upsert('eeo.gender', 'Gender', gender)
      await upsert('eeo.race', 'Race / Ethnicity', race)
      await upsert('eeo.veteran', 'Veteran Status', veteran)
      await upsert('eeo.disability', 'Disability Status', disability)
      status = 'Saved'
    } catch {
      status = 'Error saving'
    }
    saving = false
  }
</script>

<PageWrapper>
  <PageHeader title="EEO Voluntary Disclosures" />

  <div class="settings-form">
    <p class="disclaimer">
      These responses are used to auto-fill voluntary disclosure fields on job applications.
      They are stored locally in your Forge database and never shared externally.
    </p>

    <div class="field-group">
      <label for="gender">Gender</label>
      <select id="gender" bind:value={gender}>
        <option value="">-- Select --</option>
        {#each genderOptions as opt}
          <option value={opt}>{opt}</option>
        {/each}
      </select>
    </div>

    <div class="field-group">
      <label for="race">Race / Ethnicity</label>
      <select id="race" bind:value={race}>
        <option value="">-- Select --</option>
        {#each raceOptions as opt}
          <option value={opt}>{opt}</option>
        {/each}
      </select>
    </div>

    <div class="field-group">
      <label for="veteran">Veteran Status</label>
      <select id="veteran" bind:value={veteran}>
        <option value="">-- Select --</option>
        {#each veteranOptions as opt}
          <option value={opt}>{opt}</option>
        {/each}
      </select>
    </div>

    <div class="field-group">
      <label for="disability">Disability Status</label>
      <select id="disability" bind:value={disability}>
        <option value="">-- Select --</option>
        {#each disabilityOptions as opt}
          <option value={opt}>{opt}</option>
        {/each}
      </select>
    </div>

    <div class="actions">
      <button class="btn btn-primary" onclick={save} disabled={saving}>
        {saving ? 'Saving...' : 'Save'}
      </button>
      {#if status}
        <span class="status" class:ok={status === 'Saved'}>{status}</span>
      {/if}
    </div>
  </div>
</PageWrapper>

<style>
  .settings-form {
    max-width: 500px;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  .disclaimer {
    font-size: 0.85rem;
    color: var(--color-text-muted);
    line-height: 1.5;
    margin: 0;
  }
  .field-group label {
    display: block;
    font-weight: 600;
    margin-bottom: 0.4rem;
    color: var(--color-text);
  }
  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    background: var(--color-bg-input);
    color: var(--color-text);
    font-size: 0.9rem;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .status {
    font-size: 0.85rem;
    color: var(--color-danger);
  }
  .status.ok {
    color: var(--color-success);
  }
</style>
```

- [ ] **Step 4: Add settings routes to WebUI navigation**

Check `packages/webui/src/lib/` for the sidebar/navigation component and add links for `/settings/work-auth` and `/settings/eeo`. This depends on the existing navigation structure — add entries under a "Settings" group.

- [ ] **Step 5: Commit**

```bash
git add packages/webui/src/routes/settings/
git commit -m "feat(webui): add work-auth and EEO settings pages (M6)"
```

---

## Task 16: Version Bump + Manifest Update

**Files:**
- Modify: `packages/extension/manifest.json`
- Modify: `packages/extension/manifest.firefox.json`

- [ ] **Step 1: Bump version in both manifests**

In both `manifest.json` and `manifest.firefox.json`, change:
```json
"version": "0.1.5"
```
to:
```json
"version": "0.1.6"
```

- [ ] **Step 2: Commit**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json
git commit -m "chore(ext): bump version to 0.1.6 (M6)"
```

---

## Task 17: Build + Full Test Suite

**Files:** None (verification only)

- [ ] **Step 1: Run core tests**

Run: `cd packages/core && bun test`
Expected: All PASS

- [ ] **Step 2: Run SDK tests**

Run: `cd packages/sdk && bun test`
Expected: All PASS

- [ ] **Step 3: Run extension tests**

Run: `cd packages/extension && bun test`
Expected: All PASS

- [ ] **Step 4: Build extension (both browsers)**

Run: `cd packages/extension && bun run build`
Expected: Both `dist/chrome/` and `dist/firefox/` produced. No shared chunks in content scripts.

- [ ] **Step 5: Verify build output**

Run: `cd packages/extension && bun test tests/build/`
Expected: All PASS (no import statements in content scripts, manifests in parity)

- [ ] **Step 6: Final commit if any fixes were needed**

---

## Task Dependency Graph

```
Task 1 (migration)
  └─► Task 2 (service)
       └─► Task 3 (API routes + tests)
            └─► Task 4 (SDK resource)
                 ├─► Task 13 (autofill merge)
                 └─► Task 15 (WebUI pages)

Task 5 (confidence model)
  └─► Task 6 (enrichment + confidence)
       └─► Task 10 (linkedin.ts overlay integration)

Task 7 (toast)  ──┐
Task 8 (overlay) ─┤
                   └─► Task 10 (linkedin.ts overlay integration)
                        └─► Task 11 (inject-button Shift+click)

Task 9 (background handlers + messaging) ──► Task 10

Task 12 (Workday field detection) ──► Task 13

Task 14 (build verification) ──► Task 17
Task 16 (version bump) ──► Task 17

All tasks ──► Task 17 (final verification)
```

**Parallelizable groups:**
- Tasks 1-4 (backend pipeline) can run in parallel with Tasks 5-8 (extension confidence + UI)
- Task 12 (Workday fields) is independent until Task 13
- Task 15 (WebUI) only depends on Task 4
