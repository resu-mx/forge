# M5a — Parser Wiring + Core Persist Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the M3 parser into extension capture and MCP ingest so parsed sections, salary, locations, and work posture are auto-populated and persisted on every JD.

**Architecture:** Content script calls `parseJobDescription()` on extracted description, enriches the payload with parsed fields (salary, locations, posture, classified sections). Background forwards them to SDK create. MCP ingest tool also calls parser server-side. Four new DB columns persist the parsed data. Backfill script re-parses existing JDs.

**Tech Stack:** TypeScript, Bun, Vite, SQLite, MCP SDK

**Spec:** `.claude/plans/forge-resume-browser-extension/refs/specs/2026-04-20-M5a-parser-wiring-design.md`

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Create | `packages/core/src/db/migrations/049_jd_parsed_fields.sql` | Add 4 columns to job_descriptions |
| Create | `packages/core/src/scripts/backfill-parser.ts` | Re-parse existing JDs |
| Create | `packages/extension/src/lib/enrich-extraction.ts` | Parser enrichment helper |
| Create | `packages/extension/tests/lib/enrich-extraction.test.ts` | Test enrichment |
| Modify | `packages/core/src/types/index.ts:151-228` | Add fields to JD interfaces |
| Modify | `packages/core/src/storage/entity-map.data.ts:855-871` | Add fields to ELM entity map |
| Modify | `packages/core/src/services/job-description-service.ts:111-121,253-263` | Pass new fields in create/update |
| Modify | `packages/sdk/src/types.ts:503-547` | Mirror core type changes |
| Modify | `packages/mcp/src/tools/tier2-jd.ts` | Import parser, wire into ingest |
| Modify | `packages/extension/package.json` | Add `@forge/core` workspace dep |
| Modify | `packages/extension/src/plugin/types.ts:13-25` | Add parsed fields to ExtractedJob |
| Modify | `packages/extension/src/content/linkedin.ts` | Call enrichWithParser before response |
| Modify | `packages/extension/src/background/handlers/capture.ts:127-134` | Forward parsed fields to SDK |
| Modify | `packages/extension/manifest.json` | Bump version to 0.1.5 |
| Modify | `packages/extension/manifest.firefox.json` | Bump version to 0.1.5 |

---

### Task 1: DB Migration + ELM Entity Map

**Files:**
- Create: `packages/core/src/db/migrations/049_jd_parsed_fields.sql`
- Modify: `packages/core/src/storage/entity-map.data.ts:855-871`

- [ ] **Step 1: Write migration SQL**

```sql
-- 049_jd_parsed_fields.sql
-- M5a: Add parser-derived columns to job_descriptions

ALTER TABLE job_descriptions ADD COLUMN parsed_sections TEXT;
ALTER TABLE job_descriptions ADD COLUMN work_posture TEXT;
ALTER TABLE job_descriptions ADD COLUMN parsed_locations TEXT;
ALTER TABLE job_descriptions ADD COLUMN salary_period TEXT;
```

- [ ] **Step 2: Update ELM entity map**

In `packages/core/src/storage/entity-map.data.ts`, find the `job_descriptions` entity (around line 854). Add 4 fields after `location`:

```typescript
      location: { type: 'text' },
      parsed_sections: { type: 'text' },
      work_posture: { type: 'text' },
      parsed_locations: { type: 'text' },
      salary_period: { type: 'text' },
      created_at: CREATED_AT,
```

- [ ] **Step 3: Run migration**

Run: `cd packages/core && bun run src/db/migrate.ts`
Expected: Migration 049 applied successfully, no errors.

- [ ] **Step 4: Verify columns exist**

Run: `sqlite3 data/forge.db ".schema job_descriptions" | grep -E "parsed_sections|work_posture|parsed_locations|salary_period"`
Expected: All 4 columns visible in schema output.

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/db/migrations/049_jd_parsed_fields.sql packages/core/src/storage/entity-map.data.ts
git commit -m "schema(core): add parser-derived columns to job_descriptions (M5a)"
```

---

### Task 2: Core Types + JD Service

**Files:**
- Modify: `packages/core/src/types/index.ts:151-228`
- Modify: `packages/core/src/services/job-description-service.ts:111-121,253-263`

- [ ] **Step 1: Update JobDescription interface**

In `packages/core/src/types/index.ts`, add 4 fields to `JobDescription` (after `location`, before `created_at` — around line 161):

```typescript
export interface JobDescription {
  id: string
  organization_id: string | null
  title: string
  url: string | null
  raw_text: string
  status: JobDescriptionStatus
  salary_range: string | null
  salary_min: number | null
  salary_max: number | null
  location: string | null
  parsed_sections: string | null
  work_posture: string | null
  parsed_locations: string | null
  salary_period: string | null
  created_at: string
  updated_at: string
}
```

- [ ] **Step 2: Update CreateJobDescription interface**

Add optional fields (after `location` — around line 214):

```typescript
export interface CreateJobDescription {
  title: string
  organization_id?: string
  url?: string
  raw_text: string
  status?: JobDescriptionStatus
  salary_range?: string
  salary_min?: number
  salary_max?: number
  location?: string
  parsed_sections?: string
  work_posture?: string
  parsed_locations?: string
  salary_period?: string
}
```

- [ ] **Step 3: Update UpdateJobDescription interface**

Add optional nullable fields (after `location` — around line 228):

```typescript
export interface UpdateJobDescription {
  title?: string
  organization_id?: string | null
  url?: string | null
  raw_text?: string
  status?: JobDescriptionStatus
  salary_range?: string | null
  salary_min?: number | null
  salary_max?: number | null
  location?: string | null
  parsed_sections?: string | null
  work_posture?: string | null
  parsed_locations?: string | null
  salary_period?: string | null
}
```

- [ ] **Step 4: Update JD service create()**

In `packages/core/src/services/job-description-service.ts`, update the `elm.create` call (around line 111). Add 4 fields after `location`:

```typescript
    const createResult = await this.elm.create('job_descriptions', {
      organization_id: input.organization_id ?? null,
      title: input.title,
      url: input.url ?? null,
      raw_text: input.raw_text,
      status: input.status ?? 'discovered',
      salary_range: input.salary_range ?? null,
      salary_min: input.salary_min ?? null,
      salary_max: input.salary_max ?? null,
      location: input.location ?? null,
      parsed_sections: input.parsed_sections ?? null,
      work_posture: input.work_posture ?? null,
      parsed_locations: input.parsed_locations ?? null,
      salary_period: input.salary_period ?? null,
    })
```

- [ ] **Step 5: Update JD service update()**

In the same file, update the patch-building block in `update()` (around line 253). Add after the `location` line:

```typescript
    if (input.parsed_sections !== undefined) patch.parsed_sections = input.parsed_sections
    if (input.work_posture !== undefined) patch.work_posture = input.work_posture
    if (input.parsed_locations !== undefined) patch.parsed_locations = input.parsed_locations
    if (input.salary_period !== undefined) patch.salary_period = input.salary_period
```

- [ ] **Step 6: Run core tests**

Run: `cd packages/core && bun test`
Expected: All existing tests pass. No new tests yet — we verify round-trip in Task 4.

- [ ] **Step 7: Commit**

```bash
git add packages/core/src/types/index.ts packages/core/src/services/job-description-service.ts
git commit -m "feat(core): add parser fields to JD types and service (M5a)"
```

---

### Task 3: SDK Types

**Files:**
- Modify: `packages/sdk/src/types.ts:503-547`

- [ ] **Step 1: Update SDK JobDescription interface**

In `packages/sdk/src/types.ts`, add 4 fields to `JobDescription` (after `location`, before `created_at` — around line 513):

```typescript
export interface JobDescription {
  id: string
  organization_id: string | null
  title: string
  url: string | null
  raw_text: string
  status: JobDescriptionStatus
  salary_range: string | null
  salary_min: number | null
  salary_max: number | null
  location: string | null
  parsed_sections: string | null
  work_posture: string | null
  parsed_locations: string | null
  salary_period: string | null
  created_at: string
  updated_at: string
}
```

- [ ] **Step 2: Update SDK CreateJobDescription**

Add optional fields (after `location` — around line 533):

```typescript
export interface CreateJobDescription {
  title: string
  organization_id?: string
  url?: string
  raw_text: string
  status?: JobDescriptionStatus
  salary_range?: string
  salary_min?: number
  salary_max?: number
  location?: string
  parsed_sections?: string
  work_posture?: string
  parsed_locations?: string
  salary_period?: string
}
```

- [ ] **Step 3: Update SDK UpdateJobDescription**

Add optional nullable fields (after `location` — around line 546):

```typescript
export interface UpdateJobDescription {
  title?: string
  organization_id?: string | null
  url?: string | null
  raw_text?: string
  status?: JobDescriptionStatus
  salary_range?: string | null
  salary_min?: number | null
  salary_max?: number | null
  location?: string | null
  parsed_sections?: string | null
  work_posture?: string | null
  parsed_locations?: string | null
  salary_period?: string | null
}
```

- [ ] **Step 4: Commit**

```bash
git add packages/sdk/src/types.ts
git commit -m "feat(sdk): add parser fields to JD types (M5a)"
```

---

### Task 4: MCP Ingest Parser Wiring

**Files:**
- Modify: `packages/mcp/src/tools/tier2-jd.ts`
- Test: `packages/mcp/tests/` (find existing JD ingest tests, or create new)

- [ ] **Step 1: Write failing test — ingest auto-populates parser fields**

Find existing MCP test patterns by searching `packages/mcp/tests/` for JD-related tests. Create or extend a test file. The test should call `forge_ingest_job_description` with raw_text containing salary info and verify the response includes parsed fields:

```typescript
import { describe, test, expect } from 'bun:test'

// Test that ingest auto-populates parser-derived fields
describe('forge_ingest_job_description parser wiring', () => {
  test('auto-populates salary from raw_text', async () => {
    const rawText = `
## About the Role
Senior Software Engineer at Acme Corp.

## Compensation
The salary range for this role is $150,000 - $200,000 per year.

## Location
San Francisco, CA (Hybrid)

## Requirements
- 5+ years experience
`
    // Call ingest (via SDK or direct handler — match existing test patterns)
    const result = await sdk.jobDescriptions.create({
      title: 'Senior Software Engineer',
      raw_text: rawText,
    })

    expect(result.ok).toBe(true)
    if (!result.ok) return

    expect(result.data.salary_min).toBe(150000)
    expect(result.data.salary_max).toBe(200000)
    expect(result.data.salary_period).toBe('annual')
    expect(result.data.work_posture).toBe('hybrid')
    expect(result.data.parsed_locations).toBeTruthy()
    const locations = JSON.parse(result.data.parsed_locations!)
    expect(locations).toContain('San Francisco, CA')
    expect(result.data.parsed_sections).toBeTruthy()
    const sections = JSON.parse(result.data.parsed_sections!)
    expect(sections.length).toBeGreaterThan(0)
  })
})
```

Adapt this to match the existing test infrastructure (test DB setup, SDK initialization, etc.).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/mcp && bun test` (or the specific test file)
Expected: FAIL — salary_min/salary_max/parsed_sections are null because parser isn't wired yet.

- [ ] **Step 3: Wire parser into MCP ingest handler**

In `packages/mcp/src/tools/tier2-jd.ts`, add the parser import at the top:

```typescript
import { parseJobDescription } from '@forge/core/src/parser'
```

Replace the `forge_ingest_job_description` handler (the `async (params)` callback) with:

```typescript
    async (params) => {
      // M5a: Run parser on raw_text to auto-populate structured fields
      const parsed = parseJobDescription(params.raw_text)

      const result = await sdk.jobDescriptions.create({
        title: params.title,
        raw_text: params.raw_text,
        organization_id: params.organization_id,
        url: params.url,
        status: params.status,
        salary_range: params.salary_range,
        location: params.location,
        // Parser-derived fields
        salary_min: parsed.salary?.min,
        salary_max: parsed.salary?.max,
        salary_period: parsed.salary?.period,
        work_posture: parsed.workPosture ?? undefined,
        parsed_locations: parsed.locations.length > 0 ? JSON.stringify(parsed.locations) : undefined,
        parsed_sections: JSON.stringify(parsed.sections),
      })
      if (!result.ok) {
        return respond(result)
      }
      const data = {
        ...result.data,
        embedding_status: (result.data as any).embedding_status ?? null,
      }
      return respond({ ok: true, data })
    },
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd packages/mcp && bun test`
Expected: PASS — parser auto-populates all fields.

- [ ] **Step 5: Run full test suite**

Run: `cd packages/core && bun test && cd ../mcp && bun test`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add packages/mcp/src/tools/tier2-jd.ts packages/mcp/tests/
git commit -m "feat(mcp): wire parser into JD ingest for auto-populated fields (M5a)"
```

---

### Task 5: Backfill Script

**Files:**
- Create: `packages/core/src/scripts/backfill-parser.ts`

- [ ] **Step 1: Write backfill script**

```typescript
// packages/core/src/scripts/backfill-parser.ts
//
// Re-parse existing JDs through the M3 parser to populate
// parsed_sections, work_posture, parsed_locations, salary_period,
// and (where missing) salary_min/salary_max.
//
// Usage: bun run packages/core/src/scripts/backfill-parser.ts [db-path]
// Default db-path: data/forge.db

import { Database } from 'bun:sqlite'
import { parseJobDescription } from '../parser'

const dbPath = process.argv[2] ?? 'data/forge.db'
const db = new Database(dbPath)

interface JDRow {
  id: string
  raw_text: string
}

const rows = db
  .query<JDRow, []>(
    'SELECT id, raw_text FROM job_descriptions WHERE parsed_sections IS NULL AND raw_text IS NOT NULL',
  )
  .all()

console.log(`Backfilling ${rows.length} job descriptions...`)

const update = db.prepare(`
  UPDATE job_descriptions
  SET parsed_sections = ?1,
      work_posture = ?2,
      parsed_locations = ?3,
      salary_period = ?4,
      salary_min = COALESCE(salary_min, ?5),
      salary_max = COALESCE(salary_max, ?6),
      updated_at = ?7
  WHERE id = ?8
`)

let count = 0
for (const row of rows) {
  const parsed = parseJobDescription(row.raw_text)
  update.run(
    JSON.stringify(parsed.sections),
    parsed.workPosture,
    JSON.stringify(parsed.locations),
    parsed.salary?.period ?? null,
    parsed.salary?.min ?? null,
    parsed.salary?.max ?? null,
    new Date().toISOString(),
    row.id,
  )
  count++
}

console.log(`Done. Updated ${count} job descriptions.`)
db.close()
```

- [ ] **Step 2: Run backfill**

Run: `bun run packages/core/src/scripts/backfill-parser.ts data/forge.db`
Expected: Prints count of updated JDs, no errors.

- [ ] **Step 3: Verify backfill results**

Run: `sqlite3 data/forge.db "SELECT id, work_posture, parsed_locations, salary_min, salary_max FROM job_descriptions WHERE parsed_sections IS NOT NULL LIMIT 5"`
Expected: Rows show populated parsed fields.

- [ ] **Step 4: Commit**

```bash
git add packages/core/src/scripts/backfill-parser.ts
git commit -m "feat(core): add parser backfill script for existing JDs (M5a)"
```

---

### Task 6: Extension Parser Wiring + Tests

**Files:**
- Modify: `packages/extension/package.json`
- Modify: `packages/extension/src/plugin/types.ts:13-25`
- Create: `packages/extension/src/lib/enrich-extraction.ts`
- Create: `packages/extension/tests/lib/enrich-extraction.test.ts`
- Modify: `packages/extension/src/content/linkedin.ts`
- Modify: `packages/extension/src/background/handlers/capture.ts:127-134`

- [ ] **Step 1: Add @forge/core workspace dependency**

In `packages/extension/package.json`, add `@forge/core` to `dependencies`:

```json
  "dependencies": {
    "svelte": "^5.0.0",
    "@forge/sdk": "workspace:*",
    "@forge/core": "workspace:*"
  },
```

Then run: `cd packages/extension && bun install`

- [ ] **Step 2: Update ExtractedJob type**

In `packages/extension/src/plugin/types.ts`, add parsed fields to `ExtractedJob` (after `company_url`, before the closing `}`):

```typescript
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
  apply_url?: string | null          // External apply link (decoded from LinkedIn redirect)
  company_url?: string | null        // Company profile URL (e.g. linkedin.com/company/anthropic)
  // Parser-derived fields (M5a)
  salary_min?: number | null
  salary_max?: number | null
  salary_period?: string | null
  work_posture?: string | null
  parsed_locations?: string[]
  parsed_sections?: string           // JSON-serialized ClassifiedSection[]
}
```

- [ ] **Step 3: Write enrichment test**

Create `packages/extension/tests/lib/enrich-extraction.test.ts`:

```typescript
import { describe, test, expect } from 'bun:test'
import { enrichWithParser } from '../../src/lib/enrich-extraction'
import type { ExtractedJob } from '../../src/plugin/types'

function makeExtracted(overrides: Partial<ExtractedJob> = {}): ExtractedJob {
  return {
    title: 'Senior Engineer',
    company: 'Acme',
    location: 'San Francisco, CA',
    salary_range: null,
    description: null,
    url: 'https://example.com/job/1',
    extracted_at: new Date().toISOString(),
    source_plugin: 'test',
    ...overrides,
  }
}

describe('enrichWithParser', () => {
  test('returns unchanged when description is null', () => {
    const input = makeExtracted({ description: null })
    const result = enrichWithParser(input)
    expect(result.parsed_sections).toBeUndefined()
    expect(result.salary_min).toBeUndefined()
  })

  test('extracts salary from description body', () => {
    const input = makeExtracted({
      description: `
## Compensation
The salary range is $150,000 - $200,000 per year.

## Requirements
5+ years experience with TypeScript.
`,
    })
    const result = enrichWithParser(input)
    expect(result.salary_min).toBe(150000)
    expect(result.salary_max).toBe(200000)
    expect(result.salary_period).toBe('annual')
  })

  test('extracts work posture', () => {
    const input = makeExtracted({
      description: `
## About the Role
This is a fully remote position.

## Responsibilities
Build great software.
`,
    })
    const result = enrichWithParser(input)
    expect(result.work_posture).toBe('remote')
  })

  test('extracts multiple locations', () => {
    const input = makeExtracted({
      description: `
## Location
Positions available in San Francisco, CA and Austin, TX.

## About
A great company.
`,
    })
    const result = enrichWithParser(input)
    expect(result.parsed_locations).toContain('San Francisco, CA')
    expect(result.parsed_locations).toContain('Austin, TX')
  })

  test('populates parsed_sections as JSON string', () => {
    const input = makeExtracted({
      description: `
## Requirements
5+ years experience.

## Benefits
Health insurance.
`,
    })
    const result = enrichWithParser(input)
    expect(result.parsed_sections).toBeTruthy()
    const sections = JSON.parse(result.parsed_sections!)
    expect(Array.isArray(sections)).toBe(true)
    expect(sections.length).toBeGreaterThan(0)
    expect(sections[0]).toHaveProperty('category')
    expect(sections[0]).toHaveProperty('confidence')
  })
})
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cd packages/extension && bun test tests/lib/enrich-extraction.test.ts`
Expected: FAIL — module `../../src/lib/enrich-extraction` not found.

- [ ] **Step 5: Write enrichment implementation**

Create `packages/extension/src/lib/enrich-extraction.ts`:

```typescript
// packages/extension/src/lib/enrich-extraction.ts
//
// Runs the M3 parser on an extracted JD to populate structured fields.
// Used by content scripts before sending extraction to background.
// Pure function — safe to import in content script IIFE bundles
// (Vite inlines the parser's pure functions).

import { parseJobDescription } from '@forge/core/src/parser'
import type { ExtractedJob } from '../plugin/types'

export function enrichWithParser(extracted: ExtractedJob): ExtractedJob {
  if (!extracted.description) return extracted

  const parsed = parseJobDescription(extracted.description)

  return {
    ...extracted,
    salary_min: parsed.salary?.min ?? null,
    salary_max: parsed.salary?.max ?? null,
    salary_period: parsed.salary?.period ?? null,
    work_posture: parsed.workPosture,
    parsed_locations: parsed.locations,
    parsed_sections: JSON.stringify(parsed.sections),
  }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cd packages/extension && bun test tests/lib/enrich-extraction.test.ts`
Expected: All 5 tests PASS.

- [ ] **Step 7: Wire enrichment into linkedin.ts content script**

In `packages/extension/src/content/linkedin.ts`, add the import at the top (after the existing import):

```typescript
import { enrichWithParser } from '../lib/enrich-extraction'
```

Then in the `extract` message handler, enrich the result before sending the response. Replace:

```typescript
        sendResponse({ ok: true, data: result })
```

with:

```typescript
        sendResponse({ ok: true, data: enrichWithParser(result) })
```

- [ ] **Step 8: Update capture handler to forward parsed fields**

In `packages/extension/src/background/handlers/capture.ts`, update the SDK create call (around line 127). Replace the existing `client.jobDescriptions.create(...)` block:

```typescript
    const result = await client.jobDescriptions.create({
      title: extracted.title!,
      raw_text: extracted.description!,
      url: canonicalUrl,
      location: extracted.location ?? undefined,
      salary_range: extracted.salary_range ?? undefined,
      organization_id: organizationId ?? undefined,
      // M5a: Parser-derived fields from content script enrichment
      salary_min: extracted.salary_min ?? undefined,
      salary_max: extracted.salary_max ?? undefined,
      salary_period: extracted.salary_period ?? undefined,
      work_posture: extracted.work_posture ?? undefined,
      parsed_locations: extracted.parsed_locations?.length
        ? JSON.stringify(extracted.parsed_locations)
        : undefined,
      parsed_sections: extracted.parsed_sections ?? undefined,
    })
```

- [ ] **Step 9: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: All tests pass (existing 119 + 5 new enrichment tests).

- [ ] **Step 10: Commit**

```bash
git add packages/extension/package.json packages/extension/bun.lockb \
  packages/extension/src/plugin/types.ts \
  packages/extension/src/lib/enrich-extraction.ts \
  packages/extension/tests/lib/enrich-extraction.test.ts \
  packages/extension/src/content/linkedin.ts \
  packages/extension/src/background/handlers/capture.ts
git commit -m "feat(ext): wire parser into capture flow for auto-populated JD fields (M5a)"
```

---

### Task 7: Build Verification + Version Bump

**Files:**
- Modify: `packages/extension/manifest.json`
- Modify: `packages/extension/manifest.firefox.json`

- [ ] **Step 1: Build both browsers**

Run: `cd packages/extension && bun run build`
Expected: Both `dist/chrome/` and `dist/firefox/` built successfully.

- [ ] **Step 2: Verify content script IIFE integrity**

Run: `cd packages/extension && bun test tests/build/output.test.ts`
Expected: PASS — no `import` statements leaked into content script bundles. The parser's pure functions are inlined by Vite.

- [ ] **Step 3: Run full test suite across all packages**

Run: `cd packages/core && bun test && cd ../sdk && bun test && cd ../mcp && bun test && cd ../extension && bun test`
Expected: All tests pass across all 4 packages.

- [ ] **Step 4: Bump version to 0.1.5**

In `packages/extension/manifest.json`, change:
```json
"version": "0.1.5"
```

In `packages/extension/manifest.firefox.json`, change:
```json
"version": "0.1.5"
```

- [ ] **Step 5: Final build with new version**

Run: `cd packages/extension && bun run build`
Expected: Clean build, both browsers.

- [ ] **Step 6: Commit**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json \
  packages/extension/dist/
git commit -m "chore(ext): bump version to 0.1.5 (M5a)"
```
