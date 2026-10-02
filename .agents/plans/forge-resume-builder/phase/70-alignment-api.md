# Phase 70: Alignment & Health API

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-mcp-server-design.md](../refs/specs/2026-04-03-mcp-server-design.md) (Alignment Routes, SDK Alignment Resource, Embedding Service Design, Health Route sections)
**Depends on:** Phase 69 (EmbeddingService -- `embed()`, `alignResume()`, `matchRequirements()`, `checkStale()`, embeddings table migration 020+, `@xenova/transformers` integration)
**Blocks:** MCP Server implementation (`forge_align_resume`, `forge_match_requirements`, `forge_health` tools)
**Parallel with:** Independent of UI phases (59-68). Independent of Phase 60-62 (JD linkage/skills) unless those modify shared files concurrently.
**Duration:** Short-Medium (6-8 tasks, no migrations, no UI)

## Goal

Expose the alignment scoring engine (built in Phase 69's EmbeddingService) and a health check endpoint through `@forge/core` HTTP routes and `@forge/sdk` client methods. After this phase, any consumer (MCP server, CLI, WebUI) can:

1. **Score a resume against a JD** -- `GET /api/alignment/score` returns an `AlignmentReport` with per-requirement match verdicts (strong/adjacent/gap), an overall weighted score, and unmatched resume noise.
2. **Discover best-matching content for a JD** -- `GET /api/alignment/match` returns a `RequirementMatchReport` with top N candidate bullets or perspectives per requirement, independent of any specific resume.
3. **Verify server connectivity** -- `GET /api/health` returns server status and version, enabling the MCP `forge_health` tool and general uptime checks.

## Non-Goals

- Embedding computation or storage (Phase 69 -- EmbeddingService owns that)
- JD requirement parsing (Phase 69 -- `parseRequirements()` is part of the embedding pipeline)
- MCP tool registration (future MCP Server phase -- this phase provides the SDK methods those tools delegate to)
- WebUI alignment visualization (future phase)
- LLM-based alignment or reasoning (this is purely programmatic cosine-similarity)
- Caching alignment reports (computed on-the-fly; caching is a future optimization)
- Custom weighting of requirements (all requirements weighted equally in v1)
- Batch alignment (scoring multiple resumes against a JD in a single call)
- Version compatibility checking between MCP server and API is deferred.

## Context

Phase 69 creates the `EmbeddingService` class in `@forge/core` with these methods:

```typescript
class EmbeddingService {
  alignResume(jdId, resumeId, opts?): Promise<Result<AlignmentReport>>
  matchRequirements(jdId, entityType, opts?): Promise<Result<RequirementMatchReport>>
  // ... other methods (embed, findSimilar, checkStale, etc.)
}
```

This phase wraps those service methods in HTTP routes and SDK resource methods, following established patterns:

- **Route pattern:** Hono route file exporting a factory function that receives `Services`, following `auditRoutes()` / `integrityRoutes()` conventions.
- **SDK resource pattern:** Class receiving bound `request` / `requestList` functions, following `IntegrityResource` / `ReviewResource` conventions.
- **Error envelope:** `{ data: T }` on success, `{ error: { code, message } }` on failure, using `mapStatusCode()`.

The existing health check (`GET /api/health`) returns `{ status: 'ok' }`. This phase enhances it to include `version` from `package.json`, matching the MCP spec's `{ server: 'ok', version: string }` shape.

> **[C1 — Cross-phase note]:** The health response field is `version` (not `api_version`). Phase 71 must use `version` not `api_version` when parsing the health response. Update the MCP spec file and Phase 71 references accordingly.

### Overall Score Formula

> **[S1]** `overall_score = mean(best_match?.similarity ?? 0.0 for each requirement)`. In other words, for each requirement take the best_match similarity (or 0.0 if best_match is null), then compute the arithmetic mean across all requirements. This formula is used by `EmbeddingService.alignResume()` and verified in tests.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| Alignment HTTP routes (`GET /api/alignment/score`, `GET /api/alignment/match`) | Yes |
| Health HTTP route (`GET /api/health` enhanced with version) | Yes |
| `AlignmentReport`, `RequirementMatch`, `UnmatchedEntry`, `RequirementMatchReport` types | Yes |
| SDK `AlignmentResource` class | Yes |
| `ForgeClient.alignment` property | Yes |
| SDK barrel exports for new types/resource | Yes |
| EmbeddingService internals (cosine similarity, vector storage) | No -- Phase 69 |
| MCP tool wrappers (`forge_align_resume`, `forge_match_requirements`, `forge_health`) | No -- MCP Server phase |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/routes/alignment.ts` | Alignment HTTP routes (`/alignment/score`, `/alignment/match`) |
| `packages/core/src/routes/status-codes.ts` | Extracted `mapStatusCode` function shared by `server.ts` and `alignment.ts` |
| `packages/sdk/src/resources/alignment.ts` | SDK `AlignmentResource` class with `score()` and `matchRequirements()` methods |
| `packages/sdk/src/__tests__/type-sync.test.ts` | Structural type compatibility test between `@forge/core` and `@forge/sdk` alignment types |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `AlignmentReport`, `RequirementMatch`, `UnmatchedEntry`, `RequirementMatchReport`, `AlignmentScoreOptions`, `MatchRequirementsOptions` interfaces; add `STRONG_THRESHOLD_DEFAULT` and `ADJACENT_THRESHOLD_DEFAULT` constants |
| `packages/core/src/routes/server.ts` | Enhance health route to include `version`; import and mount `alignmentRoutes`; remove `mapStatusCode` (moved to `status-codes.ts`), re-import from `status-codes.ts`; add `MISSING_EMBEDDINGS` to `mapStatusCode` (→ 422) |
| `packages/core/src/services/index.ts` | Add `embedding?: EmbeddingService` to `Services` interface (if not already added by Phase 69) |
| `packages/core/src/index.ts` | Export alignment types from core barrel (if this file exists) |
| `packages/sdk/src/types.ts` | Add `AlignmentReport`, `RequirementMatch`, `UnmatchedEntry`, `RequirementMatchReport`, `AlignmentScoreOptions`, `MatchRequirementsOptions` types |
| `packages/sdk/src/client.ts` | Import `AlignmentResource`, add `alignment` property, instantiate in constructor |
| `packages/sdk/src/index.ts` | Export `AlignmentResource` class and alignment types |

## Fallback Strategies

- **EmbeddingService not initialized (Phase 69 not landed):** The alignment routes call `services.embedding.alignResume()` / `services.embedding.matchRequirements()`. If `services.embedding` is undefined, the route handler returns 503 with `{ code: 'SERVICE_UNAVAILABLE', message: 'Embedding service is not available. Ensure Phase 69 dependencies are installed.' }`. This lets the rest of the server start without the embedding service.
- **Embeddings missing for JD requirements:** `EmbeddingService.alignResume()` returns `{ ok: false, error: { code: 'MISSING_EMBEDDINGS', message: 'N of M JD requirements have no embeddings. Run embedding pipeline first.' } }`. The route maps `MISSING_EMBEDDINGS` to HTTP 422. The error message lists exactly which entities need embedding so the caller can take action.
- **Embeddings missing for resume perspectives:** Same pattern as above -- the error message specifies the resume entry IDs lacking embeddings.
- **JD not found:** EmbeddingService returns `{ ok: false, error: { code: 'NOT_FOUND', message: 'Job description {id} not found' } }`. Route returns 404.
- **Resume not found:** Same NOT_FOUND pattern.
- **No requirements parsed from JD:** `alignResume()` returns a valid `AlignmentReport` with `summary.total_requirements: 0`, `overall_score: 0`, empty `requirement_matches`, and all resume entries in `unmatched_entries`. This is not an error -- it's a meaningful result ("this JD has no parseable requirements").
- **Empty resume (no entries):** Returns a valid report with `summary.total_entries: 0`, all requirements as `gap`, `overall_score: 0`. Not an error.
- **Invalid threshold values:** The route handler clamps thresholds to [0.0, 1.0] and validates `strong_threshold > adjacent_threshold`. Returns 400 `VALIDATION_ERROR` if thresholds are inverted or equal.
- **Invalid entity_type for match route:** Returns 400 `VALIDATION_ERROR` listing accepted values (`bullet`, `perspective`).
- **package.json version read failure (health route):** Fall back to `'unknown'`. The health route must never fail.

---

## Tasks

### T70.1: Add AlignmentReport Types and Constants to Shared Types

**File:** `packages/core/src/types/index.ts`

[CRITICAL] These types define the contract between `EmbeddingService` (Phase 69), the HTTP routes (this phase), the SDK (this phase), and the MCP server (future phase). They must match the spec exactly. Any deviation here propagates through the entire stack.

[IMPORTANT] The `computed_at` field is an ISO-8601 timestamp generated at report creation time, not stored. It tells the consumer when the alignment was computed (useful for staleness detection if embeddings have been refreshed since).

Add after the existing result/entity type sections:

```typescript
// ── Alignment Constants ─────────────────────────────────────────────

/** Default threshold for "strong" match classification. */
export const STRONG_THRESHOLD_DEFAULT = 0.75

/** Default threshold for "adjacent" match classification. */
export const ADJACENT_THRESHOLD_DEFAULT = 0.50

// ── Alignment Types ──────────────────────────────────────────────────

/** Classification of a requirement match strength. */
export type MatchVerdict = 'strong' | 'adjacent' | 'gap'

/** A single JD requirement matched against the best resume entry. */
export interface RequirementMatch {
  requirement_text: string
  requirement_index: number
  best_match: {
    entry_id: string
    perspective_id: string
    perspective_content: string
    similarity: number
  } | null
  verdict: MatchVerdict
}

/** A resume entry that does not strongly match any JD requirement. */
export interface UnmatchedEntry {
  entry_id: string
  perspective_content: string
  best_requirement_similarity: number
}

/** Full alignment report comparing a resume against a JD. */
export interface AlignmentReport {
  job_description_id: string
  resume_id: string
  /**
   * Overall alignment score: arithmetic mean of best_match similarity across
   * all requirements. For requirements with no match (best_match === null),
   * similarity is treated as 0.0.
   *
   * Formula: mean(best_match?.similarity ?? 0.0 for each requirement)
   */
  overall_score: number
  requirement_matches: RequirementMatch[]
  unmatched_entries: UnmatchedEntry[]
  summary: {
    strong: number
    adjacent: number
    gaps: number
    total_requirements: number
    total_entries: number
  }
  computed_at: string
}

/**
 * Pre-resume discovery: top matching entities per JD requirement.
 * Independent of any specific resume.
 */
export interface RequirementMatchReport {
  job_description_id: string
  matches: Array<{
    requirement_text: string
    candidates: Array<{
      entity_id: string
      content: string
      similarity: number
    }>
  }>
  computed_at: string
}

/** Options for alignment score request. */
export interface AlignmentScoreOptions {
  strong_threshold?: number
  adjacent_threshold?: number
}

/** Options for requirement match request. */
export interface MatchRequirementsOptions {
  threshold?: number
  /** Range: [1, 100]. Server enforces ceiling of 100. */
  limit?: number
}
```

**Acceptance criteria:**
- All four types (`RequirementMatch`, `UnmatchedEntry`, `AlignmentReport`, `RequirementMatchReport`) are exported from `packages/core/src/types/index.ts`.
- `MatchVerdict` union type is exported.
- `AlignmentReport.computed_at` is typed as `string` (ISO-8601).
- `RequirementMatchReport.computed_at` is typed as `string` (ISO-8601).
- `RequirementMatch.best_match` is nullable (`| null`) for gap requirements with zero matches.
- `AlignmentReport.summary` counts are `number` (non-negative integers at runtime, enforced by EmbeddingService).
- `RequirementMatchReport.matches[].candidates` is sorted by descending similarity (enforced by EmbeddingService, documented here for contract clarity).
- `AlignmentScoreOptions` and `MatchRequirementsOptions` are exported from core types (not just SDK types).
- `STRONG_THRESHOLD_DEFAULT` and `ADJACENT_THRESHOLD_DEFAULT` are exported constants.
- `overall_score` JSDoc documents the exact formula.
- Types compile without errors under strict TypeScript.

**Failure criteria:**
- Missing `computed_at` on `AlignmentReport` -- MCP server spec expects it.
- Missing `computed_at` on `RequirementMatchReport` -- consumers need to know when the report was generated.
- Non-nullable `best_match` on `RequirementMatch` -- gaps have no match.
- Missing `overall_score` on `AlignmentReport` -- MCP tool returns it.
- Missing `AlignmentScoreOptions` or `MatchRequirementsOptions` from core types -- route code needs them.
- Magic number thresholds without named constants.

---

### T70.1b: Export Alignment Types from Core Barrel

**File:** `packages/core/src/index.ts` (if it exists)

Add alignment type and constant exports to the core barrel file so downstream consumers can import from `@forge/core` directly:

```typescript
// Alignment
export type {
  MatchVerdict,
  RequirementMatch,
  UnmatchedEntry,
  AlignmentReport,
  RequirementMatchReport,
  AlignmentScoreOptions,
  MatchRequirementsOptions,
} from './types'
export {
  STRONG_THRESHOLD_DEFAULT,
  ADJACENT_THRESHOLD_DEFAULT,
} from './types'
```

**Acceptance criteria:**
- If `packages/core/src/index.ts` exists, alignment types and constants are exported from it.
- If the barrel file does not exist, skip this task (note it for a future phase).

---

### T70.2: Mirror AlignmentReport Types in SDK Types

**File:** `packages/sdk/src/types.ts`

[IMPORTANT] The SDK types are standalone (no imports from `@forge/core`) to keep the SDK consumable in browser contexts. These must be identical copies of the core types. Any drift between core and SDK types will cause runtime contract violations.

> **[M1]** Verify `RequestFn` is exported from `packages/sdk/src/types.ts` (established in Phase 5). The `AlignmentResource` imports it.

Add after the existing type sections:

```typescript
// ---------------------------------------------------------------------------
// Alignment types
// ---------------------------------------------------------------------------

/** Classification of a requirement match strength. */
export type MatchVerdict = 'strong' | 'adjacent' | 'gap'

export interface RequirementMatch {
  requirement_text: string
  requirement_index: number
  best_match: {
    entry_id: string
    perspective_id: string
    perspective_content: string
    similarity: number
  } | null
  verdict: MatchVerdict
}

export interface UnmatchedEntry {
  entry_id: string
  perspective_content: string
  best_requirement_similarity: number
}

export interface AlignmentReport {
  job_description_id: string
  resume_id: string
  /**
   * Overall alignment score: arithmetic mean of best_match similarity across
   * all requirements. For requirements with no match (best_match === null),
   * similarity is treated as 0.0.
   *
   * Formula: mean(best_match?.similarity ?? 0.0 for each requirement)
   */
  overall_score: number
  requirement_matches: RequirementMatch[]
  unmatched_entries: UnmatchedEntry[]
  summary: {
    strong: number
    adjacent: number
    gaps: number
    total_requirements: number
    total_entries: number
  }
  computed_at: string
}

export interface RequirementMatchReport {
  job_description_id: string
  matches: Array<{
    requirement_text: string
    candidates: Array<{
      entity_id: string
      content: string
      similarity: number
    }>
  }>
  computed_at: string
}

/** Options for alignment score request. */
export interface AlignmentScoreOptions {
  strong_threshold?: number
  adjacent_threshold?: number
}

/** Options for requirement match request. */
export interface MatchRequirementsOptions {
  threshold?: number
  /** Range: [1, 100]. Server enforces ceiling of 100. */
  limit?: number
}
```

**Acceptance criteria:**
- SDK types are structurally identical to core types.
- `AlignmentScoreOptions` and `MatchRequirementsOptions` are defined (used by SDK resource methods).
- `RequirementMatchReport` includes `computed_at: string`.
- `MatchRequirementsOptions.limit` has JSDoc documenting the [1, 100] range.
- All types compile without imports from `@forge/core`.

---

### T70.2b: Extract mapStatusCode into Shared Module

**File to create:** `packages/core/src/routes/status-codes.ts`

> **[S3]** `alignment.ts` importing `mapStatusCode` from `server.ts` creates a circular dependency risk (server.ts imports alignment routes, alignment.ts imports from server.ts). Extract `mapStatusCode` into its own module.

```typescript
/**
 * Maps application error codes to HTTP status codes.
 * Shared by all route modules.
 */
export function mapStatusCode(code: string): number {
  switch (code) {
    case 'NOT_FOUND':
      return 404
    case 'VALIDATION_ERROR':
      return 400
    case 'CONFLICT':
      return 409
    case 'MISSING_EMBEDDINGS':
      return 422
    case 'SERVICE_UNAVAILABLE':
      return 503
    // ... preserve all existing cases from server.ts
    default:
      return 500
  }
}
```

Update `packages/core/src/routes/server.ts` to import from `./status-codes` instead of defining `mapStatusCode` inline. Update `packages/core/src/routes/alignment.ts` to import from `./status-codes`.

**Acceptance criteria:**
- `mapStatusCode` is defined in exactly one place: `packages/core/src/routes/status-codes.ts`.
- `server.ts` imports `mapStatusCode` from `./status-codes` (no inline definition).
- `alignment.ts` imports `mapStatusCode` from `./status-codes` (no import from `./server`).
- All existing error code → status mappings are preserved.
- New cases `MISSING_EMBEDDINGS → 422` and `SERVICE_UNAVAILABLE → 503` are included.

**Failure criteria:**
- Circular dependency between `alignment.ts` and `server.ts`.

---

### T70.3: Create Alignment HTTP Routes

**File:** `packages/core/src/routes/alignment.ts`

[CRITICAL] These routes are the HTTP contract that the SDK (and therefore the MCP server) depends on. Query parameter names must match the spec exactly: `jd_id`, `resume_id`, `strong_threshold`, `adjacent_threshold`, `entity_type`, `threshold`, `limit`.

[IMPORTANT] Threshold validation must enforce `strong_threshold > adjacent_threshold` (strictly greater than, NOT greater-than-or-equal) and both in [0.0, 1.0]. Invalid thresholds return 400 with a clear message explaining the constraint. Do NOT silently swap or clamp inverted thresholds -- the caller made a mistake and should know.

> **[AP3 — Future cleanup note]:** Consider adding a `requireEmbedding` middleware for the alignment routes sub-app that checks `services.embedding` once instead of per-handler. Not required for v1 but would reduce duplication.

Follow the `integrityRoutes()` / `auditRoutes()` pattern:

```typescript
/**
 * Alignment routes — JD↔Resume scoring and requirement matching.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './status-codes'
import { STRONG_THRESHOLD_DEFAULT, ADJACENT_THRESHOLD_DEFAULT } from '../types'

export function alignmentRoutes(services: Services) {
  const app = new Hono()

  // GET /alignment/score?jd_id=X&resume_id=Y&strong_threshold=0.75&adjacent_threshold=0.50
  app.get('/alignment/score', async (c) => {
    const jdId = c.req.query('jd_id')
    const resumeId = c.req.query('resume_id')

    if (!jdId || !resumeId) {
      return c.json({
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Both jd_id and resume_id query parameters are required',
        },
      }, 400)
    }

    const strongThreshold = parseFloat(
      c.req.query('strong_threshold') ?? String(STRONG_THRESHOLD_DEFAULT),
    )
    const adjacentThreshold = parseFloat(
      c.req.query('adjacent_threshold') ?? String(ADJACENT_THRESHOLD_DEFAULT),
    )

    // Validate thresholds
    if (isNaN(strongThreshold) || isNaN(adjacentThreshold)) {
      return c.json({
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Thresholds must be valid numbers',
        },
      }, 400)
    }

    if (strongThreshold < 0 || strongThreshold > 1 || adjacentThreshold < 0 || adjacentThreshold > 1) {
      return c.json({
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Thresholds must be between 0.0 and 1.0',
        },
      }, 400)
    }

    if (strongThreshold <= adjacentThreshold) {
      return c.json({
        error: {
          code: 'VALIDATION_ERROR',
          message: `strong_threshold (${strongThreshold}) must be greater than adjacent_threshold (${adjacentThreshold})`,
        },
      }, 400)
    }

    if (!services.embedding) {
      return c.json({
        error: {
          code: 'SERVICE_UNAVAILABLE',
          message: 'Embedding service is not available',
        },
      }, 503)
    }

    const result = await services.embedding.alignResume(jdId, resumeId, {
      strong_threshold: strongThreshold,
      adjacent_threshold: adjacentThreshold,
    })

    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // GET /alignment/match?jd_id=X&entity_type=perspective&threshold=0.50&limit=10
  app.get('/alignment/match', async (c) => {
    const jdId = c.req.query('jd_id')
    const entityType = c.req.query('entity_type')

    if (!jdId) {
      return c.json({
        error: {
          code: 'VALIDATION_ERROR',
          message: 'jd_id query parameter is required',
        },
      }, 400)
    }

    if (!entityType || !['bullet', 'perspective'].includes(entityType)) {
      return c.json({
        error: {
          code: 'VALIDATION_ERROR',
          message: 'entity_type must be "bullet" or "perspective"',
        },
      }, 400)
    }

    const threshold = parseFloat(c.req.query('threshold') ?? '0.50')
    const limit = parseInt(c.req.query('limit') ?? '10', 10)

    if (isNaN(threshold) || threshold < 0 || threshold > 1) {
      return c.json({
        error: {
          code: 'VALIDATION_ERROR',
          message: 'threshold must be a number between 0.0 and 1.0',
        },
      }, 400)
    }

    if (isNaN(limit) || limit < 1 || limit > 100) {
      return c.json({
        error: {
          code: 'VALIDATION_ERROR',
          message: 'limit must be an integer between 1 and 100',
        },
      }, 400)
    }

    if (!services.embedding) {
      return c.json({
        error: {
          code: 'SERVICE_UNAVAILABLE',
          message: 'Embedding service is not available',
        },
      }, 503)
    }

    const result = await services.embedding.matchRequirements(
      jdId,
      entityType as 'bullet' | 'perspective',
      { threshold, limit },
    )

    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  return app
}
```

**Acceptance criteria:**
- `GET /api/alignment/score` with valid params delegates to `services.embedding.alignResume()` and returns `{ data: AlignmentReport }`.
- `GET /api/alignment/match` with valid params delegates to `services.embedding.matchRequirements()` and returns `{ data: RequirementMatchReport }`.
- Missing `jd_id` or `resume_id` on `/score` returns 400 with `VALIDATION_ERROR`.
- Missing or invalid `entity_type` on `/match` returns 400 with `VALIDATION_ERROR`.
- Inverted thresholds (`strong <= adjacent`) return 400 with a message including both values.
- Equal thresholds (`strong == adjacent`) return 400 with a message including both values.
- Out-of-range thresholds return 400.
- NaN thresholds return 400.
- `limit` outside [1, 100] returns 400.
- Missing `services.embedding` returns 503 `SERVICE_UNAVAILABLE` on both `/score` and `/match`.
- EmbeddingService errors (NOT_FOUND, MISSING_EMBEDDINGS) are passed through via `mapStatusCode()`.
- Imports `mapStatusCode` from `./status-codes` (NOT from `./server`).
- Uses `STRONG_THRESHOLD_DEFAULT` and `ADJACENT_THRESHOLD_DEFAULT` constants (no magic numbers).

**Failure criteria:**
- Silently accepting inverted thresholds -- produces misleading classification.
- Silently accepting equal thresholds -- ambiguous classification boundary.
- Not checking for `services.embedding` existence -- crashes with undefined method call.
- Using POST instead of GET -- spec requires GET for idempotent reads.
- Importing `mapStatusCode` from `./server` -- creates circular dependency.

---

### T70.4: Enhance Health Route with Version; Mount Alignment Routes

**File:** `packages/core/src/routes/server.ts`

[IMPORTANT] The existing health route at line 98 returns `{ status: 'ok' }`. The MCP spec expects `{ server: 'ok', version: string }`. Enhance the existing route rather than creating a separate file. The version comes from `package.json`. If reading `package.json` fails, fall back to `'unknown'`.

[IMPORTANT] The `createRequire` / `import` approach for reading package.json must work in Bun's ESM context. Use `Bun.file()` or a static import if available; otherwise use `fs.readFileSync` with `import.meta.dir`.

> **[AP2]** Do NOT read `package.json` with `readFileSync` at module initialization (top-level side effect). Wrap the version read in a function called during server startup. Tests should assert `typeof version === 'string'` rather than a specific version value.

> **[C1 — Health response field]:** The health response uses `version` (not `api_version`). Phase 71 must use `version` not `api_version` when parsing the health response.

> **[IN2 — Route mount path]:** Confirm existing `server.ts` mount pattern for other routes (e.g., `auditRoutes`). Mirror exactly. If server applies `/api` at top level, use `app.route('/api', alignmentRoutes(services))`. If routes are mounted under `/` and the `/api` prefix is applied elsewhere, use `app.route('/', alignmentRoutes(services))`.

Changes:

1. **Extract `mapStatusCode` to `status-codes.ts`** (see T70.2b). Replace the inline definition with an import:

```typescript
import { mapStatusCode } from './status-codes'
```

2. Wrap version reading in a function called at server startup, not module init:

```typescript
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

function readServerVersion(): string {
  try {
    const pkg = JSON.parse(readFileSync(join(import.meta.dir, '../../package.json'), 'utf-8'))
    return pkg.version ?? 'unknown'
  } catch {
    return 'unknown'  // Fall back silently -- health route must never fail
  }
}

// Called during createApp() or server startup, NOT at module scope:
const serverVersion = readServerVersion()
```

3. Update the health route (currently line 98):

```typescript
// Before:
app.get('/health', (c) => c.json({ status: 'ok' }))

// After:
app.get('/health', (c) => c.json({ server: 'ok', version: serverVersion }))
```

4. Import and mount alignment routes:

```typescript
import { alignmentRoutes } from './alignment'
// ...
app.route('/', alignmentRoutes(services))
```

5. **[I5 — Health route consumer audit]:** Search all `*.test.ts`, `*.ts`, `*.svelte` files for `status.*ok` references against the health route. Update any assertions that check for `{ status: 'ok' }` to check for `{ server: 'ok', version: <string> }`. Document findings in the PR description.

**Acceptance criteria:**
- `GET /api/health` returns `{ server: 'ok', version: '0.0.1' }` (or whatever the current version is).
- Health route never throws, even if `package.json` is missing or malformed.
- Version is read via a function call during startup, not a top-level `readFileSync`.
- `mapStatusCode` is imported from `./status-codes` (inline definition removed from this file).
- `mapStatusCode('MISSING_EMBEDDINGS')` returns 422.
- `mapStatusCode('SERVICE_UNAVAILABLE')` returns 503.
- Alignment routes are mounted and accessible under `/api/alignment/...`.
- Existing routes and middleware are unchanged.
- All files referencing the old `{ status: 'ok' }` health shape have been audited and updated.

**Failure criteria:**
- Health route throws on missing `package.json` -- breaks MCP `forge_health`.
- Forgetting to mount alignment routes -- 404 on `/api/alignment/score`.
- Breaking the existing `{ status: 'ok' }` contract without updating consumers that depend on it (check if any existing tests or scripts parse the old shape).
- `readFileSync` at module-level initialization (top-level side effect).

---

### T70.5: Update Services Interface

**File:** `packages/core/src/services/index.ts`

[IMPORTANT] Phase 69 may have already added `embedding` to the `Services` interface. If so, verify it matches. If not, add it. The `embedding` property must be optional (`embedding?: EmbeddingService`) because the server must boot without it if the embedding model is not installed.

> **[C2]** Phase 69 should define `embedding` as optional (`embedding?: EmbeddingService`) in the Services interface. If Phase 69 added it as required, convert to optional in this phase. `EmbeddingService` initialization is async (model loading) so it must be injected post-startup, not in `createServices()`.

```typescript
// Add to Services interface:
embedding?: EmbeddingService

// Add to createServices():
// embedding is NOT created here -- it requires async model loading.
// It is injected after createServices() by the server startup code.
// See Phase 69 for the initialization sequence.
```

**Acceptance criteria:**
- `Services.embedding` is typed as `EmbeddingService | undefined` (optional property).
- If Phase 69 added `embedding` as required, it is changed to optional in this task.
- `createServices()` does NOT construct an `EmbeddingService` (async model loading is handled separately).
- Routes that use `services.embedding` check for `undefined` before calling methods.
- Existing services are unchanged.

---

### T70.5b: Verify Post-createServices Injection Startup Sequence

**File:** Server startup code (e.g., `packages/core/src/index.ts` or `packages/core/src/server.ts` entry point)

> **[I6]** Verify or update server startup code to initialize `EmbeddingService` asynchronously after `createServices()`. The startup sequence should be:

1. `const services = createServices(db)` -- synchronous, no embedding service.
2. `const app = createApp(services)` -- server starts, health route is available.
3. `services.embedding = await EmbeddingService.create(db)` -- async model loading, injected after boot.

If Phase 69's startup sequence does not match, update it. If Phase 69 has not landed, document the expected sequence so Phase 69 can implement it correctly.

**Acceptance criteria:**
- Server boots and responds to `/api/health` before embedding model is loaded.
- `EmbeddingService` is injected into `services` after async initialization.
- Alignment routes return 503 during the window between server start and embedding service readiness.

---

### T70.6: Create SDK AlignmentResource

**File:** `packages/sdk/src/resources/alignment.ts`

[CRITICAL] The SDK resource is the contract that MCP tools, CLI commands, and WebUI hooks call. Method signatures must exactly match the spec's `AlignmentResource` class design. Query parameter serialization must produce the same names the HTTP routes expect.

Follow the `IntegrityResource` / `ReviewResource` pattern (constructor receives `RequestFn`):

```typescript
import type {
  AlignmentReport,
  AlignmentScoreOptions,
  MatchRequirementsOptions,
  RequestFn,
  RequirementMatchReport,
  Result,
} from '../types'

export class AlignmentResource {
  constructor(private request: RequestFn) {}

  /**
   * Score a resume against a job description using embedding similarity.
   *
   * @param jdId - Job description ID
   * @param resumeId - Resume ID
   * @param opts - Optional threshold overrides
   * @returns AlignmentReport with per-requirement verdicts and overall score
   */
  score(
    jdId: string,
    resumeId: string,
    opts?: AlignmentScoreOptions,
  ): Promise<Result<AlignmentReport>> {
    const params = new URLSearchParams({
      jd_id: jdId,
      resume_id: resumeId,
    })
    if (opts?.strong_threshold !== undefined) {
      params.set('strong_threshold', String(opts.strong_threshold))
    }
    if (opts?.adjacent_threshold !== undefined) {
      params.set('adjacent_threshold', String(opts.adjacent_threshold))
    }
    return this.request<AlignmentReport>('GET', `/api/alignment/score?${params.toString()}`)
  }

  /**
   * Match JD requirements against the full bullet or perspective inventory.
   * Independent of any specific resume — used for pre-resume content discovery.
   *
   * @param jdId - Job description ID
   * @param entityType - 'bullet' or 'perspective'
   * @param opts - Optional threshold and limit overrides
   * @returns RequirementMatchReport with top candidates per requirement
   */
  matchRequirements(
    jdId: string,
    entityType: 'bullet' | 'perspective',
    opts?: MatchRequirementsOptions,
  ): Promise<Result<RequirementMatchReport>> {
    const params = new URLSearchParams({
      jd_id: jdId,
      entity_type: entityType,
    })
    if (opts?.threshold !== undefined) {
      params.set('threshold', String(opts.threshold))
    }
    if (opts?.limit !== undefined) {
      params.set('limit', String(opts.limit))
    }
    return this.request<RequirementMatchReport>('GET', `/api/alignment/match?${params.toString()}`)
  }
}
```

**Acceptance criteria:**
- `AlignmentResource.score(jdId, resumeId)` calls `GET /api/alignment/score?jd_id=X&resume_id=Y` and returns `Promise<Result<AlignmentReport>>`.
- `AlignmentResource.score(jdId, resumeId, { strong_threshold: 0.8 })` includes `&strong_threshold=0.8` in the query string.
- `AlignmentResource.matchRequirements(jdId, 'perspective')` calls `GET /api/alignment/match?jd_id=X&entity_type=perspective` and returns `Promise<Result<RequirementMatchReport>>`.
- Optional parameters are only included in the query string when provided (no `undefined` values serialized).
- Class follows the `constructor(private request: RequestFn)` pattern used by all other resources.

**Failure criteria:**
- Using `requestList` instead of `request` -- alignment endpoints return single objects, not paginated lists.
- Misspelling query parameter names -- silent 400 from the server.
- Passing thresholds in the body of a GET request -- non-standard and may be stripped by proxies.

---

### T70.7: Register AlignmentResource on ForgeClient

**File:** `packages/sdk/src/client.ts`

[IMPORTANT] Follow the exact pattern used by other resources. The `alignment` property must be documented with a JSDoc comment.

Changes:

1. Add import:

```typescript
import { AlignmentResource } from './resources/alignment'
```

2. Add property declaration (after existing public properties):

```typescript
/** JD↔Resume alignment scoring and requirement matching. */
public alignment: AlignmentResource
```

3. Add instantiation in constructor (after existing resource instantiation):

```typescript
this.alignment = new AlignmentResource(req)
```

4. Update `packages/sdk/src/index.ts` to export the new resource class and types:

```typescript
// Alignment types
export type {
  AlignmentReport,
  RequirementMatch,
  UnmatchedEntry,
  RequirementMatchReport,
  MatchVerdict,
  AlignmentScoreOptions,
  MatchRequirementsOptions,
} from './types'

// Resource class
export { AlignmentResource } from './resources/alignment'
```

**Acceptance criteria:**
- `new ForgeClient({ baseUrl: '...' }).alignment` is an `AlignmentResource` instance.
- `ForgeClient.alignment` has a JSDoc comment.
- `AlignmentResource` is exported from `@forge/sdk` barrel.
- All alignment types are exported from `@forge/sdk` barrel.
- Existing resources and exports are unchanged.

---

### T70.8: Tests

#### Unit Tests: Type Validation

**File:** `packages/core/src/routes/__tests__/alignment.test.ts`

Test the route handler logic with mocked `EmbeddingService`:

```typescript
// Test fixtures:

const SAMPLE_JD_REQUIREMENTS = [
  'Experience with Kubernetes and container orchestration',
  'Strong background in Python or Go for backend services',
  'Familiarity with CI/CD pipelines and GitOps workflows',
  'Understanding of zero-trust security architectures',
]

const SAMPLE_PERSPECTIVES = [
  {
    entry_id: 'e1',
    perspective_id: 'p1',
    content: 'Designed and deployed Kubernetes clusters across 3 cloud providers, managing 200+ microservices',
  },
  {
    entry_id: 'e2',
    perspective_id: 'p2',
    content: 'Built Python-based ETL pipelines processing 2M records daily with 99.9% uptime',
  },
  {
    entry_id: 'e3',
    perspective_id: 'p3',
    content: 'Implemented ArgoCD-driven GitOps deployment pipeline reducing release cycle from 2 weeks to 2 hours',
  },
  {
    entry_id: 'e4',
    perspective_id: 'p4',
    content: 'Created Grafana dashboards for infrastructure monitoring across 50+ nodes',
  },
]

// Expected AlignmentReport shape:
// Similarities: [0.87, 0.72, 0.81, 0.0 (gap, null best_match)]
// overall_score = mean(0.87 + 0.72 + 0.81 + 0.0) = 2.40 / 4 = 0.60
const EXPECTED_REPORT: AlignmentReport = {
  job_description_id: 'jd-1',
  resume_id: 'r-1',
  overall_score: 0.60,
  requirement_matches: [
    {
      requirement_text: 'Experience with Kubernetes and container orchestration',
      requirement_index: 0,
      best_match: {
        entry_id: 'e1',
        perspective_id: 'p1',
        perspective_content: 'Designed and deployed Kubernetes clusters...',
        similarity: 0.87,
      },
      verdict: 'strong',
    },
    {
      requirement_text: 'Strong background in Python or Go for backend services',
      requirement_index: 1,
      best_match: {
        entry_id: 'e2',
        perspective_id: 'p2',
        perspective_content: 'Built Python-based ETL pipelines...',
        similarity: 0.72,
      },
      verdict: 'adjacent',
    },
    {
      requirement_text: 'Familiarity with CI/CD pipelines and GitOps workflows',
      requirement_index: 2,
      best_match: {
        entry_id: 'e3',
        perspective_id: 'p3',
        perspective_content: 'Implemented ArgoCD-driven GitOps...',
        similarity: 0.81,
      },
      verdict: 'strong',
    },
    {
      requirement_text: 'Understanding of zero-trust security architectures',
      requirement_index: 3,
      best_match: null,
      verdict: 'gap',
    },
  ],
  unmatched_entries: [
    {
      entry_id: 'e4',
      perspective_content: 'Created Grafana dashboards...',
      best_requirement_similarity: 0.31,
    },
  ],
  summary: {
    strong: 2,
    adjacent: 1,
    gaps: 1,
    total_requirements: 4,
    total_entries: 4,
  },
  computed_at: '2026-04-03T12:00:00.000Z',
}
```

**Test cases:**

1. **`/alignment/score` -- happy path:** Mock `services.embedding.alignResume()` to return `EXPECTED_REPORT`. Assert response is `{ data: <report> }` with status 200.
2. **`/alignment/score` -- missing jd_id:** Assert 400 with `VALIDATION_ERROR`.
3. **`/alignment/score` -- missing resume_id:** Assert 400 with `VALIDATION_ERROR`.
4. **`/alignment/score` -- inverted thresholds:** `strong_threshold=0.3&adjacent_threshold=0.7` returns 400 with message including both values.
5. **`/alignment/score` -- equal thresholds:** `strong_threshold=0.5&adjacent_threshold=0.5` returns 400 with `VALIDATION_ERROR`.
6. **`/alignment/score` -- out-of-range threshold:** `strong_threshold=1.5` returns 400.
7. **`/alignment/score` -- NaN threshold:** `strong_threshold=abc` returns 400.
8. **`/alignment/score` -- default thresholds:** Omit threshold params, verify `alignResume()` called with `{ strong_threshold: 0.75, adjacent_threshold: 0.50 }`.
9. **`/alignment/score` -- JD not found:** Mock returns `{ ok: false, error: { code: 'NOT_FOUND' } }`. Assert 404.
10. **`/alignment/score` -- missing embeddings:** Mock returns `{ ok: false, error: { code: 'MISSING_EMBEDDINGS' } }`. Assert 422.
11. **`/alignment/score` -- no embedding service:** `services.embedding` is `undefined`. Assert 503 with `SERVICE_UNAVAILABLE`.
12. **`/alignment/match` -- happy path:** Mock `services.embedding.matchRequirements()`. Assert `{ data: <report> }` with status 200.
13. **`/alignment/match` -- missing jd_id:** Assert 400.
14. **`/alignment/match` -- invalid entity_type:** `entity_type=summary` returns 400.
15. **`/alignment/match` -- missing entity_type:** Assert 400.
16. **`/alignment/match` -- limit out of range:** `limit=0` or `limit=200` returns 400.
17. **`/alignment/match` -- default options:** Omit threshold/limit, verify `matchRequirements()` called with `{ threshold: 0.50, limit: 10 }`.
18. **`/alignment/match` -- no embedding service:** `services.embedding` is `undefined`. Assert 503 with `SERVICE_UNAVAILABLE`.
19. **`/health` -- returns version:** Assert `{ server: 'ok', version: <string> }`. Assert `typeof version === 'string'` (not a specific value).

#### Structural Invariant Tests

**File:** `packages/core/src/routes/__tests__/alignment.test.ts` (add to existing)

20. **All JD requirements appear in matches exactly once:** Given a mock with N requirements, assert `requirement_matches.length === N` and each `requirement_index` from 0..N-1 appears exactly once.
21. **Candidates sorted by descending similarity:** For a mock `RequirementMatchReport`, assert each requirement's `candidates` array has `candidates[i].similarity >= candidates[i+1].similarity`.

#### Integration Tests: End-to-End Route Shape

**File:** `packages/core/src/routes/__tests__/alignment-integration.test.ts`

If EmbeddingService is available in the test environment:

1. Seed a JD with known requirements and a resume with known entries.
2. Embed all entities.
3. Call `GET /api/alignment/score` and validate the response shape matches `AlignmentReport`.
4. Call `GET /api/alignment/match` and validate the response shape matches `RequirementMatchReport`.
5. Verify `computed_at` is a valid ISO-8601 timestamp within the last 5 seconds.
6. Verify `summary.strong + summary.adjacent + summary.gaps === summary.total_requirements`.

#### Contract Tests: SDK Matches API

**File:** `packages/sdk/src/resources/__tests__/alignment.test.ts`

1. Mock `fetch` to return a `{ data: AlignmentReport }` response.
2. Call `sdk.alignment.score('jd-1', 'r-1')` and verify the fetch URL is `GET /api/alignment/score?jd_id=jd-1&resume_id=r-1`.
3. Call `sdk.alignment.score('jd-1', 'r-1', { strong_threshold: 0.8 })` and verify `&strong_threshold=0.8` is in the URL.
4. Call `sdk.alignment.matchRequirements('jd-1', 'perspective', { threshold: 0.6, limit: 5 })` and verify the fetch URL includes all params.
5. Verify the returned data matches the mocked `AlignmentReport` / `RequirementMatchReport`.
6. **Error contract -- MISSING_EMBEDDINGS:** Mock returns `{ error: { code: 'MISSING_EMBEDDINGS', message: '...' } }` with status 422. Assert SDK returns `{ ok: false, error: { code: 'MISSING_EMBEDDINGS' } }`.
7. **Error contract -- NOT_FOUND:** Mock returns `{ error: { code: 'NOT_FOUND', message: '...' } }` with status 404. Assert SDK returns `{ ok: false, error: { code: 'NOT_FOUND' } }`.

#### Type Compatibility Tests

**File:** `packages/sdk/src/__tests__/type-sync.test.ts`

> **[AP1]** Structural type check to prevent drift between `@forge/core` and `@forge/sdk` alignment types.

```typescript
import type { AlignmentReport as CoreReport } from '@forge/core'
import type { AlignmentReport as SdkReport } from '../types'

// Compile-time structural check -- if these assignments fail,
// the types have drifted between core and SDK.
const _coreToSdk: SdkReport = {} as CoreReport
const _sdkToCore: CoreReport = {} as SdkReport
```

Do the same for `RequirementMatchReport`, `RequirementMatch`, `UnmatchedEntry`, and `MatchVerdict`.

#### Rollback Safety Test

**File:** `packages/core/src/routes/__tests__/status-codes.test.ts`

> **[G2]** Assert that pre-existing error codes still return the same HTTP status after adding new cases to `mapStatusCode`.

```typescript
import { mapStatusCode } from '../status-codes'

describe('mapStatusCode', () => {
  // Pre-existing codes (must not change)
  test('NOT_FOUND → 404', () => expect(mapStatusCode('NOT_FOUND')).toBe(404))
  test('VALIDATION_ERROR → 400', () => expect(mapStatusCode('VALIDATION_ERROR')).toBe(400))
  test('CONFLICT → 409', () => expect(mapStatusCode('CONFLICT')).toBe(409))
  // ... all other existing codes

  // New codes added in this phase
  test('MISSING_EMBEDDINGS → 422', () => expect(mapStatusCode('MISSING_EMBEDDINGS')).toBe(422))
  test('SERVICE_UNAVAILABLE → 503', () => expect(mapStatusCode('SERVICE_UNAVAILABLE')).toBe(503))

  // Unknown codes default to 500
  test('unknown code → 500', () => expect(mapStatusCode('SOMETHING_ELSE')).toBe(500))
})
```

**Acceptance criteria:**
- All unit tests pass with mocked EmbeddingService.
- Integration tests validate the full request/response cycle (if embedding service is available; skip gracefully if not).
- Contract tests verify SDK URL construction matches what the routes expect.
- Contract tests cover both happy path and error paths (MISSING_EMBEDDINGS, NOT_FOUND).
- Type compatibility tests verify no drift between core and SDK types.
- Status code rollback tests verify pre-existing mappings are preserved.
- Test fixtures are realistic (not trivial single-entry cases).
- `overall_score` fixture value is 0.60, matching the formula `mean(0.87, 0.72, 0.81, 0.0)`.
- `summary.strong + summary.adjacent + summary.gaps === summary.total_requirements` is checked in at least one test.
- Error cases cover all validation paths (missing params, invalid values, service unavailable, equal thresholds).
- Equal thresholds test (`strong_threshold=0.5&adjacent_threshold=0.5`) returns 400.
- `/alignment/match` with no embedding service returns 503.
- Health route test asserts `typeof version === 'string'` (not a specific version).
- Structural invariant tests check requirement uniqueness and candidate sort order.

---

## Example API Responses

### `GET /api/alignment/score?jd_id=jd-1&resume_id=r-1`

```json
{
  "data": {
    "job_description_id": "jd-1",
    "resume_id": "r-1",
    "overall_score": 0.60,
    "requirement_matches": [
      {
        "requirement_text": "Experience with Kubernetes and container orchestration",
        "requirement_index": 0,
        "best_match": {
          "entry_id": "entry-k8s-1",
          "perspective_id": "persp-k8s-1",
          "perspective_content": "Designed and deployed Kubernetes clusters across 3 cloud providers, managing 200+ microservices with Helm charts and custom operators",
          "similarity": 0.87
        },
        "verdict": "strong"
      },
      {
        "requirement_text": "Understanding of zero-trust security architectures",
        "requirement_index": 3,
        "best_match": null,
        "verdict": "gap"
      }
    ],
    "unmatched_entries": [
      {
        "entry_id": "entry-grafana-1",
        "perspective_content": "Created Grafana dashboards for infrastructure monitoring across 50+ nodes",
        "best_requirement_similarity": 0.31
      }
    ],
    "summary": {
      "strong": 2,
      "adjacent": 1,
      "gaps": 1,
      "total_requirements": 4,
      "total_entries": 4
    },
    "computed_at": "2026-04-03T14:32:01.123Z"
  }
}
```

> **Note:** `overall_score` is `mean(0.87, 0.72, 0.81, 0.0) = 0.60` per the formula defined in T70.1. The two omitted requirement_matches (Python/Go at 0.72, CI/CD at 0.81) are elided for brevity.

### `GET /api/alignment/match?jd_id=jd-1&entity_type=perspective&threshold=0.50&limit=3`

```json
{
  "data": {
    "job_description_id": "jd-1",
    "matches": [
      {
        "requirement_text": "Experience with Kubernetes and container orchestration",
        "candidates": [
          {
            "entity_id": "persp-k8s-1",
            "content": "Designed and deployed Kubernetes clusters across 3 cloud providers",
            "similarity": 0.87
          },
          {
            "entity_id": "persp-docker-2",
            "content": "Containerized 15 legacy applications using Docker multi-stage builds",
            "similarity": 0.68
          }
        ]
      },
      {
        "requirement_text": "Understanding of zero-trust security architectures",
        "candidates": []
      }
    ],
    "computed_at": "2026-04-03T14:32:05.456Z"
  }
}
```

### `GET /api/health`

```json
{
  "server": "ok",
  "version": "0.0.1"
}
```

### Error: Missing Embeddings

`GET /api/alignment/score?jd_id=jd-1&resume_id=r-1` when embeddings are incomplete:

```json
{
  "error": {
    "code": "MISSING_EMBEDDINGS",
    "message": "Cannot compute alignment: 3 of 5 JD requirements and 2 of 8 resume entries have no embeddings. Entities needing embedding: jd_requirement:req-3, jd_requirement:req-4, jd_requirement:req-5, perspective:persp-7, perspective:persp-8. Run the embedding pipeline or call forge_check_drift to identify stale entries."
  }
}
```

---

## Documentation Requirements

After this phase lands, update or create:

1. **API reference** (`docs/api-reference.md` if it exists): Document the two new alignment endpoints with request/response examples. If this file does not exist, skip.
2. **SDK README** (`packages/sdk/README.md` if it exists): Add `client.alignment.score()` and `client.alignment.matchRequirements()` usage examples. If this file does not exist, skip.
3. **Health check consumers:** Search all `*.test.ts`, `*.ts`, `*.svelte` files for `status.*ok` references against the health route. Update assertions to match the new `{ server: 'ok', version }` shape (previously `{ status: 'ok' }`). Document findings in the PR description.

---

## Migration Checklist

Before starting this phase, verify:

- [ ] Phase 69 has landed: `EmbeddingService` class exists with `alignResume()` and `matchRequirements()` methods
- [ ] The `embeddings` table migration (020+) has been applied
- [ ] `@xenova/transformers` is installed in `@forge/core`
- [ ] `EmbeddingService` returns `Result<AlignmentReport>` and `Result<RequirementMatchReport>` (matching the types defined in T70.1)
- [ ] No other in-flight phase is modifying `packages/core/src/routes/server.ts` (merge conflicts on the route mount list)
- [ ] `Services.embedding` is typed as optional in Phase 69 (if not, T70.5 converts it)
- [ ] `RequestFn` is exported from `packages/sdk/src/types.ts` (established in Phase 5)
