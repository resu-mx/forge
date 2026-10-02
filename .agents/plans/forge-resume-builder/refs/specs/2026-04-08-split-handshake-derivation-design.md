# Split-Handshake Derivation: Design Spec

> **Date:** 2026-04-08
> **Status:** Approved
> **Scope:** Forge MCP + Core + SDK + Web UI

## Problem

Three MCP tools shell out to Claude CLI via `invokeClaude()` to perform AI work server-side. This causes socket timeouts (the MCP connection drops during the 30-60s AI call), double AI cost (the MCP client is already an LLM), and fragile subprocess management.

**Affected tools:**

| Tool | Call site | What it does |
|------|-----------|--------------|
| `forge_extract_jd_skills` | `routes/job-descriptions.ts:270` | JD text → Claude CLI → extracted skills |
| `forge_derive_bullets` | `services/derivation-service.ts:87` | Source description → Claude CLI → bullets |
| `forge_derive_perspective` | `services/derivation-service.ts:231` | Bullet content → Claude CLI → perspective |

## Solution: Split-Handshake Pattern

Each AI derivation becomes a two-phase protocol:

1. **Prepare** — lock the entity, gather context, render the prompt, return it to the client
2. **Commit** — accept the client's AI response, validate, write to DB, release lock

The MCP client (which is already an LLM) executes the prompt itself and calls commit with the results. No server-side AI invocation.

`forge_extract_jd_skills` is simpler — no locking needed. It becomes a context-returning tool: returns JD text + filtered skill inventory + prompt template. The client extracts skills and calls `forge_tag_jd_skill` per accepted skill.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Lock enforcement | Endpoint-level (not just background recovery) | Prepare rejects if unexpired lock exists; commit rejects if lock expired |
| Lock timeout | Configurable via `FORGE_DERIVATION_LOCK_TIMEOUT_MS`, default 2 minutes | Enough for MCP client to process; short enough to recover from crashes |
| Cancel tool | Not needed | Stale-lock recovery handles crashed clients |
| Locking unification | Single `pending_derivations` table replaces both DB-level source `deriving` status and in-memory `Set<string>` for bullets | One mechanism, one recovery path |
| Pending table lifecycle | Ephemeral — rows deleted on commit or expiry cleanup | `prompt_logs` already handles permanent audit |
| Client ID | Stored in `pending_derivations` | Enables auditing which client initiated a derivation |
| Extract JD skills prompt | Returned to client as instruction text | Supports minimal agents and alternate models, not just the calling LLM |
| Extract JD skills inventory | Filtered to categories present in JD text | Reduces token cost vs returning all 175 skills |
| Web UI | Derive buttons hidden, functions stubbed | Deferred to backlog for future client-side AI pattern |

## Database

### Migration `045_pending_derivations.sql`

```sql
CREATE TABLE pending_derivations (
  id                TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))),2) || '-' || lower(hex(randomblob(6)))),
  entity_type       TEXT NOT NULL CHECK (entity_type IN ('source', 'bullet')),
  entity_id         TEXT NOT NULL,
  client_id         TEXT NOT NULL,
  prompt            TEXT NOT NULL,
  snapshot          TEXT NOT NULL,
  derivation_params TEXT,  -- JSON: {archetype, domain, framing} for perspective derivations; null for bullet derivations
  locked_at         TEXT NOT NULL DEFAULT (datetime('now')),
  expires_at        TEXT NOT NULL,
  created_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_pending_derivations_entity
  ON pending_derivations(entity_type, entity_id);
```

- Unique index on `(entity_type, entity_id)` enforces one active lock per entity
- No FK to sources/bullets — decoupled from entity lifecycle
- Rows deleted on successful commit or by stale-lock recovery

## Core Service: `DerivationService` Refactor

### Removed

- `invokeClaude` import
- `deriveBulletsFromSource()` method
- `derivePerspectivesFromBullet()` method
- `SourceRepo.acquireDerivingLock` / `releaseDerivingLock` calls
- In-memory `derivingBullets: Set<string>` field

### New Methods

#### `prepareBulletDerivation(sourceId, clientId) → Result<PrepareResult>`

1. Validate source exists and is not archived
2. Check `pending_derivations` for unexpired lock on `(source, sourceId)` → 409 CONFLICT if exists
3. Capture snapshot from `source.description`
4. Render prompt via `renderSourceToBulletPrompt(snapshot)`
5. INSERT into `pending_derivations` with `expires_at = now + timeout`
6. Return `{derivation_id, prompt, snapshot, instructions, expires_at}`

Instructions include the expected JSON response schema from the validator.

#### `preparePerspectiveDerivation(bulletId, params, clientId) → Result<PrepareResult>`

1. Validate bullet exists, is approved, not archived
2. Validate archetype and domain exist in DB
3. Check `pending_derivations` for unexpired lock on `(bullet, bulletId)` → 409 CONFLICT
4. Capture snapshot from `bullet.content`
5. Render prompt via `renderBulletToPerspectivePrompt(...)`
6. INSERT into `pending_derivations` with `derivation_params` JSON
7. Return `{derivation_id, prompt, snapshot, instructions, expires_at}`

#### `commitBulletDerivation(derivationId, bullets[]) → Result<Bullet[]>`

1. Look up `pending_derivations` row by id → 404 NOT_FOUND if missing
2. Check `expires_at` → 410 GONE if expired
3. Validate response shape via `validateBulletDerivation()`
4. Transaction: write bullets via BulletRepository, write prompt log, DELETE pending row
5. Fire-and-forget embedding hooks
6. Return created bullets

#### `commitPerspectiveDerivation(derivationId, content, reasoning) → Result<Perspective>`

1. Look up pending row → 404 NOT_FOUND if missing
2. Check expiry → 410 GONE if expired
3. Validate via `validatePerspectiveDerivation()`
4. Transaction: write perspective via PerspectiveRepository, write prompt log, DELETE pending row
5. Fire-and-forget embedding hook
6. Return created perspective

#### `recoverStaleLocks()` — updated

DELETEs expired rows from `pending_derivations` instead of resetting source status.

### Configurable Timeout

Env var: `FORGE_DERIVATION_LOCK_TIMEOUT_MS` (default `120000` / 2 minutes). Read once at service construction.

## HTTP Routes

### New: `packages/core/src/routes/derivations.ts`

Mounted at `/api/derivations` in the server.

#### `POST /api/derivations/prepare`

```
Body: { entity_type: "source"|"bullet", entity_id, client_id, params?: {archetype, domain, framing} }
Response: { data: { derivation_id, prompt, snapshot, instructions, expires_at } }
Errors: 409 CONFLICT (locked), 404 NOT_FOUND, 400 VALIDATION_ERROR
```

Dispatches to `prepareBulletDerivation` or `preparePerspectiveDerivation` based on `entity_type`.

#### `POST /api/derivations/:id/commit`

```
Body (source derivation): { bullets: [{ content, technologies, metrics }] }
Body (bullet derivation): { content, reasoning }
Response (source): { data: Bullet[] }
Response (bullet): { data: Perspective }
Errors: 404 NOT_FOUND, 410 GONE (expired), 400 VALIDATION_ERROR
```

The route looks up the pending row's `entity_type` to determine which commit method to call and which body shape to validate. If the body doesn't match the expected shape for the entity_type, returns 400.

### Modified Routes

| Route | Change |
|-------|--------|
| `POST /api/sources/:id/derive-bullets` | Returns 501 with message: "Server-side derivation disabled. Use POST /api/derivations/prepare with entity_type='source'." |
| `POST /api/bullets/:id/derive-perspectives` | Returns 501 with same pattern |
| `POST /api/job-descriptions/:id/extract-skills` | No longer calls `invokeClaude`. Returns context payload: `{ jd_raw_text, existing_skills[], prompt_template, instructions }`. `existing_skills` filtered to categories present in the JD text. |

## SDK Changes

### New: `packages/sdk/src/resources/derivations.ts`

```ts
class DerivationsResource {
  prepare(input: {
    entity_type: 'source' | 'bullet',
    entity_id: string,
    client_id: string,
    params?: { archetype: string, domain: string, framing: string }
  }): Promise<Result<PrepareResult>>

  commit(derivationId: string, input: BulletCommitInput | PerspectiveCommitInput): Promise<Result<Bullet[] | Perspective>>
}
```

Exposed as `sdk.derivations` on `ForgeClient`.

### New Types

```ts
interface PrepareResult {
  derivation_id: string
  prompt: string
  snapshot: string
  instructions: string
  expires_at: string
}

interface BulletCommitInput {
  bullets: Array<{ content: string; technologies: string[]; metrics: string | null }>
}

interface PerspectiveCommitInput {
  content: string
  reasoning: string
}

interface JDSkillExtractionContext {
  jd_raw_text: string
  existing_skills: Array<{ id: string; name: string; category: string }>
  prompt_template: string
  instructions: string
}
```

### Removed

- `sdk.sources.deriveBullets()` — method deleted
- `sdk.bullets.derivePerspectives()` — method deleted

### Modified

- `sdk.jobDescriptions.extractSkills()` — return type changes from `SkillExtractionResult` to `JDSkillExtractionContext`

## MCP Tool Changes

### Removed

- `forge_derive_bullets`
- `forge_derive_perspective`

### New

#### `forge_prepare_derivation`

- **Params:** `entity_type ("source"|"bullet"), entity_id, params?: {archetype, domain, framing}`
- Calls `sdk.derivations.prepare()` with `client_id: "mcp"`
- Returns `{derivation_id, prompt, snapshot, instructions, expires_at}`
- Tool description tells the MCP client to execute the prompt and then call `forge_commit_derivation`

#### `forge_commit_derivation`

- **Params:** `derivation_id, bullets?: [{content, technologies, metrics}], content?: string, reasoning?: string`
- Calls `sdk.derivations.commit()`
- Returns created entities

### Modified

#### `forge_extract_jd_skills`

- No longer feature-flagged (no AI call = no risk)
- Returns `{jd_raw_text, existing_skills[], prompt_template, instructions}`
- Tool description: "Returns JD text and context for skill extraction. Execute the prompt template against the JD text, then call forge_tag_jd_skill for each accepted skill."

### Unchanged

- `forge_tag_jd_skill` — still used to persist accepted skills
- `forge_untag_jd_skill` — unchanged
- `forge_match_requirements` — unchanged (still requires embeddings)

### Registration

`registerDeriveTools` renamed to `registerDerivationTools`. Two tools (prepare + commit) instead of two (derive_bullets + derive_perspective).

## Web UI Changes

Disable AI derive buttons and stub functions. Three files:

| File | Change |
|------|--------|
| `SourcesView.svelte` | Hide "Derive Bullets" button, stub `deriveBullets()` with toast |
| `DerivePerspectivesDialog.svelte` | Hide derive trigger, stub submit |
| `JDSkillExtraction.svelte` | Hide "Extract Skills" button, stub function |

Pages still render. Manual skill tagging via JDSkillPicker still works. Web UI derive support is deferred to backlog.

## Testing

### Unit Tests

- `prepareBulletDerivation` — happy path, source not found, source archived, lock conflict, lock expired allows re-lock
- `commitBulletDerivation` — happy path, derivation not found, expired (410), invalid response shape
- Same pairs for perspective prepare/commit
- `recoverStaleLocks` — deletes expired rows

### Route Tests

- `POST /api/derivations/prepare` — 200, 409, 400
- `POST /api/derivations/:id/commit` — 200, 404, 410
- Old routes return 501
- `POST /api/job-descriptions/:id/extract-skills` — returns context payload

### SDK Tests

- `sdk.derivations.prepare()` sends correct HTTP
- `sdk.derivations.commit()` sends correct HTTP
- `sdk.sources.deriveBullets` removed (TypeScript confirms)
- `sdk.jobDescriptions.extractSkills` returns new shape

### MCP Tests

- `forge_prepare_derivation` and `forge_commit_derivation` registered
- `forge_derive_bullets` and `forge_derive_perspective` no longer registered
- `forge_extract_jd_skills` returns context

### E2E Tests

Existing tests in `packages/core/src/__tests__/e2e/e2e.test.ts` rewritten for prepare/commit flow. No more `invokeClaude` mocking.

## Files Affected

### New Files
- `packages/core/src/db/migrations/045_pending_derivations.sql`
- `packages/core/src/routes/derivations.ts`
- `packages/sdk/src/resources/derivations.ts`

### Modified Files
- `packages/core/src/services/derivation-service.ts` — full rewrite
- `packages/core/src/routes/sources.ts` — 501 stub
- `packages/core/src/routes/bullets.ts` — 501 stub
- `packages/core/src/routes/job-descriptions.ts` — extract-skills returns context
- `packages/core/src/routes/server.ts` — mount derivations routes
- `packages/sdk/src/client.ts` — add `derivations` resource
- `packages/sdk/src/resources/sources.ts` — remove `deriveBullets`
- `packages/sdk/src/resources/bullets.ts` — remove `derivePerspectives`
- `packages/sdk/src/resources/job-descriptions.ts` — change `extractSkills` return type
- `packages/sdk/src/types.ts` — new types
- `packages/mcp/src/tools/derive.ts` — replace derive tools with prepare/commit
- `packages/mcp/src/tools/tier2-jd.ts` — update extract_jd_skills
- `packages/mcp/src/server.ts` — registration update
- `packages/mcp/src/utils/feature-flags.ts` — remove jdSkillExtraction flag for extract
- `packages/webui/src/lib/components/SourcesView.svelte` — hide derive button
- `packages/webui/src/lib/components/DerivePerspectivesDialog.svelte` — hide dialog
- `packages/webui/src/lib/components/jd/JDSkillExtraction.svelte` — hide extract button

### Test Files
- `packages/core/src/__tests__/e2e/e2e.test.ts` — rewrite for prepare/commit
- `packages/core/src/services/__tests__/derivation-service.test.ts` — rewrite
- `packages/sdk/src/__tests__/resources.test.ts` — update
- New: `packages/core/src/routes/__tests__/derivations.test.ts`

### Potentially Removable
- `packages/core/src/ai/claude-cli.ts` — if no other callers remain
- `packages/core/src/ai/__tests__/claude-cli.test.ts` — same
