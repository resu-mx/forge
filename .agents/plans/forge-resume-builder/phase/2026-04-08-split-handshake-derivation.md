# Split-Handshake Derivation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace server-side Claude CLI subprocess calls with a two-phase prepare/commit protocol where the MCP client performs AI work.

**Architecture:** New `pending_derivations` table provides unified locking. `DerivationService` splits into prepare (lock + return prompt) and commit (validate + write) methods. MCP tools become `forge_prepare_derivation` / `forge_commit_derivation`. `forge_extract_jd_skills` returns context instead of invoking AI.

**Tech Stack:** Bun, Hono, SQLite, TypeScript, @modelcontextprotocol/sdk

**Spec:** `docs/superpowers/specs/2026-04-08-split-handshake-derivation-design.md`

---

### Task 1: Migration — `pending_derivations` table

**Files:**
- Create: `packages/core/src/db/migrations/045_pending_derivations.sql`

- [ ] **Step 1: Write the migration**

```sql
-- 045_pending_derivations.sql
-- Split-handshake derivation: unified locking for prepare/commit protocol.
-- Replaces source-level 'deriving' status and in-memory bullet lock Set.

CREATE TABLE pending_derivations (
  id                TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))),2) || '-' || lower(hex(randomblob(6)))),
  entity_type       TEXT NOT NULL CHECK (entity_type IN ('source', 'bullet')),
  entity_id         TEXT NOT NULL,
  client_id         TEXT NOT NULL,
  prompt            TEXT NOT NULL,
  snapshot          TEXT NOT NULL,
  derivation_params TEXT,
  locked_at         TEXT NOT NULL DEFAULT (datetime('now')),
  expires_at        TEXT NOT NULL,
  created_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_pending_derivations_entity
  ON pending_derivations(entity_type, entity_id);
```

- [ ] **Step 2: Verify migration runs**

Run: `cd packages/core && bun run src/index.ts`

Expected: Server starts without errors, migration 045 applied.

- [ ] **Step 3: Verify table exists**

Run: `cd packages/core && bun -e "import Database from 'bun:sqlite'; const db = new Database('forge.db'); console.log(db.query(\"SELECT sql FROM sqlite_master WHERE name = 'pending_derivations'\").get())"`

Expected: Prints the CREATE TABLE statement.

- [ ] **Step 4: Commit**

```bash
git add packages/core/src/db/migrations/045_pending_derivations.sql
git commit -m "feat(db): add pending_derivations table for split-handshake derivation"
```

---

### Task 2: Core — Pending derivations repository

**Files:**
- Create: `packages/core/src/db/repositories/pending-derivation-repository.ts`
- Test: `packages/core/src/db/repositories/__tests__/pending-derivation-repository.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/core/src/db/repositories/__tests__/pending-derivation-repository.test.ts`:

```ts
import { describe, test, expect, beforeEach } from 'bun:test'
import Database from 'bun:sqlite'
import { runMigrations } from '../../migrate'
import * as PendingDerivationRepo from '../pending-derivation-repository'

describe('PendingDerivationRepository', () => {
  let db: Database

  beforeEach(() => {
    db = new Database(':memory:')
    db.exec('PRAGMA journal_mode = WAL')
    db.exec('PRAGMA foreign_keys = ON')
    runMigrations(db)
  })

  describe('create', () => {
    test('creates a pending derivation and returns it', () => {
      const pd = PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: 'src-1',
        client_id: 'mcp',
        prompt: 'test prompt',
        snapshot: 'test snapshot',
        derivation_params: null,
        expires_at: new Date(Date.now() + 120_000).toISOString(),
      })

      expect(pd.id).toBeTruthy()
      expect(pd.entity_type).toBe('source')
      expect(pd.entity_id).toBe('src-1')
      expect(pd.client_id).toBe('mcp')
      expect(pd.prompt).toBe('test prompt')
      expect(pd.snapshot).toBe('test snapshot')
      expect(pd.derivation_params).toBeNull()
      expect(pd.expires_at).toBeTruthy()
    })

    test('rejects duplicate entity locks', () => {
      PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: 'src-1',
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() + 120_000).toISOString(),
      })

      expect(() =>
        PendingDerivationRepo.create(db, {
          entity_type: 'source',
          entity_id: 'src-1',
          client_id: 'webui',
          prompt: 'p2',
          snapshot: 's2',
          derivation_params: null,
          expires_at: new Date(Date.now() + 120_000).toISOString(),
        }),
      ).toThrow()
    })
  })

  describe('getById', () => {
    test('returns the pending derivation', () => {
      const created = PendingDerivationRepo.create(db, {
        entity_type: 'bullet',
        entity_id: 'b-1',
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: JSON.stringify({ archetype: 'agentic-ai', domain: 'ai_ml', framing: 'accomplishment' }),
        expires_at: new Date(Date.now() + 120_000).toISOString(),
      })

      const fetched = PendingDerivationRepo.getById(db, created.id)
      expect(fetched).not.toBeNull()
      expect(fetched!.entity_type).toBe('bullet')
      expect(fetched!.derivation_params).toContain('agentic-ai')
    })

    test('returns null for missing id', () => {
      expect(PendingDerivationRepo.getById(db, 'nope')).toBeNull()
    })
  })

  describe('findUnexpiredByEntity', () => {
    test('returns lock if not expired', () => {
      PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: 'src-1',
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() + 120_000).toISOString(),
      })

      const found = PendingDerivationRepo.findUnexpiredByEntity(db, 'source', 'src-1')
      expect(found).not.toBeNull()
    })

    test('returns null if expired', () => {
      PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: 'src-1',
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() - 1000).toISOString(),
      })

      const found = PendingDerivationRepo.findUnexpiredByEntity(db, 'source', 'src-1')
      expect(found).toBeNull()
    })
  })

  describe('deleteById', () => {
    test('deletes the row', () => {
      const created = PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: 'src-1',
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() + 120_000).toISOString(),
      })

      PendingDerivationRepo.deleteById(db, created.id)
      expect(PendingDerivationRepo.getById(db, created.id)).toBeNull()
    })
  })

  describe('deleteExpired', () => {
    test('deletes expired rows and returns count', () => {
      PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: 'src-1',
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() - 5000).toISOString(),
      })
      PendingDerivationRepo.create(db, {
        entity_type: 'bullet',
        entity_id: 'b-1',
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() + 120_000).toISOString(),
      })

      const deleted = PendingDerivationRepo.deleteExpired(db)
      expect(deleted).toBe(1)

      // Non-expired still exists
      expect(PendingDerivationRepo.findUnexpiredByEntity(db, 'bullet', 'b-1')).not.toBeNull()
    })
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/db/repositories/__tests__/pending-derivation-repository.test.ts`

Expected: FAIL — module not found.

- [ ] **Step 3: Implement the repository**

Create `packages/core/src/db/repositories/pending-derivation-repository.ts`:

```ts
/**
 * PendingDerivationRepository — CRUD for the pending_derivations lock table.
 *
 * Rows are ephemeral: created by prepare, deleted by commit or stale-lock recovery.
 */

import type { Database } from 'bun:sqlite'

export interface PendingDerivation {
  id: string
  entity_type: 'source' | 'bullet'
  entity_id: string
  client_id: string
  prompt: string
  snapshot: string
  derivation_params: string | null
  locked_at: string
  expires_at: string
  created_at: string
}

export interface CreatePendingDerivationInput {
  entity_type: 'source' | 'bullet'
  entity_id: string
  client_id: string
  prompt: string
  snapshot: string
  derivation_params: string | null
  expires_at: string
}

export function create(db: Database, input: CreatePendingDerivationInput): PendingDerivation {
  const row = db
    .query(
      `INSERT INTO pending_derivations (entity_type, entity_id, client_id, prompt, snapshot, derivation_params, expires_at)
       VALUES (?, ?, ?, ?, ?, ?, ?)
       RETURNING *`,
    )
    .get(
      input.entity_type,
      input.entity_id,
      input.client_id,
      input.prompt,
      input.snapshot,
      input.derivation_params,
      input.expires_at,
    ) as PendingDerivation

  return row
}

export function getById(db: Database, id: string): PendingDerivation | null {
  return db
    .query('SELECT * FROM pending_derivations WHERE id = ?')
    .get(id) as PendingDerivation | null
}

/** Find an unexpired lock for the given entity. Returns null if no lock or if expired. */
export function findUnexpiredByEntity(
  db: Database,
  entityType: 'source' | 'bullet',
  entityId: string,
): PendingDerivation | null {
  return db
    .query(
      `SELECT * FROM pending_derivations
       WHERE entity_type = ? AND entity_id = ? AND expires_at > datetime('now')`,
    )
    .get(entityType, entityId) as PendingDerivation | null
}

export function deleteById(db: Database, id: string): void {
  db.run('DELETE FROM pending_derivations WHERE id = ?', [id])
}

/** Delete all expired rows. Returns count of deleted rows. */
export function deleteExpired(db: Database): number {
  const result = db.run(
    `DELETE FROM pending_derivations WHERE expires_at <= datetime('now')`,
  )
  return result.changes
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/core && bun test src/db/repositories/__tests__/pending-derivation-repository.test.ts`

Expected: All 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/db/repositories/pending-derivation-repository.ts packages/core/src/db/repositories/__tests__/pending-derivation-repository.test.ts
git commit -m "feat(core): add PendingDerivationRepository for split-handshake locking"
```

---

### Task 3: Core — Rewrite `DerivationService` with prepare/commit

**Files:**
- Modify: `packages/core/src/services/derivation-service.ts` (full rewrite)
- Modify: `packages/core/src/services/index.ts` (remove `derivingBullets` Set)
- Test: `packages/core/src/services/__tests__/derivation-service.test.ts` (rewrite)

- [ ] **Step 1: Write the failing tests**

Rewrite `packages/core/src/services/__tests__/derivation-service.test.ts`:

```ts
import { describe, test, expect, beforeEach, mock } from 'bun:test'
import Database from 'bun:sqlite'
import { runMigrations } from '../../db/migrate'
import { DerivationService } from '../derivation-service'
import * as SourceRepo from '../../db/repositories/source-repository'
import { BulletRepository } from '../../db/repositories/bullet-repository'
import * as PendingDerivationRepo from '../../db/repositories/pending-derivation-repository'

describe('DerivationService', () => {
  let db: Database
  let svc: DerivationService

  beforeEach(() => {
    db = new Database(':memory:')
    db.exec('PRAGMA journal_mode = WAL')
    db.exec('PRAGMA foreign_keys = ON')
    runMigrations(db)
    svc = new DerivationService(db)
  })

  // ── prepareBulletDerivation ─────────────────────────────────────────

  describe('prepareBulletDerivation', () => {
    test('returns prompt and derivation_id for valid source', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'Built a CLI tool in Rust' })

      const result = svc.prepareBulletDerivation(source.id, 'mcp')
      expect(result.ok).toBe(true)
      if (!result.ok) return

      expect(result.data.derivation_id).toBeTruthy()
      expect(result.data.prompt).toContain('Built a CLI tool in Rust')
      expect(result.data.snapshot).toBe('Built a CLI tool in Rust')
      expect(result.data.instructions).toContain('bullets')
      expect(result.data.expires_at).toBeTruthy()
    })

    test('returns NOT_FOUND for missing source', () => {
      const result = svc.prepareBulletDerivation('nonexistent', 'mcp')
      expect(result.ok).toBe(false)
      if (result.ok) return
      expect(result.error.code).toBe('NOT_FOUND')
    })

    test('returns VALIDATION_ERROR for archived source', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })
      SourceRepo.update(db, source.id, { status: 'archived' })

      const result = svc.prepareBulletDerivation(source.id, 'mcp')
      expect(result.ok).toBe(false)
      if (result.ok) return
      expect(result.error.code).toBe('VALIDATION_ERROR')
    })

    test('returns CONFLICT if entity already locked', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })

      const first = svc.prepareBulletDerivation(source.id, 'mcp')
      expect(first.ok).toBe(true)

      const second = svc.prepareBulletDerivation(source.id, 'webui')
      expect(second.ok).toBe(false)
      if (second.ok) return
      expect(second.error.code).toBe('CONFLICT')
    })

    test('allows re-lock after expiry', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })

      // Create an already-expired lock directly
      PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: source.id,
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() - 1000).toISOString(),
      })

      // Clean up expired locks first (as the service would do)
      PendingDerivationRepo.deleteExpired(db)

      const result = svc.prepareBulletDerivation(source.id, 'mcp')
      expect(result.ok).toBe(true)
    })
  })

  // ── commitBulletDerivation ──────────────────────────────────────────

  describe('commitBulletDerivation', () => {
    test('creates bullets and deletes pending row', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'Built a CLI' })
      const prepResult = svc.prepareBulletDerivation(source.id, 'mcp')
      if (!prepResult.ok) throw new Error('prepare failed')

      const result = svc.commitBulletDerivation(prepResult.data.derivation_id, {
        bullets: [
          { content: 'Built a CLI tool in Rust', technologies: ['Rust'], metrics: null },
        ],
      })

      expect(result.ok).toBe(true)
      if (!result.ok) return
      expect(result.data).toHaveLength(1)
      expect(result.data[0].content).toBe('Built a CLI tool in Rust')

      // Pending row should be deleted
      const pending = PendingDerivationRepo.getById(db, prepResult.data.derivation_id)
      expect(pending).toBeNull()
    })

    test('returns NOT_FOUND for missing derivation_id', () => {
      const result = svc.commitBulletDerivation('nonexistent', {
        bullets: [{ content: 'x', technologies: [], metrics: null }],
      })
      expect(result.ok).toBe(false)
      if (result.ok) return
      expect(result.error.code).toBe('NOT_FOUND')
    })

    test('returns GONE for expired derivation', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })

      // Create an already-expired pending derivation directly
      const pd = PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: source.id,
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() - 1000).toISOString(),
      })

      const result = svc.commitBulletDerivation(pd.id, {
        bullets: [{ content: 'x', technologies: [], metrics: null }],
      })
      expect(result.ok).toBe(false)
      if (result.ok) return
      expect(result.error.code).toBe('GONE')
    })

    test('returns VALIDATION_ERROR for invalid response shape', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })
      const prepResult = svc.prepareBulletDerivation(source.id, 'mcp')
      if (!prepResult.ok) throw new Error('prepare failed')

      const result = svc.commitBulletDerivation(prepResult.data.derivation_id, {
        bullets: [],
      })
      expect(result.ok).toBe(false)
      if (result.ok) return
      expect(result.error.code).toBe('VALIDATION_ERROR')
    })
  })

  // ── preparePerspectiveDerivation ────────────────────────────────────

  describe('preparePerspectiveDerivation', () => {
    test('returns prompt for approved bullet', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })
      const bullet = BulletRepository.create(db, {
        content: 'Deployed K8s clusters across 3 regions',
        source_content_snapshot: 'desc',
        technologies: ['Kubernetes'],
        metrics: '3 regions',
        status: 'approved',
        source_ids: [{ id: source.id, is_primary: true }],
      })

      const result = svc.preparePerspectiveDerivation(
        bullet.id,
        { archetype: 'infrastructure', domain: 'infrastructure', framing: 'accomplishment' },
        'mcp',
      )
      expect(result.ok).toBe(true)
      if (!result.ok) return
      expect(result.data.prompt).toContain('Deployed K8s clusters')
      expect(result.data.derivation_id).toBeTruthy()
    })

    test('rejects non-approved bullet', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })
      const bullet = BulletRepository.create(db, {
        content: 'test',
        source_content_snapshot: 'desc',
        technologies: [],
        metrics: null,
        status: 'draft',
        source_ids: [{ id: source.id, is_primary: true }],
      })

      const result = svc.preparePerspectiveDerivation(
        bullet.id,
        { archetype: 'infrastructure', domain: 'infrastructure', framing: 'accomplishment' },
        'mcp',
      )
      expect(result.ok).toBe(false)
      if (result.ok) return
      expect(result.error.code).toBe('VALIDATION_ERROR')
    })
  })

  // ── commitPerspectiveDerivation ─────────────────────────────────────

  describe('commitPerspectiveDerivation', () => {
    test('creates perspective and deletes pending row', () => {
      const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })
      const bullet = BulletRepository.create(db, {
        content: 'Deployed K8s clusters',
        source_content_snapshot: 'desc',
        technologies: ['Kubernetes'],
        metrics: null,
        status: 'approved',
        source_ids: [{ id: source.id, is_primary: true }],
      })

      const prepResult = svc.preparePerspectiveDerivation(
        bullet.id,
        { archetype: 'infrastructure', domain: 'infrastructure', framing: 'accomplishment' },
        'mcp',
      )
      if (!prepResult.ok) throw new Error('prepare failed')

      const result = svc.commitPerspectiveDerivation(prepResult.data.derivation_id, {
        content: 'Architected multi-region K8s deployment across 3 AWS regions',
        reasoning: 'Emphasized infrastructure ownership',
      })

      expect(result.ok).toBe(true)
      if (!result.ok) return
      expect(result.data.content).toContain('multi-region')

      // Pending row should be deleted
      const pending = PendingDerivationRepo.getById(db, prepResult.data.derivation_id)
      expect(pending).toBeNull()
    })
  })

  // ── recoverStaleLocks ───────────────────────────────────────────────

  describe('recoverStaleLocks', () => {
    test('deletes expired pending derivations', () => {
      PendingDerivationRepo.create(db, {
        entity_type: 'source',
        entity_id: 'src-1',
        client_id: 'mcp',
        prompt: 'p',
        snapshot: 's',
        derivation_params: null,
        expires_at: new Date(Date.now() - 5000).toISOString(),
      })

      const count = DerivationService.recoverStaleLocks(db)
      expect(count).toBe(1)
    })
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/services/__tests__/derivation-service.test.ts`

Expected: FAIL — methods don't exist yet.

- [ ] **Step 3: Rewrite `DerivationService`**

Replace `packages/core/src/services/derivation-service.ts` with:

```ts
/**
 * DerivationService — orchestrates the split-handshake derivation protocol.
 *
 * Two-phase flow:
 * - prepare: lock entity via pending_derivations table, render prompt, return context
 * - commit: validate client's AI response, write entities, delete lock
 *
 * No server-side AI invocation. The MCP client (or any client) does the AI work.
 */

import type { Database } from 'bun:sqlite'
import type { Result } from '../types'
import type { Bullet } from '../db/repositories/bullet-repository'
import type { Perspective } from '../types'
import * as SourceRepo from '../db/repositories/source-repository'
import { BulletRepository } from '../db/repositories/bullet-repository'
import { PerspectiveRepository } from '../db/repositories/perspective-repository'
import * as PromptLogRepo from '../db/repositories/prompt-log-repository'
import * as ArchetypeRepo from '../db/repositories/archetype-repository'
import * as DomainRepo from '../db/repositories/domain-repository'
import * as PendingDerivationRepo from '../db/repositories/pending-derivation-repository'
import {
  renderSourceToBulletPrompt,
  renderBulletToPerspectivePrompt,
  validateBulletDerivation,
  validatePerspectiveDerivation,
  SOURCE_TO_BULLET_TEMPLATE_VERSION,
  BULLET_TO_PERSPECTIVE_TEMPLATE_VERSION,
} from '../ai'
import type { EmbeddingService } from './embedding-service'

// ── Types ────────────────────────────────────────────────────────────

export interface PrepareResult {
  derivation_id: string
  prompt: string
  snapshot: string
  instructions: string
  expires_at: string
}

export interface BulletCommitInput {
  bullets: Array<{ content: string; technologies: string[]; metrics: string | null }>
}

export interface PerspectiveCommitInput {
  content: string
  reasoning: string
}

// ── Instructions (returned to client) ────────────────────────────────

const BULLET_INSTRUCTIONS = `Execute the prompt above. Respond with a JSON object matching this exact schema:
{
  "bullets": [
    {
      "content": "factual bullet text (string, required, non-empty)",
      "technologies": ["tech1", "tech2"] (string array, required),
      "metrics": "quantitative metric if present, null otherwise" (string or null, required)
    }
  ]
}
Rules:
- bullets array must contain at least one item
- Each bullet.content must be a non-empty string
- Each bullet.technologies must be an array of strings
- Each bullet.metrics must be a string or null
After receiving the response, call forge_commit_derivation with the derivation_id and the bullets array.`

const PERSPECTIVE_INSTRUCTIONS = `Execute the prompt above. Respond with a JSON object matching this exact schema:
{
  "content": "reframed bullet text (string, required, non-empty)",
  "reasoning": "brief explanation of what was emphasized and why (string, required)"
}
After receiving the response, call forge_commit_derivation with the derivation_id, content, and reasoning.`

// ── Service ──────────────────────────────────────────────────────────

export class DerivationService {
  private embeddingService: EmbeddingService | null = null
  private lockTimeoutMs: number

  constructor(private db: Database) {
    this.lockTimeoutMs = Number(process.env.FORGE_DERIVATION_LOCK_TIMEOUT_MS) || 120_000
  }

  setEmbeddingService(svc: EmbeddingService): void {
    this.embeddingService = svc
  }

  // ── Prepare: Bullet Derivation ─────────────────────────────────────

  prepareBulletDerivation(sourceId: string, clientId: string): Result<PrepareResult> {
    // 1. Validate source
    const source = SourceRepo.get(this.db, sourceId)
    if (!source) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Source ${sourceId} not found` } }
    }
    if (source.status === 'archived') {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Cannot derive from archived source. Unarchive it first.' } }
    }

    // 2. Check for existing unexpired lock
    const existing = PendingDerivationRepo.findUnexpiredByEntity(this.db, 'source', sourceId)
    if (existing) {
      return { ok: false, error: { code: 'CONFLICT', message: `Source ${sourceId} is already locked for derivation by client '${existing.client_id}' (expires ${existing.expires_at})` } }
    }

    // 3. Capture snapshot and render prompt
    const snapshot = source.description
    const prompt = renderSourceToBulletPrompt(snapshot)
    const expiresAt = new Date(Date.now() + this.lockTimeoutMs).toISOString()

    // 4. Create pending derivation (lock)
    const pd = PendingDerivationRepo.create(this.db, {
      entity_type: 'source',
      entity_id: sourceId,
      client_id: clientId,
      prompt,
      snapshot,
      derivation_params: null,
      expires_at: expiresAt,
    })

    return {
      ok: true,
      data: {
        derivation_id: pd.id,
        prompt,
        snapshot,
        instructions: BULLET_INSTRUCTIONS,
        expires_at: expiresAt,
      },
    }
  }

  // ── Prepare: Perspective Derivation ────────────────────────────────

  preparePerspectiveDerivation(
    bulletId: string,
    params: { archetype: string; domain: string; framing: string },
    clientId: string,
  ): Result<PrepareResult> {
    // 1. Validate bullet
    const bullet = BulletRepository.get(this.db, bulletId)
    if (!bullet) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Bullet ${bulletId} not found` } }
    }
    if (bullet.status === 'archived') {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Cannot derive from archived bullet. Unarchive it first.' } }
    }
    if (bullet.status !== 'approved') {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Bullet must be approved to derive perspectives (current: ${bullet.status})` } }
    }

    // 2. Validate archetype and domain
    const archetypeExists = ArchetypeRepo.getByName(this.db, params.archetype)
    if (!archetypeExists) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Unknown archetype: '${params.archetype}'` } }
    }
    const domainExists = DomainRepo.getByName(this.db, params.domain)
    if (!domainExists) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Unknown domain: '${params.domain}'` } }
    }

    // 3. Check for existing lock
    const existing = PendingDerivationRepo.findUnexpiredByEntity(this.db, 'bullet', bulletId)
    if (existing) {
      return { ok: false, error: { code: 'CONFLICT', message: `Bullet ${bulletId} is already locked for derivation by client '${existing.client_id}'` } }
    }

    // 4. Capture snapshot and render prompt
    const snapshot = bullet.content
    const prompt = renderBulletToPerspectivePrompt(
      snapshot,
      bullet.technologies,
      bullet.metrics,
      params.archetype,
      params.domain,
      params.framing,
    )
    const expiresAt = new Date(Date.now() + this.lockTimeoutMs).toISOString()

    // 5. Create pending derivation
    const pd = PendingDerivationRepo.create(this.db, {
      entity_type: 'bullet',
      entity_id: bulletId,
      client_id: clientId,
      prompt,
      snapshot,
      derivation_params: JSON.stringify(params),
      expires_at: expiresAt,
    })

    return {
      ok: true,
      data: {
        derivation_id: pd.id,
        prompt,
        snapshot,
        instructions: PERSPECTIVE_INSTRUCTIONS,
        expires_at: expiresAt,
      },
    }
  }

  // ── Commit: Bullet Derivation ──────────────────────────────────────

  commitBulletDerivation(derivationId: string, input: BulletCommitInput): Result<Bullet[]> {
    // 1. Look up pending derivation
    const pd = PendingDerivationRepo.getById(this.db, derivationId)
    if (!pd) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Derivation ${derivationId} not found` } }
    }

    // 2. Check expiry
    if (new Date(pd.expires_at) <= new Date()) {
      PendingDerivationRepo.deleteById(this.db, derivationId)
      return { ok: false, error: { code: 'GONE', message: `Derivation ${derivationId} has expired` } }
    }

    // 3. Validate entity_type
    if (pd.entity_type !== 'source') {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Derivation ${derivationId} is a ${pd.entity_type} derivation, not a source derivation` } }
    }

    // 4. Validate response shape
    const validation = validateBulletDerivation(input)
    if (!validation.ok) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Invalid bullet data: ${validation.error}` } }
    }

    // 5. Atomic write
    const bullets: Bullet[] = []
    const txn = this.db.transaction(() => {
      for (const item of validation.data.bullets) {
        const bullet = BulletRepository.create(this.db, {
          content: item.content,
          source_content_snapshot: pd.snapshot,
          technologies: item.technologies,
          metrics: item.metrics,
          status: 'in_review',
          source_ids: [{ id: pd.entity_id, is_primary: true }],
        })

        PromptLogRepo.create(this.db, {
          entity_type: 'bullet',
          entity_id: bullet.id,
          prompt_template: SOURCE_TO_BULLET_TEMPLATE_VERSION,
          prompt_input: pd.prompt,
          raw_response: JSON.stringify(input),
        })

        bullets.push(bullet)
      }

      // Delete pending row (release lock)
      PendingDerivationRepo.deleteById(this.db, derivationId)
    })

    txn()

    // Fire-and-forget embedding hooks
    if (this.embeddingService) {
      for (const bullet of bullets) {
        queueMicrotask(() =>
          this.embeddingService!.onBulletCreated(bullet).catch(err =>
            console.error(`[DerivationService] Embedding hook failed for bullet ${bullet.id}:`, err)
          )
        )
      }
    }

    return { ok: true, data: bullets }
  }

  // ── Commit: Perspective Derivation ─────────────────────────────────

  commitPerspectiveDerivation(derivationId: string, input: PerspectiveCommitInput): Result<Perspective> {
    // 1. Look up pending derivation
    const pd = PendingDerivationRepo.getById(this.db, derivationId)
    if (!pd) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Derivation ${derivationId} not found` } }
    }

    // 2. Check expiry
    if (new Date(pd.expires_at) <= new Date()) {
      PendingDerivationRepo.deleteById(this.db, derivationId)
      return { ok: false, error: { code: 'GONE', message: `Derivation ${derivationId} has expired` } }
    }

    // 3. Validate entity_type
    if (pd.entity_type !== 'bullet') {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Derivation ${derivationId} is a ${pd.entity_type} derivation, not a bullet derivation` } }
    }

    // 4. Validate response shape
    const validation = validatePerspectiveDerivation(input)
    if (!validation.ok) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Invalid perspective data: ${validation.error}` } }
    }

    // 5. Parse derivation params
    const params = JSON.parse(pd.derivation_params!) as { archetype: string; domain: string; framing: string }

    // 6. Atomic write
    let perspective!: Perspective
    const txn = this.db.transaction(() => {
      perspective = PerspectiveRepository.create(this.db, {
        bullet_id: pd.entity_id,
        content: validation.data.content,
        bullet_content_snapshot: pd.snapshot,
        target_archetype: params.archetype,
        domain: params.domain,
        framing: params.framing,
        status: 'in_review',
      })

      PromptLogRepo.create(this.db, {
        entity_type: 'perspective',
        entity_id: perspective.id,
        prompt_template: BULLET_TO_PERSPECTIVE_TEMPLATE_VERSION,
        prompt_input: pd.prompt,
        raw_response: JSON.stringify(input),
      })

      PendingDerivationRepo.deleteById(this.db, derivationId)
    })

    txn()

    // Fire-and-forget embedding
    if (this.embeddingService) {
      queueMicrotask(() =>
        this.embeddingService!.onPerspectiveCreated(perspective).catch(err =>
          console.error(`[DerivationService] Embedding hook failed for perspective ${perspective.id}:`, err)
        )
      )
    }

    return { ok: true, data: perspective }
  }

  // ── Stale Lock Recovery ────────────────────────────────────────────

  static recoverStaleLocks(db: Database): number {
    return PendingDerivationRepo.deleteExpired(db)
  }
}
```

- [ ] **Step 4: Update `services/index.ts` — remove `derivingBullets` Set**

In `packages/core/src/services/index.ts`, change the `createServices` function:

Replace:
```ts
export function createServices(db: Database, dbPath: string): Services {
  const derivingBullets = new Set<string>()

  return {
    sources: new SourceService(db),
    bullets: new BulletService(db),
    perspectives: new PerspectiveService(db),
    derivation: new DerivationService(db, derivingBullets),
```

With:
```ts
export function createServices(db: Database, dbPath: string): Services {
  return {
    sources: new SourceService(db),
    bullets: new BulletService(db),
    perspectives: new PerspectiveService(db),
    derivation: new DerivationService(db),
```

Also update the comment above `createServices`:
```ts
/**
 * Create all services with shared database connection.
 */
export function createServices(db: Database, dbPath: string): Services {
```

- [ ] **Step 5: Run tests**

Run: `cd packages/core && bun test src/services/__tests__/derivation-service.test.ts`

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add packages/core/src/services/derivation-service.ts packages/core/src/services/index.ts packages/core/src/services/__tests__/derivation-service.test.ts
git commit -m "feat(core): rewrite DerivationService with prepare/commit split-handshake"
```

---

### Task 4: Core — New derivation routes + stub old routes

**Files:**
- Create: `packages/core/src/routes/derivations.ts`
- Modify: `packages/core/src/routes/sources.ts:106-109` (501 stub)
- Modify: `packages/core/src/routes/bullets.ts:77-83` (501 stub)
- Modify: `packages/core/src/routes/server.ts` (mount new routes)
- Test: `packages/core/src/routes/__tests__/derivations.test.ts`

- [ ] **Step 1: Write the failing route tests**

Create `packages/core/src/routes/__tests__/derivations.test.ts`:

```ts
import { describe, test, expect, beforeEach } from 'bun:test'
import Database from 'bun:sqlite'
import { runMigrations } from '../../db/migrate'
import { createServices } from '../../services'
import { createApp } from '../server'
import * as SourceRepo from '../../db/repositories/source-repository'
import { BulletRepository } from '../../db/repositories/bullet-repository'

function createTestApp() {
  const db = new Database(':memory:')
  db.exec('PRAGMA journal_mode = WAL')
  db.exec('PRAGMA foreign_keys = ON')
  runMigrations(db)
  const services = createServices(db, ':memory:')
  const app = createApp(services, db)
  return { db, app, services }
}

describe('POST /api/derivations/prepare', () => {
  test('returns prompt for valid source', async () => {
    const { db, app } = createTestApp()
    const source = SourceRepo.create(db, { title: 'Test', description: 'Built a thing' })

    const res = await app.request('/api/derivations/prepare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ entity_type: 'source', entity_id: source.id, client_id: 'test' }),
    })

    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.derivation_id).toBeTruthy()
    expect(body.data.prompt).toContain('Built a thing')
    expect(body.data.instructions).toBeTruthy()
  })

  test('returns 409 for locked entity', async () => {
    const { db, app } = createTestApp()
    const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })

    await app.request('/api/derivations/prepare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ entity_type: 'source', entity_id: source.id, client_id: 'a' }),
    })

    const res = await app.request('/api/derivations/prepare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ entity_type: 'source', entity_id: source.id, client_id: 'b' }),
    })

    expect(res.status).toBe(409)
  })

  test('returns 400 for bullet without params', async () => {
    const { app } = createTestApp()

    const res = await app.request('/api/derivations/prepare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ entity_type: 'bullet', entity_id: 'x', client_id: 'test' }),
    })

    // Will be 404 (bullet not found) or 400 (missing params) depending on order of checks
    expect(res.status).toBeGreaterThanOrEqual(400)
  })
})

describe('POST /api/derivations/:id/commit', () => {
  test('commits bullets for source derivation', async () => {
    const { db, app } = createTestApp()
    const source = SourceRepo.create(db, { title: 'Test', description: 'Built a CLI' })

    const prepRes = await app.request('/api/derivations/prepare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ entity_type: 'source', entity_id: source.id, client_id: 'test' }),
    })
    const prepBody = await prepRes.json()
    const derivationId = prepBody.data.derivation_id

    const commitRes = await app.request(`/api/derivations/${derivationId}/commit`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        bullets: [{ content: 'Built a CLI tool', technologies: ['Rust'], metrics: null }],
      }),
    })

    expect(commitRes.status).toBe(200)
    const commitBody = await commitRes.json()
    expect(commitBody.data).toHaveLength(1)
  })

  test('returns 404 for unknown derivation', async () => {
    const { app } = createTestApp()
    const res = await app.request('/api/derivations/nope/commit', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ bullets: [{ content: 'x', technologies: [], metrics: null }] }),
    })
    expect(res.status).toBe(404)
  })
})

describe('old derive routes return 501', () => {
  test('POST /api/sources/:id/derive-bullets returns 501', async () => {
    const { db, app } = createTestApp()
    const source = SourceRepo.create(db, { title: 'Test', description: 'desc' })

    const res = await app.request(`/api/sources/${source.id}/derive-bullets`, { method: 'POST' })
    expect(res.status).toBe(501)
  })

  test('POST /api/bullets/:id/derive-perspectives returns 501', async () => {
    const { app } = createTestApp()
    const res = await app.request('/api/bullets/fake-id/derive-perspectives', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ archetype: 'x', domain: 'y', framing: 'accomplishment' }),
    })
    expect(res.status).toBe(501)
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/routes/__tests__/derivations.test.ts`

Expected: FAIL — routes don't exist yet.

- [ ] **Step 3: Create `derivations.ts` routes**

Create `packages/core/src/routes/derivations.ts`:

```ts
/**
 * Derivation routes — split-handshake prepare/commit protocol.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './status-codes'

export function derivationRoutes(services: Services) {
  const app = new Hono()

  // POST /derivations/prepare
  app.post('/derivations/prepare', async (c) => {
    const body = await c.req.json<{
      entity_type: 'source' | 'bullet'
      entity_id: string
      client_id: string
      params?: { archetype: string; domain: string; framing: string }
    }>()

    if (!body.entity_type || !body.entity_id || !body.client_id) {
      return c.json({
        error: { code: 'VALIDATION_ERROR', message: 'entity_type, entity_id, and client_id are required' },
      }, 400)
    }

    if (!['source', 'bullet'].includes(body.entity_type)) {
      return c.json({
        error: { code: 'VALIDATION_ERROR', message: 'entity_type must be "source" or "bullet"' },
      }, 400)
    }

    let result
    if (body.entity_type === 'source') {
      result = services.derivation.prepareBulletDerivation(body.entity_id, body.client_id)
    } else {
      if (!body.params || !body.params.archetype || !body.params.domain || !body.params.framing) {
        return c.json({
          error: { code: 'VALIDATION_ERROR', message: 'params (archetype, domain, framing) required for bullet derivation' },
        }, 400)
      }
      result = services.derivation.preparePerspectiveDerivation(body.entity_id, body.params, body.client_id)
    }

    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // POST /derivations/:id/commit
  app.post('/derivations/:id/commit', async (c) => {
    const { id } = c.req.param()
    const body = await c.req.json()

    // Determine commit type by checking which fields are present
    if ('bullets' in body) {
      const result = services.derivation.commitBulletDerivation(id, body)
      if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
      return c.json({ data: result.data })
    } else if ('content' in body && 'reasoning' in body) {
      const result = services.derivation.commitPerspectiveDerivation(id, body)
      if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
      return c.json({ data: result.data })
    } else {
      return c.json({
        error: { code: 'VALIDATION_ERROR', message: 'Body must contain either { bullets: [...] } or { content, reasoning }' },
      }, 400)
    }
  })

  return app
}
```

- [ ] **Step 4: Stub old source derive route**

In `packages/core/src/routes/sources.ts`, replace the derive-bullets handler:

Replace:
```ts
  app.post('/sources/:id/derive-bullets', async (c) => {
    const result = await services.derivation.deriveBulletsFromSource(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })
```

With:
```ts
  app.post('/sources/:id/derive-bullets', (c) => {
    return c.json({
      error: {
        code: 'NOT_IMPLEMENTED',
        message: 'Server-side derivation disabled. Use POST /api/derivations/prepare with entity_type="source".',
      },
    }, 501)
  })
```

- [ ] **Step 5: Stub old bullet derive route**

In `packages/core/src/routes/bullets.ts`, replace the derive-perspectives handler:

Replace:
```ts
  app.post('/bullets/:id/derive-perspectives', async (c) => {
    const body = await c.req.json<{ archetype: string; domain: string; framing: string }>()
    const result = await services.derivation.derivePerspectivesFromBullet(c.req.param('id'), {
```

Find the full handler block and replace with:
```ts
  app.post('/bullets/:id/derive-perspectives', (c) => {
    return c.json({
      error: {
        code: 'NOT_IMPLEMENTED',
        message: 'Server-side derivation disabled. Use POST /api/derivations/prepare with entity_type="bullet".',
      },
    }, 501)
  })
```

- [ ] **Step 6: Mount derivation routes in server.ts**

In `packages/core/src/routes/server.ts`, add the import:

```ts
import { derivationRoutes } from './derivations'
```

And mount it after the alignment routes (line 129):

```ts
  app.route('/', alignmentRoutes(services))
  app.route('/', derivationRoutes(services))
```

- [ ] **Step 7: Add GONE (410) to status code mapper**

Check `packages/core/src/routes/status-codes.ts` — if `GONE` is not already mapped, add it:

```ts
case 'GONE': return 410
```

- [ ] **Step 8: Run route tests**

Run: `cd packages/core && bun test src/routes/__tests__/derivations.test.ts`

Expected: All tests PASS.

- [ ] **Step 9: Commit**

```bash
git add packages/core/src/routes/derivations.ts packages/core/src/routes/sources.ts packages/core/src/routes/bullets.ts packages/core/src/routes/server.ts packages/core/src/routes/status-codes.ts packages/core/src/routes/__tests__/derivations.test.ts
git commit -m "feat(core): add derivation prepare/commit routes, stub old derive routes as 501"
```

---

### Task 5: Core — Refactor `extract-skills` route to return context

**Files:**
- Modify: `packages/core/src/routes/job-descriptions.ts:238-346`

- [ ] **Step 1: Replace the extract-skills handler**

In `packages/core/src/routes/job-descriptions.ts`, replace the entire `extract-skills` handler (lines 238-346) with:

```ts
  // ── JD Skill Extraction (Context Return) ─────────────────────────────
  // Returns JD text + filtered skill inventory + prompt template for
  // client-side AI extraction. No server-side AI invocation.

  app.post('/job-descriptions/:id/extract-skills', async (c) => {
    const { id } = c.req.param()

    // 1. Fetch the JD
    const jdResult = services.jobDescriptions.get(id)
    if (!jdResult.ok) {
      return c.json({ error: jdResult.error }, mapStatusCode(jdResult.error.code))
    }
    const jd = jdResult.data

    // 2. Validate raw_text exists and is non-empty
    if (!jd.raw_text || jd.raw_text.trim().length === 0) {
      return c.json(
        {
          error: {
            code: 'VALIDATION_ERROR',
            message: 'Job description has no text to extract skills from',
          },
        },
        400,
      )
    }

    // 3. Render prompt template
    const prompt_template = renderJDSkillExtractionPrompt(jd.raw_text)

    // 4. Get existing skills filtered by categories mentioned in the JD text
    const rawText = jd.raw_text.toLowerCase()
    const allSkills = services.skills.list()
    const categoryKeywords: Record<string, string[]> = {
      language: ['python', 'java', 'go', 'rust', 'typescript', 'javascript', 'c++', 'scala', 'ruby', 'kotlin', 'swift'],
      framework: ['fastapi', 'django', 'flask', 'react', 'next.js', 'express', 'spring', 'pytorch', 'tensorflow'],
      tool: ['docker', 'kubernetes', 'terraform', 'helm', 'git', 'jenkins', 'grafana', 'prometheus'],
      platform: ['aws', 'gcp', 'azure', 'vercel', 'heroku', 'cloudflare'],
      methodology: ['agile', 'scrum', 'kanban', 'devops', 'devsecops', 'ci/cd', 'tdd', 'sre'],
      domain: ['machine learning', 'deep learning', 'nlp', 'computer vision', 'distributed systems', 'security'],
      certification: ['cka', 'ckad', 'aws certified', 'security+', 'cissp'],
    }

    // Find which categories have at least one keyword match in JD text
    const matchedCategories = new Set<string>()
    for (const [category, keywords] of Object.entries(categoryKeywords)) {
      if (keywords.some(kw => rawText.includes(kw))) {
        matchedCategories.add(category)
      }
    }
    // Always include 'other' and 'soft_skill' as fallbacks
    matchedCategories.add('other')
    matchedCategories.add('soft_skill')

    let existingSkills: Array<{ id: string; name: string; category: string }> = []
    if (allSkills.ok) {
      existingSkills = allSkills.data
        .filter(s => s.category && matchedCategories.has(s.category))
        .map(s => ({ id: s.id, name: s.name, category: s.category ?? 'other' }))
    }

    // 5. Return context payload
    return c.json({
      data: {
        jd_raw_text: jd.raw_text,
        existing_skills: existingSkills,
        prompt_template,
        instructions: 'Execute the prompt_template to extract skills from the JD text. For each extracted skill, check existing_skills for a match by name before creating new ones. Call forge_tag_jd_skill (or POST /api/job-descriptions/:id/skills) for each accepted skill.',
      },
    })
  })
```

Also remove the now-unused imports from the top of the file: `invokeClaude`, `JD_SKILL_EXTRACTION_TEMPLATE_VERSION`, `validateSkillExtraction`, and `PromptLogRepo` (if not used elsewhere in the file).

- [ ] **Step 2: Run existing tests to make sure nothing else breaks**

Run: `cd packages/core && bun test`

Expected: All passing (the e2e tests that mock invokeClaude may need updating — see Task 8).

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/routes/job-descriptions.ts
git commit -m "feat(core): extract-skills returns context payload instead of invoking AI"
```

---

### Task 6: SDK — Add `DerivationsResource`, update types, remove old methods

**Files:**
- Create: `packages/sdk/src/resources/derivations.ts`
- Modify: `packages/sdk/src/types.ts`
- Modify: `packages/sdk/src/client.ts`
- Modify: `packages/sdk/src/resources/sources.ts:58-60`
- Modify: `packages/sdk/src/resources/bullets.ts:82-90`
- Modify: `packages/sdk/src/resources/job-descriptions.ts:135-141`

- [ ] **Step 1: Add new types to `types.ts`**

Add at the end of `packages/sdk/src/types.ts`:

```ts
// ── Split-Handshake Derivation ─────────────────────────────────────

export interface PrepareResult {
  derivation_id: string
  prompt: string
  snapshot: string
  instructions: string
  expires_at: string
}

export interface BulletCommitInput {
  bullets: Array<{ content: string; technologies: string[]; metrics: string | null }>
}

export interface PerspectiveCommitInput {
  content: string
  reasoning: string
}

export interface JDSkillExtractionContext {
  jd_raw_text: string
  existing_skills: Array<{ id: string; name: string; category: string }>
  prompt_template: string
  instructions: string
}
```

- [ ] **Step 2: Create `DerivationsResource`**

Create `packages/sdk/src/resources/derivations.ts`:

```ts
import type {
  BulletCommitInput,
  PerspectiveCommitInput,
  PrepareResult,
  RequestFn,
  Result,
} from '../types'

export class DerivationsResource {
  constructor(private request: RequestFn) {}

  prepare(input: {
    entity_type: 'source' | 'bullet'
    entity_id: string
    client_id: string
    params?: { archetype: string; domain: string; framing: string }
  }): Promise<Result<PrepareResult>> {
    return this.request<PrepareResult>('POST', '/api/derivations/prepare', input)
  }

  commitBullets(derivationId: string, input: BulletCommitInput): Promise<Result<unknown[]>> {
    return this.request<unknown[]>('POST', `/api/derivations/${derivationId}/commit`, input)
  }

  commitPerspective(derivationId: string, input: PerspectiveCommitInput): Promise<Result<unknown>> {
    return this.request<unknown>('POST', `/api/derivations/${derivationId}/commit`, input)
  }
}
```

- [ ] **Step 3: Wire `DerivationsResource` into `ForgeClient`**

In `packages/sdk/src/client.ts`, add the import:

```ts
import { DerivationsResource } from './resources/derivations'
```

Add the property declaration (after `alignment`):

```ts
  /** Split-handshake derivation: prepare/commit protocol. */
  public derivations: DerivationsResource
```

Add initialization in the constructor (after `this.alignment = ...`):

```ts
    this.derivations = new DerivationsResource(req)
```

Update the `sources` and `bullets` property comments:

```ts
  /** Source CRUD. */
  public sources: SourcesResource
  /** Bullet listing, status transitions. */
  public bullets: BulletsResource
```

- [ ] **Step 4: Remove `deriveBullets` from `SourcesResource`**

In `packages/sdk/src/resources/sources.ts`, remove the `deriveBullets` method:

```ts
  deriveBullets(id: string): Promise<Result<Bullet[]>> {
    return this.request<Bullet[]>('POST', `/api/sources/${id}/derive-bullets`)
  }
```

- [ ] **Step 5: Remove `derivePerspectives` from `BulletsResource`**

In `packages/sdk/src/resources/bullets.ts`, remove the `derivePerspectives` method:

```ts
  derivePerspectives(
    id: string,
    input: DerivePerspectiveInput,
  ): Promise<Result<Perspective>> {
    return this.request<Perspective>(
      'POST',
      `/api/bullets/${id}/derive-perspectives`,
      input,
    )
  }
```

Also remove the `DerivePerspectiveInput` import if it's only used by this method.

- [ ] **Step 6: Update `extractSkills` return type in `JobDescriptionsResource`**

In `packages/sdk/src/resources/job-descriptions.ts`, change:

```ts
  /** Extract skills from JD text using AI. Returns suggested skills for review. */
  extractSkills(jdId: string): Promise<Result<SkillExtractionResult>> {
    return this.request<SkillExtractionResult>(
      'POST',
      `/api/job-descriptions/${jdId}/extract-skills`,
    )
  }
```

To:

```ts
  /** Returns JD text + skill inventory + prompt template for client-side extraction. */
  extractSkills(jdId: string): Promise<Result<JDSkillExtractionContext>> {
    return this.request<JDSkillExtractionContext>(
      'POST',
      `/api/job-descriptions/${jdId}/extract-skills`,
    )
  }
```

Update the import at the top to include `JDSkillExtractionContext` and remove `SkillExtractionResult`.

- [ ] **Step 7: Run SDK tests**

Run: `cd packages/sdk && bun test`

Expected: Existing tests for `deriveBullets` and `derivePerspectives` will fail (removed methods). Update or remove those test cases in `packages/sdk/src/__tests__/resources.test.ts`.

- [ ] **Step 8: Commit**

```bash
git add packages/sdk/src/resources/derivations.ts packages/sdk/src/types.ts packages/sdk/src/client.ts packages/sdk/src/resources/sources.ts packages/sdk/src/resources/bullets.ts packages/sdk/src/resources/job-descriptions.ts packages/sdk/src/__tests__/resources.test.ts
git commit -m "feat(sdk): add DerivationsResource, remove old derive methods, update extractSkills"
```

---

### Task 7: MCP — Replace derive tools with prepare/commit

**Files:**
- Modify: `packages/mcp/src/tools/derive.ts` (full rewrite)
- Modify: `packages/mcp/src/tools/tier2-jd.ts` (update extract_jd_skills)
- Modify: `packages/mcp/src/server.ts` (registration)
- Modify: `packages/mcp/src/utils/feature-flags.ts` (remove jdSkillExtraction guard for extract)

- [ ] **Step 1: Rewrite `derive.ts` with prepare/commit tools**

Replace `packages/mcp/src/tools/derive.ts` with:

```ts
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapResult } from '../utils/error-mapper'

export function registerDerivationTools(server: McpServer, sdk: ForgeClient): void {

  // -- forge_prepare_derivation --

  registerTool(
    server,
    'forge_prepare_derivation',
    'Lock an entity and return a prompt for AI derivation. For source entities, returns a prompt to derive bullets. For bullet entities, returns a prompt to derive a perspective. Execute the returned prompt, then call forge_commit_derivation with the results. The lock expires after 2 minutes.',
    {
      entity_type: z.enum(['source', 'bullet'])
        .describe('Entity type: "source" for bullet derivation, "bullet" for perspective derivation'),
      entity_id: z.string().uuid()
        .describe('UUID of the source or bullet to derive from'),
      params: z.object({
        archetype: z.string().describe('Target archetype slug (e.g., "agentic-ai")'),
        domain: z.string().describe('Target domain slug (e.g., "ai_ml")'),
        framing: z.enum(['accomplishment', 'responsibility', 'context'])
          .describe('Perspective framing style'),
      }).optional()
        .describe('Required for bullet derivation (perspective params). Omit for source derivation.'),
    },
    async (params) => {
      const result = await sdk.derivations.prepare({
        entity_type: params.entity_type,
        entity_id: params.entity_id,
        client_id: 'mcp',
        params: params.params,
      })
      return mapResult(result)
    },
  )

  // -- forge_commit_derivation --

  registerTool(
    server,
    'forge_commit_derivation',
    'Commit the AI-generated results for a prepared derivation. For source derivations, provide bullets array. For bullet derivations, provide content and reasoning. The derivation_id comes from forge_prepare_derivation.',
    {
      derivation_id: z.string()
        .describe('Derivation ID from forge_prepare_derivation'),
      bullets: z.array(z.object({
        content: z.string().describe('Factual bullet text'),
        technologies: z.array(z.string()).describe('Technologies mentioned'),
        metrics: z.string().nullable().describe('Quantitative metrics, or null'),
      })).optional()
        .describe('For source derivations: array of derived bullets'),
      content: z.string().optional()
        .describe('For bullet derivations: reframed perspective text'),
      reasoning: z.string().optional()
        .describe('For bullet derivations: explanation of reframing choices'),
    },
    async (params) => {
      if (params.bullets) {
        const result = await sdk.derivations.commitBullets(params.derivation_id, {
          bullets: params.bullets,
        })
        return mapResult(result)
      } else if (params.content && params.reasoning) {
        const result = await sdk.derivations.commitPerspective(params.derivation_id, {
          content: params.content,
          reasoning: params.reasoning,
        })
        return mapResult(result)
      } else {
        return {
          content: [{
            type: 'text' as const,
            text: 'Invalid commit: provide either { bullets: [...] } for source derivations or { content, reasoning } for perspective derivations.',
          }],
          isError: true,
        }
      }
    },
  )
}
```

- [ ] **Step 2: Update `tier2-jd.ts` — make extract_jd_skills always registered**

In `packages/mcp/src/tools/tier2-jd.ts`, move `forge_extract_jd_skills` registration outside the `if (flags.jdSkillExtraction)` block and update its description:

Replace the `forge_extract_jd_skills` registration:

```ts
    registerTool(
      server,
      'forge_extract_jd_skills',
      'Trigger AI-powered skill extraction from a job description. Returns suggested skills with confidence scores.',
      {
        job_description_id: z.string().describe('Job description ID'),
      },
      async (params) => {
        // TODO: Remove `as any` once Phase 62 SDK types are available (see Phase 62)
        const result = await (sdk.jobDescriptions as any).extractSkills(
          params.job_description_id,
        )
        return respond(result)
      },
    )
```

With (placed before the `if (flags.jdSkillExtraction)` block):

```ts
  // forge_extract_jd_skills — always registered (no AI call)
  registerTool(
    server,
    'forge_extract_jd_skills',
    'Returns JD text, existing skill inventory, and a prompt template for client-side skill extraction. Execute the prompt template, then call forge_tag_jd_skill for each accepted skill. The existing_skills list helps avoid creating duplicate skills.',
    {
      job_description_id: z.string().describe('Job description ID'),
    },
    async (params) => {
      const result = await sdk.jobDescriptions.extractSkills(
        params.job_description_id,
      )
      return respond(result)
    },
  )
```

Keep `forge_tag_jd_skill` and `forge_untag_jd_skill` inside the feature flag guard.

- [ ] **Step 3: Update `server.ts` registration**

In `packages/mcp/src/server.ts`, update the import and call:

Replace:
```ts
import { registerDeriveTools } from './tools/derive'
```
With:
```ts
import { registerDerivationTools } from './tools/derive'
```

Replace:
```ts
  registerDeriveTools(server, sdk)    // 2 tools
```
With:
```ts
  registerDerivationTools(server, sdk)  // 2 tools (prepare + commit)
```

- [ ] **Step 4: Run MCP tests**

Run: `cd packages/mcp && bun test`

Expected: Tests for old derive tools fail. Update or remove those test cases.

- [ ] **Step 5: Commit**

```bash
git add packages/mcp/src/tools/derive.ts packages/mcp/src/tools/tier2-jd.ts packages/mcp/src/server.ts
git commit -m "feat(mcp): replace derive tools with prepare/commit, update extract_jd_skills"
```

---

### Task 8: Web UI — Hide derive buttons and stub functions

**Files:**
- Modify: `packages/webui/src/lib/components/SourcesView.svelte`
- Modify: `packages/webui/src/lib/components/DerivePerspectivesDialog.svelte`
- Modify: `packages/webui/src/lib/components/jd/JDSkillExtraction.svelte`

- [ ] **Step 1: Hide derive button in SourcesView.svelte**

In `packages/webui/src/lib/components/SourcesView.svelte`, find the derive button block (around line 1319-1326) and wrap it in `{#if false}`:

```svelte
            {#if false}
            {#if selectedSource.status === 'draft' || selectedSource.status === 'approved'}
              <button
                class="btn btn-derive"
                onclick={deriveBullets}
                disabled={deriving}
              >
                ...
              </button>
            {/if}
            {/if}
```

Also stub the `deriveBullets` function (around line 555):

Replace the function body with:
```ts
  async function deriveBullets() {
    addToast({ message: 'Derivation temporarily disabled — use MCP tools (forge_prepare_derivation)', type: 'info' })
  }
```

- [ ] **Step 2: Hide derive trigger in DerivePerspectivesDialog.svelte**

In `packages/webui/src/lib/components/DerivePerspectivesDialog.svelte`, stub the submit handler.

Replace the body of the derive function (the one calling `forge.bullets.derivePerspectives`) with:
```ts
    addToast({ message: 'Perspective derivation temporarily disabled — use MCP tools', type: 'info' })
```

(Or simply add `return` at the top of the handler after the canDerive check.)

- [ ] **Step 3: Hide extract button in JDSkillExtraction.svelte**

In `packages/webui/src/lib/components/jd/JDSkillExtraction.svelte`, stub the extract function:

Replace the body (the part calling `forge.jobDescriptions.extractSkills`) with:
```ts
    addToast({ message: 'Skill extraction temporarily disabled — use MCP tools (forge_extract_jd_skills)', type: 'info' })
```

- [ ] **Step 4: Verify the web UI builds**

Run: `cd packages/webui && bun run build`

Expected: Build succeeds with no errors.

- [ ] **Step 5: Commit**

```bash
git add packages/webui/src/lib/components/SourcesView.svelte packages/webui/src/lib/components/DerivePerspectivesDialog.svelte packages/webui/src/lib/components/jd/JDSkillExtraction.svelte
git commit -m "feat(webui): disable derive/extract buttons, stub with toast messages"
```

---

### Task 9: Update E2E tests

**Files:**
- Modify: `packages/core/src/__tests__/e2e/e2e.test.ts`

- [ ] **Step 1: Rewrite derivation E2E tests to use prepare/commit**

The existing e2e tests mock `invokeClaude`. They need to be rewritten to:
1. Call `POST /api/derivations/prepare` to get the prompt
2. Call `POST /api/derivations/:id/commit` with valid bullet/perspective data
3. Remove all `spyOn(ai, 'invokeClaude')` calls

For each test suite that currently mocks `invokeClaude`, replace:
- The `beforeEach` mock setup → remove
- The derive call → two HTTP calls (prepare + commit)

Since the tests are using `app.request()`, the pattern is:

```ts
// Old:
const res = await app.request(`/api/sources/${sourceId}/derive-bullets`, { method: 'POST' })

// New:
const prepRes = await app.request('/api/derivations/prepare', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ entity_type: 'source', entity_id: sourceId, client_id: 'test' }),
})
const prepBody = await prepRes.json()

const commitRes = await app.request(`/api/derivations/${prepBody.data.derivation_id}/commit`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    bullets: [{ content: 'Test bullet', technologies: ['TypeScript'], metrics: null }],
  }),
})
```

- [ ] **Step 2: Run all tests**

Run: `cd packages/core && bun test`

Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/__tests__/e2e/e2e.test.ts
git commit -m "test(core): rewrite e2e tests for prepare/commit derivation flow"
```

---

### Task 10: Cleanup — Remove `claude-cli.ts` if no longer imported

**Files:**
- Possibly remove: `packages/core/src/ai/claude-cli.ts`
- Possibly remove: `packages/core/src/ai/__tests__/claude-cli.test.ts`
- Modify: `packages/core/src/ai/index.ts` (remove re-exports if removed)

- [ ] **Step 1: Check for remaining callers of `invokeClaude`**

Run: `cd packages/core && grep -r "invokeClaude" src/ --include="*.ts" | grep -v "__tests__" | grep -v "node_modules"`

If the only remaining references are in `ai/claude-cli.ts` and `ai/index.ts` (the definition and re-export), it's safe to remove.

- [ ] **Step 2: Remove if unused**

If no production callers remain:
- Delete `packages/core/src/ai/claude-cli.ts`
- Delete `packages/core/src/ai/__tests__/claude-cli.test.ts`
- Remove the `invokeClaude`, `parseClaudeEnvelope`, `stripCodeFences` exports from `packages/core/src/ai/index.ts`
- Remove the `ClaudeOptions`, `ClaudeResult` type exports from `packages/core/src/ai/index.ts`

If there are still callers, leave it and note which callers remain.

- [ ] **Step 3: Run all tests to confirm nothing breaks**

Run: `cd packages/core && bun test`

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add packages/core/src/ai/
git commit -m "chore(core): remove claude-cli module (no longer used by split-handshake derivation)"
```

---

### Task 11: Final verification

- [ ] **Step 1: Run full test suite across all packages**

```bash
cd packages/core && bun test
cd packages/sdk && bun test
cd packages/mcp && bun test
cd packages/webui && bun run build
```

Expected: All tests pass, web UI builds cleanly.

- [ ] **Step 2: Manual smoke test — start the server and test MCP tools**

```bash
cd packages/core && bun run src/index.ts
```

In a separate terminal, test the MCP tools via Claude Code to verify:
- `forge_prepare_derivation` returns a prompt
- `forge_commit_derivation` creates entities
- `forge_extract_jd_skills` returns context payload
- Old `forge_derive_bullets` and `forge_derive_perspective` are gone

- [ ] **Step 3: Commit any remaining fixes**

If any fixes were needed during verification, commit them.
