# Phase 69: Embedding Service Foundation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Spec:** `refs/specs/2026-04-03-mcp-server-design.md` (Embedding Service Design section)
**Depends on:** Phase 40 (stable schema baseline -- all org model migrations complete)
**Parallel with:** Phases 42-68 (independent of UI/visualization work)
**Duration:** Medium (9 tasks: T69.0 through T69.8, mostly backend)

## Goals

- Add a local vector embedding service to `@forge/core` using `all-MiniLM-L6-v2` via `@huggingface/transformers` (v3+, 384-dimensional vectors, ~50ms/embed, no API calls)
- Create the `embeddings` table (migration N+1 from audit) to persist vectors for bullets, perspectives, JD requirements, and sources
- Expose a `EmbeddingService` with embed, findSimilar, checkStale, refreshStale, alignResume, and matchRequirements operations
- Build a programmatic JD requirement parser that splits raw JD text into individual requirement strings with confidence scores
- Integrate fire-and-forget embedding hooks into entity creation flows (bullet, perspective, JD, source) so vectors are computed asynchronously without blocking writes
- Export all new types and services from the `@forge/core` barrel

## Non-Goals

- LLM-based semantic analysis (this is purely programmatic embedding math)
- JD requirement parsing via AI (that is a future confidence-gated fallback)
- HTTP routes for alignment scoring (those belong to the MCP server phase and the alignment API phase)
- SDK or WebUI integration (downstream phases consume the service)
- Batch embedding CLI commands (future ergonomic addition)
- GPU acceleration or WASM-optimized inference
- Embedding model fine-tuning or custom training
- Real-time embedding updates on entity content edits (checkStale + refreshStale handles drift)

## Context

The MCP server design spec defines a programmatic alignment system where JD requirements and resume content (bullets, perspectives) are compared using cosine similarity of vector embeddings. This phase builds the foundational embedding infrastructure that the `forge_align_resume` and `forge_match_requirements` MCP tools will consume.

The embedding lifecycle is fire-and-forget: after an entity is created and committed to the database, the embedding is computed asynchronously. If embedding fails, the entity remains fully functional -- `checkStale()` detects missing or outdated embeddings, and `refreshStale()` recomputes them.

> **Known gap (G5):** Update hooks (`onBulletUpdated`, `onPerspectiveUpdated`, `onSourceUpdated`) are deferred to a future phase. `checkStale()` + `refreshStale()` serve as the recovery path for content edits that drift from stored embeddings.

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/{N+1}_embeddings.sql` | Embeddings table DDL (N+1 determined by T69.0 audit) |
| `packages/core/src/db/repositories/embedding-repository.ts` | Data access for embeddings table |
| `packages/core/src/services/embedding-service.ts` | Embedding computation, similarity search, staleness, alignment |
| `packages/core/src/lib/jd-parser.ts` | Programmatic JD requirement parser |
| `packages/core/src/lib/model-loader.ts` | Model download/cache utility for @huggingface/transformers |
| `packages/core/src/db/repositories/__tests__/embedding-repository.test.ts` | Repository unit tests |
| `packages/core/src/services/__tests__/embedding-service.test.ts` | Service unit/integration tests |
| `packages/core/src/lib/__tests__/jd-parser.test.ts` | Parser unit tests |
| `packages/core/src/lib/__tests__/model-loader.test.ts` | Model loader tests |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/package.json` | Add `@huggingface/transformers` dependency (v3+) |
| `packages/core/src/services/index.ts` | Add `EmbeddingService` to `Services` interface and `createServices()` |
| `packages/core/src/types/index.ts` | Add `Embedding`, `ParsedRequirements`, `StaleEmbedding`, `EmbeddingEntityType`, `AlignmentReport`, `RequirementMatch`, `UnmatchedEntry`, `RequirementMatchReport` types |
| `packages/core/src/services/derivation-service.ts` | Add fire-and-forget `onBulletCreated` hook after bullet creation in transaction |
| `packages/core/src/services/job-description-service.ts` | Add fire-and-forget `onJDCreated` hook after JD creation; add `onJDUpdated` hook when `raw_text` is updated |
| `packages/core/src/services/source-service.ts` | Add fire-and-forget `onSourceCreated` hook after source creation |

## Fallback Strategies

- **`@huggingface/transformers` not compatible with Bun:** The library is designed for Node.js and browser runtimes. If Bun's ONNX runtime support causes failures, document TF-IDF (term frequency-inverse document frequency) as a future fallback option but do not implement it in this phase. The `EmbeddingService` interface stays the same -- only the `embed()` internals would change if TF-IDF were implemented later. *(Note: TF-IDF is a documented future option only -- it is not an actionable fallback in this phase.)*
- **Model download fails (network error, disk full):** The `ModelLoader` catches download errors and returns a descriptive `Result<void>` error. All embedding methods short-circuit with a logged warning. Entity creation continues unblocked. `checkStale()` reports all entities as missing embeddings.
- **Embedding computation too slow (>200ms per embed):** Add a `pendingEmbeddings` queue backed by `queueMicrotask` that batches embeddings (up to 16 texts per batch). The `@huggingface/transformers` pipeline accepts arrays, so batching is native.
- **Model too large for memory (~80MB):** The model is loaded lazily on first `embed()` call, not at server startup. If memory pressure is detected (via `process.memoryUsage()`), skip embedding and log a warning. Recovery via `refreshStale()`.
- **SQLite BLOB storage too slow for similarity search:** For v1, full-table scan with in-memory cosine similarity is acceptable (expected <1000 embeddings). If this becomes a bottleneck, migrate to `sqlite-vss` extension or an external vector store.

---

## Tasks

### Task Dependency DAG

```
T69.0 gates all tasks
T69.1 ∥ T69.2 ∥ T69.5 (no deps after T69.0)
T69.3 after T69.2
T69.4 after T69.1 + T69.3
T69.6 after T69.4 + T69.5
T69.7 after T69.4
T69.8 after T69.4
```

---

### T69.0: Audit Migration Sequence (MANDATORY PRE-TASK)

**This task gates all subsequent tasks. No other task may proceed until T69.0 is complete.**

Run the following audit:

```bash
ls packages/core/src/db/migrations/ | sort
```

List the highest existing migration number. Assign N+1 as the migration number for the embeddings table.

> **Context:** Migrations from Phases 40, 43, 47, 49, 50, and 60 may have already claimed migration numbers in this range. The hardcoded "020" used in earlier drafts is not reliable. The actual migration number MUST be determined by this audit.

**Acceptance criteria:**
- The highest existing migration number is identified and documented.
- N+1 is assigned and used consistently in all subsequent tasks (T69.2 and all references to the migration file).
- No migration number conflict exists after assignment.

**Failure criteria:**
- A migration number is used that conflicts with an existing migration file.
- T69.2 proceeds without completing this audit.

---

### T69.1: Add `@huggingface/transformers` Dependency and Model Loader

**File to modify:** `packages/core/package.json`
**File to create:** `packages/core/src/lib/model-loader.ts`
**File to create:** `packages/core/src/lib/__tests__/model-loader.test.ts`

Add the `@huggingface/transformers` package (v3+) and create a model loader utility that handles first-use download, caching, and lazy initialization.

#### 69.1.1: Add dependency

```bash
cd packages/core && bun add @huggingface/transformers
```

Verify `package.json` includes:

```json
{
  "dependencies": {
    "@huggingface/transformers": "^3.0.0",
    "hono": "^4.12.9"
  }
}
```

#### 69.1.2: Create model loader

```typescript
// packages/core/src/lib/model-loader.ts

/**
 * ModelLoader — lazy singleton for the all-MiniLM-L6-v2 sentence-transformer.
 *
 * Downloads and caches the model on first use (~80MB). Subsequent calls
 * return the cached pipeline. Thread-safe via promise deduplication.
 *
 * NOTE: resetPipeline() is for testing only. It clears the cached pipeline
 * so tests can start fresh. Do not call in production code.
 */

import { pipeline, type Pipeline } from '@huggingface/transformers'

const MODEL_NAME = 'Xenova/all-MiniLM-L6-v2'
const EMBEDDING_DIM = 384

let pipelinePromise: Promise<Pipeline> | null = null

/**
 * Get (or lazily create) the feature-extraction pipeline.
 *
 * First call triggers model download + ONNX runtime init (~2-5s).
 * Subsequent calls return the cached pipeline instantly.
 */
export async function getEmbeddingPipeline(): Promise<Pipeline> {
  if (!pipelinePromise) {
    pipelinePromise = pipeline('feature-extraction', MODEL_NAME, {
      // Use quantized model for smaller download and faster inference
      quantized: true,
    })
  }
  return pipelinePromise
}

/**
 * Compute a 384-dimensional embedding vector for the given text.
 *
 * Returns a Float32Array of length 384.
 * Throws if the model fails to load or inference errors.
 */
export async function computeEmbedding(text: string): Promise<Float32Array> {
  const extractor = await getEmbeddingPipeline()
  const output = await extractor(text, { pooling: 'mean', normalize: true })
  // output.data is a Float32Array of shape [1, 384] — flatten to [384]
  return new Float32Array(output.data)
}

/**
 * Reset the cached pipeline. Used in tests or when the model needs reloading.
 * NOTE: This is for testing only. See module-level docstring.
 */
export function resetPipeline(): void {
  pipelinePromise = null
}

export { EMBEDDING_DIM, MODEL_NAME }
```

#### 69.1.3: Test model loader

```typescript
// packages/core/src/lib/__tests__/model-loader.test.ts

import { describe, it, expect, afterAll } from 'bun:test'
import { computeEmbedding, resetPipeline, EMBEDDING_DIM } from '../model-loader'

// Guard for CI environments where model download may be skipped
const SKIP_MODEL_TESTS = process.env.CI && process.env.SKIP_MODEL_TESTS

afterAll(() => {
  resetPipeline()
})

describe('model-loader', () => {
  if (SKIP_MODEL_TESTS) {
    it.skip('skipped: SKIP_MODEL_TESTS is set in CI', () => {})
    return
  }

  // Model download may take up to 120s on first run.
  // CI environments should pre-cache the model or set SKIP_MODEL_TESTS=1.
  // Cache location: ~/.cache/huggingface/hub (or HF_HOME if set).
  // To pre-download for CI: `npx @huggingface/transformers download Xenova/all-MiniLM-L6-v2`

  it('produces a 384-dimensional Float32Array', async () => {
    const vec = await computeEmbedding('test string for embedding')
    expect(vec).toBeInstanceOf(Float32Array)
    expect(vec.length).toBe(EMBEDDING_DIM)
  }, 120_000)

  it('produces normalized vectors (L2 norm ~1.0)', async () => {
    const vec = await computeEmbedding('another test string')
    let sumSq = 0
    for (let i = 0; i < vec.length; i++) sumSq += vec[i] * vec[i]
    const norm = Math.sqrt(sumSq)
    expect(norm).toBeCloseTo(1.0, 2)
  }, 120_000)

  it('produces different vectors for different inputs', async () => {
    const v1 = await computeEmbedding('Designed and deployed Kubernetes clusters')
    const v2 = await computeEmbedding('Baked chocolate chip cookies')
    // Cosine similarity should be low for unrelated texts
    let dot = 0
    for (let i = 0; i < v1.length; i++) dot += v1[i] * v2[i]
    expect(dot).toBeLessThan(0.5)
  }, 120_000)

  it('produces similar vectors for semantically related inputs', async () => {
    const v1 = await computeEmbedding('Managed AWS infrastructure using Terraform')
    const v2 = await computeEmbedding('Provisioned cloud resources with infrastructure-as-code tools')
    let dot = 0
    for (let i = 0; i < v1.length; i++) dot += v1[i] * v2[i]
    expect(dot).toBeGreaterThan(0.5)
  }, 120_000)
})
```

**Acceptance criteria:**
- `bun add @huggingface/transformers` completes without error in `packages/core`.
- `computeEmbedding('test')` returns a `Float32Array` of length 384.
- The returned vector is L2-normalized (norm within 0.01 of 1.0).
- Semantically related texts produce cosine similarity > 0.5.
- Semantically unrelated texts produce cosine similarity < 0.5.
- The pipeline is lazily initialized (no work at import time).
- Verify that `@huggingface/transformers` (v3+) is installed, NOT `@xenova/transformers` (superseded).

**Failure criteria:**
- `@huggingface/transformers` fails to install under Bun (triggers fallback strategy).
- Vector dimension is not 384.
- Model download hangs or exceeds 60s on reasonable network.
- `computeEmbedding` throws on valid UTF-8 input.

---

### T69.2: Create Migration (Embeddings Table)

**File to create:** `packages/core/src/db/migrations/{N+1}_embeddings.sql`

> **Note:** The migration number MUST be determined by the T69.0 audit. Migrations from Phases 40, 43, 47, 49, 50, and 60 may have claimed numbers in this range. Audit the full migration sequence and use N+1 from the highest existing migration.

#### 69.2.1: SQL DDL

```sql
-- Embeddings Table
-- Migration: {N+1}_embeddings
-- Stores vector embeddings for semantic similarity search.
-- entity_type discriminates between bullets, perspectives, JD requirements, and sources.
-- content_hash enables staleness detection (SHA256 of embedded text).
-- vector stores Float32Array as BLOB (384 * 4 = 1536 bytes per row).

CREATE TABLE embeddings (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL CHECK (entity_type IN ('bullet', 'perspective', 'jd_requirement', 'source')),
  entity_id TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  vector BLOB NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(entity_type, entity_id)
) STRICT;

CREATE INDEX idx_embeddings_type ON embeddings(entity_type);
CREATE INDEX idx_embeddings_entity ON embeddings(entity_type, entity_id);

INSERT INTO _migrations (name) VALUES ('{N+1}_embeddings');
```

**Design notes:**
- `entity_type` uses a CHECK constraint to enforce valid discriminator values, matching the pattern used in `prompt_logs`.
- `UNIQUE(entity_type, entity_id)` ensures each entity has at most one embedding. Re-embedding upserts via `INSERT OR REPLACE`.
- `vector` is stored as a BLOB containing raw Float32Array bytes. The repository handles serialization/deserialization.
- `content_hash` is the SHA256 hex digest of the text that was embedded. When the source text changes, the hash no longer matches, and `checkStale()` detects it.
- No foreign key to the entity tables. Embeddings are a derived cache -- if the source entity is deleted, stale embeddings are cleaned up by `refreshStale()` or manually.
- `STRICT` mode enforces column type affinity (TEXT columns reject non-text values).

**Acceptance criteria:**
- Migration applies cleanly on a fresh database (after all prior migrations).
- `SELECT * FROM embeddings` returns zero rows.
- `INSERT INTO embeddings (id, entity_type, entity_id, content_hash, vector, created_at) VALUES ('test', 'bullet', 'b1', 'abc', x'00', datetime('now'))` succeeds.
- `INSERT INTO embeddings (id, entity_type, entity_id, content_hash, vector, created_at) VALUES ('test2', 'invalid_type', 'b2', 'abc', x'00', datetime('now'))` fails with CHECK constraint.
- Duplicate `(entity_type, entity_id)` insert fails with UNIQUE constraint.
- `_migrations` table contains `'{N+1}_embeddings'` after migration.

**Failure criteria:**
- Migration number conflicts with an existing migration file.
- CHECK constraint does not include all four entity types.
- STRICT mode causes issues with BLOB insertion patterns.

**Migration regression test:** Add a test that runs all migrations through N+1 on an in-memory DB and verifies the `embeddings` table schema (columns, types, constraints) matches expectations.

---

### T69.3: Create EmbeddingRepository

**File to create:** `packages/core/src/db/repositories/embedding-repository.ts`
**File to create:** `packages/core/src/db/repositories/__tests__/embedding-repository.test.ts`

The repository follows the static-methods-on-object pattern established by `BulletRepository` and other existing repositories. All methods accept `db: Database` as the first argument.

> **I2 note:** `EmbeddingEntityType` is defined once in `types/index.ts` and imported here. Do NOT define a duplicate in this file.

#### 69.3.1: Repository implementation

```typescript
// packages/core/src/db/repositories/embedding-repository.ts

/**
 * EmbeddingRepository — pure data access for the embeddings table.
 *
 * Handles CRUD for vector embeddings, including BLOB serialization
 * of Float32Array vectors and staleness queries by content hash.
 * Does NOT enforce business rules (embedding lifecycle logic lives
 * in EmbeddingService).
 */

import type { Database } from 'bun:sqlite'
import type { EmbeddingEntityType } from '../../types'

// ── Types ────────────────────────────────────────────────────────────

export interface EmbeddingRow {
  id: string
  entity_type: EmbeddingEntityType
  entity_id: string
  content_hash: string
  vector: Uint8Array  // BLOB comes back as Uint8Array from bun:sqlite
  created_at: string
}

export interface UpsertEmbeddingInput {
  entity_type: EmbeddingEntityType
  entity_id: string
  content_hash: string
  vector: Float32Array
}

// ── Helpers ──────────────────────────────────────────────────────────

/** Serialize a Float32Array to a Buffer for BLOB storage. */
function serializeVector(vec: Float32Array): Buffer {
  return Buffer.from(vec.buffer, vec.byteOffset, vec.byteLength)
}

/** Deserialize a BLOB (Uint8Array) back to Float32Array. */
function deserializeVector(blob: Uint8Array): Float32Array {
  // Use .buffer.slice() to guarantee byte alignment for Float32Array.
  // Uint8Array from SQLite may have non-zero byteOffset, causing
  // alignment issues with Float32Array's 4-byte element size.
  const alignedBuffer = blob.buffer.slice(blob.byteOffset, blob.byteOffset + blob.byteLength)
  return new Float32Array(alignedBuffer)
}

// ── Repository ───────────────────────────────────────────────────────

export const EmbeddingRepository = {
  /**
   * Upsert an embedding for a (entity_type, entity_id) pair.
   * If an embedding already exists for this entity, it is replaced.
   */
  upsert(db: Database, input: UpsertEmbeddingInput): EmbeddingRow {
    const id = crypto.randomUUID()
    const vectorBlob = serializeVector(input.vector)

    const row = db
      .query(
        `INSERT INTO embeddings (id, entity_type, entity_id, content_hash, vector)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (entity_type, entity_id)
         DO UPDATE SET
           content_hash = excluded.content_hash,
           vector = excluded.vector,
           created_at = datetime('now')
         RETURNING *`,
      )
      .get(id, input.entity_type, input.entity_id, input.content_hash, vectorBlob) as EmbeddingRow

    return row
  },

  /**
   * Get an embedding by entity type and ID.
   * Returns null if no embedding exists for this entity.
   */
  findByEntity(db: Database, entityType: EmbeddingEntityType, entityId: string): EmbeddingRow | null {
    return db
      .query('SELECT * FROM embeddings WHERE entity_type = ? AND entity_id = ?')
      .get(entityType, entityId) as EmbeddingRow | null
  },

  /**
   * Get all embeddings for a given entity type.
   * Returns rows with raw BLOB vectors (caller deserializes).
   */
  findByType(db: Database, entityType: EmbeddingEntityType): EmbeddingRow[] {
    return db
      .query('SELECT * FROM embeddings WHERE entity_type = ? ORDER BY created_at DESC')
      .all(entityType) as EmbeddingRow[]
  },

  /**
   * Delete an embedding by entity type and ID.
   * Returns true if a row was deleted, false if not found.
   */
  deleteByEntity(db: Database, entityType: EmbeddingEntityType, entityId: string): boolean {
    const result = db.run(
      'DELETE FROM embeddings WHERE entity_type = ? AND entity_id = ?',
      [entityType, entityId],
    )
    return result.changes > 0
  },

  /**
   * Find stale embeddings: entities whose content_hash no longer matches.
   *
   * Compares the stored content_hash against the provided map of
   * entityId -> currentHash. Returns entity IDs where hashes differ
   * or where no embedding exists at all.
   *
   * @param entityType - The entity type to check.
   * @param currentHashes - Map of entity_id -> current content SHA256 hash.
   * @returns Array of {entity_id, stored_hash, current_hash} for stale entries.
   */
  findStale(
    db: Database,
    entityType: EmbeddingEntityType,
    currentHashes: Map<string, string>,
  ): Array<{ entity_id: string; stored_hash: string | null; current_hash: string }> {
    const stale: Array<{ entity_id: string; stored_hash: string | null; current_hash: string }> = []

    for (const [entityId, currentHash] of currentHashes) {
      const row = db
        .query('SELECT content_hash FROM embeddings WHERE entity_type = ? AND entity_id = ?')
        .get(entityType, entityId) as { content_hash: string } | null

      if (!row) {
        // No embedding exists at all
        stale.push({ entity_id: entityId, stored_hash: null, current_hash: currentHash })
      } else if (row.content_hash !== currentHash) {
        // Embedding exists but is stale
        stale.push({ entity_id: entityId, stored_hash: row.content_hash, current_hash: currentHash })
      }
    }

    return stale
  },

  /**
   * Delete all embeddings (used in tests).
   */
  deleteAll(db: Database): number {
    const result = db.run('DELETE FROM embeddings')
    return result.changes
  },

  /** Deserialize a BLOB vector from a row into a Float32Array. */
  deserializeVector,

  /** Serialize a Float32Array for BLOB storage. */
  serializeVector,
}
```

#### 69.3.2: Repository tests

```typescript
// packages/core/src/db/repositories/__tests__/embedding-repository.test.ts

import { describe, it, expect, beforeEach } from 'bun:test'
import { getDatabase } from '../../connection'
import { runMigrations } from '../../migrate'
import { EmbeddingRepository } from '../embedding-repository'
import { join } from 'node:path'

function setupDb() {
  const db = getDatabase(':memory:')
  runMigrations(db, join(import.meta.dir, '../../migrations'))
  return db
}

function makeVector(dim = 384, fill = 0.1): Float32Array {
  const vec = new Float32Array(dim)
  vec.fill(fill)
  return vec
}

describe('EmbeddingRepository', () => {
  let db: ReturnType<typeof getDatabase>

  beforeEach(() => {
    db = setupDb()
  })

  it('upserts and retrieves an embedding', () => {
    const vec = makeVector()
    const row = EmbeddingRepository.upsert(db, {
      entity_type: 'bullet',
      entity_id: 'b-001',
      content_hash: 'abc123',
      vector: vec,
    })

    expect(row.entity_type).toBe('bullet')
    expect(row.entity_id).toBe('b-001')
    expect(row.content_hash).toBe('abc123')

    const found = EmbeddingRepository.findByEntity(db, 'bullet', 'b-001')
    expect(found).not.toBeNull()
    expect(found!.content_hash).toBe('abc123')

    const deserialized = EmbeddingRepository.deserializeVector(found!.vector)
    expect(deserialized.length).toBe(384)
    expect(deserialized[0]).toBeCloseTo(0.1, 5)
  })

  it('round-trips vector through actual SQLite BLOB without alignment issues', () => {
    // This test specifically validates that deserializeVector handles
    // Uint8Array byte offsets correctly (IN4 fix).
    const original = new Float32Array(384)
    for (let i = 0; i < 384; i++) original[i] = Math.random() * 2 - 1

    EmbeddingRepository.upsert(db, {
      entity_type: 'bullet',
      entity_id: 'b-roundtrip',
      content_hash: 'roundtrip-test',
      vector: original,
    })

    const row = EmbeddingRepository.findByEntity(db, 'bullet', 'b-roundtrip')
    expect(row).not.toBeNull()

    const restored = EmbeddingRepository.deserializeVector(row!.vector)
    expect(restored.length).toBe(384)
    for (let i = 0; i < 384; i++) {
      expect(restored[i]).toBeCloseTo(original[i], 5)
    }
  })

  it('upsert replaces existing embedding for same entity', () => {
    const vec1 = makeVector(384, 0.1)
    EmbeddingRepository.upsert(db, {
      entity_type: 'bullet',
      entity_id: 'b-001',
      content_hash: 'hash1',
      vector: vec1,
    })

    const vec2 = makeVector(384, 0.9)
    EmbeddingRepository.upsert(db, {
      entity_type: 'bullet',
      entity_id: 'b-001',
      content_hash: 'hash2',
      vector: vec2,
    })

    const rows = EmbeddingRepository.findByType(db, 'bullet')
    expect(rows.length).toBe(1)
    expect(rows[0].content_hash).toBe('hash2')
  })

  it('findByType returns all embeddings for a type', () => {
    EmbeddingRepository.upsert(db, {
      entity_type: 'bullet',
      entity_id: 'b-001',
      content_hash: 'h1',
      vector: makeVector(),
    })
    EmbeddingRepository.upsert(db, {
      entity_type: 'bullet',
      entity_id: 'b-002',
      content_hash: 'h2',
      vector: makeVector(),
    })
    EmbeddingRepository.upsert(db, {
      entity_type: 'perspective',
      entity_id: 'p-001',
      content_hash: 'h3',
      vector: makeVector(),
    })

    const bullets = EmbeddingRepository.findByType(db, 'bullet')
    expect(bullets.length).toBe(2)

    const perspectives = EmbeddingRepository.findByType(db, 'perspective')
    expect(perspectives.length).toBe(1)
  })

  it('deleteByEntity removes the embedding', () => {
    EmbeddingRepository.upsert(db, {
      entity_type: 'bullet',
      entity_id: 'b-001',
      content_hash: 'h1',
      vector: makeVector(),
    })

    const deleted = EmbeddingRepository.deleteByEntity(db, 'bullet', 'b-001')
    expect(deleted).toBe(true)

    const found = EmbeddingRepository.findByEntity(db, 'bullet', 'b-001')
    expect(found).toBeNull()
  })

  it('deleteByEntity returns false for nonexistent embedding', () => {
    const deleted = EmbeddingRepository.deleteByEntity(db, 'bullet', 'nonexistent')
    expect(deleted).toBe(false)
  })

  it('findStale detects missing and changed embeddings', () => {
    EmbeddingRepository.upsert(db, {
      entity_type: 'bullet',
      entity_id: 'b-001',
      content_hash: 'old_hash',
      vector: makeVector(),
    })

    const currentHashes = new Map<string, string>([
      ['b-001', 'new_hash'],   // stale: hash changed
      ['b-002', 'some_hash'],  // missing: no embedding at all
    ])

    const stale = EmbeddingRepository.findStale(db, 'bullet', currentHashes)
    expect(stale.length).toBe(2)

    const changed = stale.find(s => s.entity_id === 'b-001')
    expect(changed!.stored_hash).toBe('old_hash')
    expect(changed!.current_hash).toBe('new_hash')

    const missing = stale.find(s => s.entity_id === 'b-002')
    expect(missing!.stored_hash).toBeNull()
    expect(missing!.current_hash).toBe('some_hash')
  })

  it('rejects invalid entity_type via CHECK constraint', () => {
    expect(() => {
      db.run(
        `INSERT INTO embeddings (id, entity_type, entity_id, content_hash, vector)
         VALUES ('x', 'invalid', 'e1', 'h', x'00')`,
      )
    }).toThrow()
  })
})
```

**Acceptance criteria:**
- All repository tests pass.
- `upsert` creates new rows and replaces existing rows for the same `(entity_type, entity_id)`.
- `upsert` does NOT mutate the primary key (`id`) on conflict -- only `content_hash`, `vector`, and `created_at` are updated.
- `findByEntity` returns null for nonexistent entities.
- `findByType` returns all rows of the specified type.
- `deleteByEntity` returns true on success, false on miss.
- `findStale` correctly identifies both missing and hash-mismatched embeddings.
- Vector round-trip (serialize -> store -> retrieve -> deserialize) preserves values within Float32 precision, including correct byte alignment handling.

**Failure criteria:**
- BLOB serialization loses data (byte alignment issue between Float32Array and Buffer).
- `ON CONFLICT ... DO UPDATE` clause fails on bun:sqlite (syntax compatibility).
- CHECK constraint does not fire for invalid entity types.

---

### T69.4: Create EmbeddingService

**File to create:** `packages/core/src/services/embedding-service.ts`
**File to create:** `packages/core/src/services/__tests__/embedding-service.test.ts`

The service orchestrates the model loader, content hashing, repository operations, and cosine similarity computation. It follows the constructor-with-db pattern used by `BulletService`, `PerspectiveService`, etc.

#### 69.4.1: Service implementation

```typescript
// packages/core/src/services/embedding-service.ts

/**
 * EmbeddingService — vector embedding computation, storage, and similarity search.
 *
 * Wraps the model loader and embedding repository to provide:
 * - embed(): compute and store an embedding for any entity
 * - findSimilar(): cosine similarity search against stored embeddings
 * - checkStale(): detect embeddings whose content has changed
 * - refreshStale(): recompute stale embeddings
 * - alignResume(): JD↔Resume alignment via cosine similarity matrix
 * - matchRequirements(): JD requirement matching against bullet/perspective inventory
 * - Fire-and-forget hooks: onBulletCreated, onPerspectiveCreated, onJDCreated, onSourceCreated
 *
 * Error isolation: embedding failures NEVER propagate to calling services.
 * The embed/hook methods catch all errors internally and log warnings.
 *
 * NOTE (AP1): Use the structured logger from Phase 23 if available. If Phase 23
 * has not landed, console.error is acceptable as a temporary measure.
 * TODO: Replace console.error with structured logger (Phase 23).
 *
 * NOTE (I3): checkStale() uses raw SQL queries for cross-entity staleness scans
 * rather than going through individual repositories. This is intentional for
 * performance: a single scan across multiple entity types avoids N+1 queries
 * through repository abstractions.
 *
 * NOTE (I7): refreshStale() uses individual lookupById calls (N+1 pattern) to
 * fetch entity content. For v1, this is acceptable for the expected small number
 * of stale embeddings. Batched lookup is a future optimization if stale counts grow.
 */

import type { Database } from 'bun:sqlite'
import type {
  Result, Bullet, Perspective, JobDescription, Source,
  EmbeddingEntityType, AlignmentReport, RequirementMatchReport,
} from '../types'
import {
  EmbeddingRepository,
} from '../db/repositories/embedding-repository'
import { computeEmbedding, EMBEDDING_DIM } from '../lib/model-loader'
import { parseRequirements } from '../lib/jd-parser'
import { createHash } from 'node:crypto'

// ── Types ────────────────────────────────────────────────────────────

export interface SimilarEntity {
  entity_id: string
  similarity: number
}

export interface StaleEmbedding {
  entity_type: EmbeddingEntityType
  entity_id: string
  stored_hash: string | null
  current_hash: string
}

// ── Helpers ──────────────────────────────────────────────────────────

/** Compute SHA256 hex digest of a string. */
function contentHash(text: string): string {
  return createHash('sha256').update(text, 'utf8').digest('hex')
}

/**
 * Cosine similarity between two Float32Arrays.
 * Both vectors are assumed to be L2-normalized (norm = 1.0),
 * so cosine similarity reduces to dot product.
 */
function cosineSimilarity(a: Float32Array, b: Float32Array): number {
  let dot = 0
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i]
  }
  return dot
}

// ── Service ──────────────────────────────────────────────────────────

export class EmbeddingService {
  constructor(private db: Database) {}

  /**
   * Compute and store an embedding for a single entity.
   *
   * If an embedding already exists and the content hash matches,
   * this is a no-op (returns early). If the hash differs, the
   * embedding is recomputed and upserted.
   */
  async embed(
    entityType: EmbeddingEntityType,
    entityId: string,
    text: string,
  ): Promise<Result<void>> {
    try {
      const hash = contentHash(text)

      // Check if current embedding is still fresh
      const existing = EmbeddingRepository.findByEntity(this.db, entityType, entityId)
      if (existing && existing.content_hash === hash) {
        return { ok: true, data: undefined }
      }

      // Compute embedding vector
      const vector = await computeEmbedding(text)

      if (vector.length !== EMBEDDING_DIM) {
        return {
          ok: false,
          error: {
            code: 'EMBEDDING_ERROR',
            message: `Expected ${EMBEDDING_DIM}-dim vector, got ${vector.length}`,
          },
        }
      }

      // Upsert into DB
      EmbeddingRepository.upsert(this.db, {
        entity_type: entityType,
        entity_id: entityId,
        content_hash: hash,
        vector,
      })

      return { ok: true, data: undefined }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      return {
        ok: false,
        error: { code: 'EMBEDDING_ERROR', message: `Embedding failed: ${message}` },
      }
    }
  }

  /**
   * Find entities of a given type that are semantically similar to the query text.
   *
   * Computes the query embedding, then performs a brute-force cosine similarity
   * scan against all stored embeddings of the specified type. Returns results
   * sorted by descending similarity, filtered by threshold.
   */
  async findSimilar(
    queryText: string,
    entityType: EmbeddingEntityType,
    threshold = 0.5,
    limit = 10,
  ): Promise<Result<SimilarEntity[]>> {
    try {
      const queryVec = await computeEmbedding(queryText)

      const rows = EmbeddingRepository.findByType(this.db, entityType)

      const scored: SimilarEntity[] = []
      for (const row of rows) {
        const storedVec = EmbeddingRepository.deserializeVector(row.vector)
        const sim = cosineSimilarity(queryVec, storedVec)
        if (sim >= threshold) {
          scored.push({ entity_id: row.entity_id, similarity: sim })
        }
      }

      // Sort descending by similarity, take top N
      scored.sort((a, b) => b.similarity - a.similarity)
      const results = scored.slice(0, limit)

      return { ok: true, data: results }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      return {
        ok: false,
        error: { code: 'EMBEDDING_ERROR', message: `Similarity search failed: ${message}` },
      }
    }
  }

  /**
   * Check for stale embeddings across all entity types.
   *
   * Queries the source-of-truth tables (bullets, perspectives, job_descriptions, sources)
   * and compares content hashes against stored embeddings.
   *
   * Returns an array of stale entries with their entity type, ID, and hash info.
   */
  async checkStale(): Promise<Result<StaleEmbedding[]>> {
    try {
      const stale: StaleEmbedding[] = []

      // Check bullets
      const bullets = this.db
        .query('SELECT id, content FROM bullets')
        .all() as Array<{ id: string; content: string }>
      const bulletHashes = new Map(bullets.map(b => [b.id, contentHash(b.content)]))
      const staleBullets = EmbeddingRepository.findStale(this.db, 'bullet', bulletHashes)
      for (const s of staleBullets) {
        stale.push({ entity_type: 'bullet', ...s })
      }

      // Check perspectives
      const perspectives = this.db
        .query('SELECT id, content FROM perspectives')
        .all() as Array<{ id: string; content: string }>
      const perspectiveHashes = new Map(perspectives.map(p => [p.id, contentHash(p.content)]))
      const stalePerspectives = EmbeddingRepository.findStale(this.db, 'perspective', perspectiveHashes)
      for (const s of stalePerspectives) {
        stale.push({ entity_type: 'perspective', ...s })
      }

      // Check sources
      const sources = this.db
        .query('SELECT id, description FROM sources')
        .all() as Array<{ id: string; description: string }>
      const sourceHashes = new Map(sources.map(s => [s.id, contentHash(s.description)]))
      const staleSources = EmbeddingRepository.findStale(this.db, 'source', sourceHashes)
      for (const s of staleSources) {
        stale.push({ entity_type: 'source', ...s })
      }

      // Check JD requirements for staleness (S1)
      // Re-parse requirements from raw_text, compute hash of requirement texts,
      // compare against stored content_hash for each jd_requirement embedding.
      const jds = this.db
        .query('SELECT id, raw_text FROM job_descriptions')
        .all() as Array<{ id: string; raw_text: string }>
      for (const jd of jds) {
        const parsed = parseRequirements(jd.raw_text)
        for (let i = 0; i < parsed.requirements.length; i++) {
          const entityId = `${jd.id}:${i}`
          const currentHash = contentHash(parsed.requirements[i].text)
          const existing = EmbeddingRepository.findByEntity(this.db, 'jd_requirement', entityId)
          if (!existing) {
            stale.push({
              entity_type: 'jd_requirement',
              entity_id: entityId,
              stored_hash: null,
              current_hash: currentHash,
            })
          } else if (existing.content_hash !== currentHash) {
            stale.push({
              entity_type: 'jd_requirement',
              entity_id: entityId,
              stored_hash: existing.content_hash,
              current_hash: currentHash,
            })
          }
        }
      }

      return { ok: true, data: stale }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      return {
        ok: false,
        error: { code: 'EMBEDDING_ERROR', message: `Staleness check failed: ${message}` },
      }
    }
  }

  /**
   * Re-embed all stale entries detected by checkStale().
   *
   * Returns the count of embeddings refreshed.
   */
  async refreshStale(): Promise<Result<number>> {
    const staleResult = await this.checkStale()
    if (!staleResult.ok) return staleResult as Result<number>

    let refreshed = 0

    for (const entry of staleResult.data) {
      let text: string | null = null

      if (entry.entity_type === 'bullet') {
        const row = this.db.query('SELECT content FROM bullets WHERE id = ?').get(entry.entity_id) as { content: string } | null
        text = row?.content ?? null
      } else if (entry.entity_type === 'perspective') {
        const row = this.db.query('SELECT content FROM perspectives WHERE id = ?').get(entry.entity_id) as { content: string } | null
        text = row?.content ?? null
      } else if (entry.entity_type === 'source') {
        const row = this.db.query('SELECT description FROM sources WHERE id = ?').get(entry.entity_id) as { description: string } | null
        text = row?.description ?? null
      } else if (entry.entity_type === 'jd_requirement') {
        // Parse the JD ID and requirement index from the entity_id (format: "{jd_id}:{index}")
        const colonIdx = entry.entity_id.lastIndexOf(':')
        if (colonIdx > 0) {
          const jdId = entry.entity_id.slice(0, colonIdx)
          const reqIdx = parseInt(entry.entity_id.slice(colonIdx + 1), 10)
          const jdRow = this.db.query('SELECT raw_text FROM job_descriptions WHERE id = ?').get(jdId) as { raw_text: string } | null
          if (jdRow) {
            const parsed = parseRequirements(jdRow.raw_text)
            if (reqIdx < parsed.requirements.length) {
              text = parsed.requirements[reqIdx].text
            }
          }
        }
      }

      if (text === null) {
        // Entity was deleted since the staleness check -- remove orphan embedding
        EmbeddingRepository.deleteByEntity(this.db, entry.entity_type, entry.entity_id)
        continue
      }

      const result = await this.embed(entry.entity_type, entry.entity_id, text)
      if (result.ok) refreshed++
    }

    return { ok: true, data: refreshed }
  }

  // ── Alignment Methods ───────────────────────────────────────────────

  /**
   * Align a resume against a job description using embedding similarity.
   *
   * Fetches JD requirement embeddings and resume entry (perspective) embeddings,
   * computes a cosine similarity matrix, classifies each requirement as
   * strong/adjacent/gap, and returns an AlignmentReport.
   *
   * overall_score formula: mean of best_match.similarity per requirement.
   * Requirements with no match (gaps) contribute 0.0 to the mean.
   */
  async alignResume(
    jdId: string,
    resumeId: string,
    opts?: { strongThreshold?: number; adjacentThreshold?: number },
  ): Promise<Result<AlignmentReport>> {
    try {
      const strongThreshold = opts?.strongThreshold ?? 0.75
      const adjacentThreshold = opts?.adjacentThreshold ?? 0.50

      // Fetch JD requirement embeddings
      const jdRows = EmbeddingRepository.findByType(this.db, 'jd_requirement')
        .filter(r => r.entity_id.startsWith(`${jdId}:`))
      if (jdRows.length === 0) {
        return {
          ok: false,
          error: { code: 'EMBEDDING_ERROR', message: `No requirement embeddings found for JD ${jdId}` },
        }
      }

      // Fetch resume entry perspective IDs
      const resumeEntries = this.db.query(
        `SELECT re.perspective_id
         FROM resume_entries re
         JOIN resume_sections rs ON re.section_id = rs.id
         WHERE rs.resume_id = ?`,
      ).all(resumeId) as Array<{ perspective_id: string }>

      if (resumeEntries.length === 0) {
        return {
          ok: false,
          error: { code: 'EMBEDDING_ERROR', message: `No resume entries found for resume ${resumeId}` },
        }
      }

      // Fetch perspective embeddings for resume entries
      const perspectiveEmbeddings: Array<{ entity_id: string; vector: Float32Array }> = []
      for (const entry of resumeEntries) {
        const embRow = EmbeddingRepository.findByEntity(this.db, 'perspective', entry.perspective_id)
        if (embRow) {
          perspectiveEmbeddings.push({
            entity_id: entry.perspective_id,
            vector: EmbeddingRepository.deserializeVector(embRow.vector),
          })
        }
      }

      // Compute similarity matrix and classify
      const matches: AlignmentReport['matches'] = []
      let similaritySum = 0

      for (const jdRow of jdRows) {
        const jdVec = EmbeddingRepository.deserializeVector(jdRow.vector)
        const reqIdx = jdRow.entity_id.split(':').pop()!
        // Look up the original requirement text
        const jdTextRow = this.db.query('SELECT raw_text FROM job_descriptions WHERE id = ?').get(jdId) as { raw_text: string } | null
        let requirementText = `Requirement ${reqIdx}`
        if (jdTextRow) {
          const parsed = parseRequirements(jdTextRow.raw_text)
          const idx = parseInt(reqIdx, 10)
          if (idx < parsed.requirements.length) {
            requirementText = parsed.requirements[idx].text
          }
        }

        let bestSimilarity = 0
        let bestEntityId: string | null = null

        for (const pe of perspectiveEmbeddings) {
          const sim = cosineSimilarity(jdVec, pe.vector)
          if (sim > bestSimilarity) {
            bestSimilarity = sim
            bestEntityId = pe.entity_id
          }
        }

        let classification: 'strong' | 'adjacent' | 'gap'
        if (bestSimilarity >= strongThreshold) {
          classification = 'strong'
        } else if (bestSimilarity >= adjacentThreshold) {
          classification = 'adjacent'
        } else {
          classification = 'gap'
        }

        // Gaps contribute 0.0 to overall_score
        similaritySum += classification === 'gap' ? 0.0 : bestSimilarity

        matches.push({
          requirement_text: requirementText,
          classification,
          best_match: bestEntityId ? {
            entity_id: bestEntityId,
            similarity: bestSimilarity,
          } : null,
        })
      }

      const overallScore = matches.length > 0 ? similaritySum / matches.length : 0

      return {
        ok: true,
        data: {
          job_description_id: jdId,
          resume_id: resumeId,
          overall_score: overallScore,
          matches,
          strong_count: matches.filter(m => m.classification === 'strong').length,
          adjacent_count: matches.filter(m => m.classification === 'adjacent').length,
          gap_count: matches.filter(m => m.classification === 'gap').length,
        },
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      return {
        ok: false,
        error: { code: 'EMBEDDING_ERROR', message: `Alignment failed: ${message}` },
      }
    }
  }

  /**
   * Match JD requirements against the full bullet or perspective inventory.
   *
   * Returns the best-matching entities for each JD requirement, independent of
   * any specific resume. Useful for pre-resume discovery.
   */
  async matchRequirements(
    jdId: string,
    entityType: 'bullet' | 'perspective',
    opts?: { threshold?: number; limit?: number },
  ): Promise<Result<RequirementMatchReport>> {
    try {
      const threshold = opts?.threshold ?? 0.50
      const limit = opts?.limit ?? 10

      // Fetch JD requirement embeddings
      const jdRows = EmbeddingRepository.findByType(this.db, 'jd_requirement')
        .filter(r => r.entity_id.startsWith(`${jdId}:`))
      if (jdRows.length === 0) {
        return {
          ok: false,
          error: { code: 'EMBEDDING_ERROR', message: `No requirement embeddings found for JD ${jdId}` },
        }
      }

      // Fetch all entity embeddings of the requested type
      const entityRows = EmbeddingRepository.findByType(this.db, entityType)

      // Look up requirement texts
      const jdTextRow = this.db.query('SELECT raw_text FROM job_descriptions WHERE id = ?').get(jdId) as { raw_text: string } | null
      const parsedReqs = jdTextRow ? parseRequirements(jdTextRow.raw_text) : null

      const matches: RequirementMatchReport['matches'] = []

      for (const jdRow of jdRows) {
        const jdVec = EmbeddingRepository.deserializeVector(jdRow.vector)
        const reqIdx = parseInt(jdRow.entity_id.split(':').pop()!, 10)
        const requirementText = parsedReqs && reqIdx < parsedReqs.requirements.length
          ? parsedReqs.requirements[reqIdx].text
          : `Requirement ${reqIdx}`

        // Score all entities against this requirement
        const candidates: Array<{ entity_id: string; content: string; similarity: number }> = []
        for (const entityRow of entityRows) {
          const entityVec = EmbeddingRepository.deserializeVector(entityRow.vector)
          const sim = cosineSimilarity(jdVec, entityVec)
          if (sim >= threshold) {
            // Look up content for the entity
            let content = ''
            if (entityType === 'bullet') {
              const row = this.db.query('SELECT content FROM bullets WHERE id = ?').get(entityRow.entity_id) as { content: string } | null
              content = row?.content ?? ''
            } else {
              const row = this.db.query('SELECT content FROM perspectives WHERE id = ?').get(entityRow.entity_id) as { content: string } | null
              content = row?.content ?? ''
            }
            candidates.push({ entity_id: entityRow.entity_id, content, similarity: sim })
          }
        }

        // Sort by similarity descending, take top N
        candidates.sort((a, b) => b.similarity - a.similarity)
        matches.push({
          requirement_text: requirementText,
          candidates: candidates.slice(0, limit),
        })
      }

      return {
        ok: true,
        data: {
          job_description_id: jdId,
          matches,
        },
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      return {
        ok: false,
        error: { code: 'EMBEDDING_ERROR', message: `Requirement matching failed: ${message}` },
      }
    }
  }

  // ── Fire-and-Forget Hooks ──────────────────────────────────────────

  /**
   * Embed a bullet's content after creation. Fire-and-forget.
   * Errors are caught and logged, never propagated.
   */
  async onBulletCreated(bullet: Bullet): Promise<void> {
    try {
      await this.embed('bullet', bullet.id, bullet.content)
    } catch (err) {
      // TODO: Replace with structured logger (Phase 23)
      console.error(`[EmbeddingService] Failed to embed bullet ${bullet.id}:`, err)
    }
  }

  /**
   * Embed a perspective's content after creation. Fire-and-forget.
   * Errors are caught and logged, never propagated.
   */
  async onPerspectiveCreated(perspective: Perspective): Promise<void> {
    try {
      await this.embed('perspective', perspective.id, perspective.content)
    } catch (err) {
      // TODO: Replace with structured logger (Phase 23)
      console.error(`[EmbeddingService] Failed to embed perspective ${perspective.id}:`, err)
    }
  }

  /**
   * Parse and embed JD requirements after JD creation. Fire-and-forget.
   *
   * Accepts pre-parsed requirement strings. Each requirement is embedded
   * with entity_type='jd_requirement' and entity_id='{jd_id}:{index}'.
   */
  async onJDCreated(jd: JobDescription, requirements: string[]): Promise<void> {
    try {
      for (let i = 0; i < requirements.length; i++) {
        const entityId = `${jd.id}:${i}`
        await this.embed('jd_requirement', entityId, requirements[i])
      }
    } catch (err) {
      // TODO: Replace with structured logger (Phase 23)
      console.error(`[EmbeddingService] Failed to embed JD requirements for ${jd.id}:`, err)
    }
  }

  /**
   * Re-parse and re-embed JD requirements when raw_text is updated.
   * Called from JobDescriptionService when raw_text changes.
   *
   * Deletes old requirement embeddings for this JD and re-embeds from scratch.
   */
  async onJDUpdated(jd: JobDescription): Promise<void> {
    try {
      // Delete all existing requirement embeddings for this JD
      const existing = EmbeddingRepository.findByType(this.db, 'jd_requirement')
        .filter(r => r.entity_id.startsWith(`${jd.id}:`))
      for (const row of existing) {
        EmbeddingRepository.deleteByEntity(this.db, 'jd_requirement', row.entity_id)
      }

      // Re-parse and re-embed
      const parsed = parseRequirements(jd.raw_text)
      for (let i = 0; i < parsed.requirements.length; i++) {
        const entityId = `${jd.id}:${i}`
        await this.embed('jd_requirement', entityId, parsed.requirements[i].text)
      }
    } catch (err) {
      // TODO: Replace with structured logger (Phase 23)
      console.error(`[EmbeddingService] Failed to re-embed JD requirements for ${jd.id}:`, err)
    }
  }

  /**
   * Embed a source's description after creation. Fire-and-forget.
   * Errors are caught and logged, never propagated.
   */
  async onSourceCreated(source: Source): Promise<void> {
    try {
      await this.embed('source', source.id, source.description)
    } catch (err) {
      // TODO: Replace with structured logger (Phase 23)
      console.error(`[EmbeddingService] Failed to embed source ${source.id}:`, err)
    }
  }
}
```

#### 69.4.2: Service tests

```typescript
// packages/core/src/services/__tests__/embedding-service.test.ts

import { describe, it, expect, beforeEach, afterAll } from 'bun:test'
import { getDatabase } from '../../db/connection'
import { runMigrations } from '../../db/migrate'
import { EmbeddingService } from '../embedding-service'
import { EmbeddingRepository } from '../../db/repositories/embedding-repository'
import { resetPipeline } from '../../lib/model-loader'
import { join } from 'node:path'

function setupDb() {
  const db = getDatabase(':memory:')
  runMigrations(db, join(import.meta.dir, '../../db/migrations'))
  return db
}

// AP2: Clean up module-level mutable singleton to prevent test pollution
afterAll(() => {
  resetPipeline()
})

describe('EmbeddingService', () => {
  let db: ReturnType<typeof getDatabase>
  let service: EmbeddingService

  beforeEach(() => {
    db = setupDb()
    service = new EmbeddingService(db)
  })

  describe('embed()', () => {
    it('computes and stores an embedding', async () => {
      const result = await service.embed('bullet', 'b-001', 'Designed Kubernetes clusters')
      expect(result.ok).toBe(true)

      const row = EmbeddingRepository.findByEntity(db, 'bullet', 'b-001')
      expect(row).not.toBeNull()
      expect(row!.entity_type).toBe('bullet')
      expect(row!.entity_id).toBe('b-001')

      const vec = EmbeddingRepository.deserializeVector(row!.vector)
      expect(vec.length).toBe(384)
    })

    it('skips recomputation if content hash matches', async () => {
      await service.embed('bullet', 'b-001', 'Same text')
      const row1 = EmbeddingRepository.findByEntity(db, 'bullet', 'b-001')

      await service.embed('bullet', 'b-001', 'Same text')
      const row2 = EmbeddingRepository.findByEntity(db, 'bullet', 'b-001')

      // Same created_at means it was not re-upserted
      expect(row1!.created_at).toBe(row2!.created_at)
    })

    it('re-embeds when content changes', async () => {
      await service.embed('bullet', 'b-001', 'Original text')
      const row1 = EmbeddingRepository.findByEntity(db, 'bullet', 'b-001')

      await service.embed('bullet', 'b-001', 'Updated text')
      const row2 = EmbeddingRepository.findByEntity(db, 'bullet', 'b-001')

      expect(row1!.content_hash).not.toBe(row2!.content_hash)
    })
  })

  describe('findSimilar()', () => {
    it('returns similar entities above threshold', async () => {
      // Seed embeddings
      await service.embed('bullet', 'b-infra', 'Managed AWS infrastructure and deployed with Terraform')
      await service.embed('bullet', 'b-sec', 'Implemented zero-trust security architecture')
      await service.embed('bullet', 'b-food', 'Baked artisan sourdough bread')

      const result = await service.findSimilar(
        'Cloud infrastructure automation',
        'bullet',
        0.3,
        10,
      )
      expect(result.ok).toBe(true)
      if (!result.ok) return

      // b-infra should be most similar
      expect(result.data.length).toBeGreaterThan(0)
      expect(result.data[0].entity_id).toBe('b-infra')

      // b-food should be least similar (possibly filtered out)
      const foodMatch = result.data.find(m => m.entity_id === 'b-food')
      if (foodMatch) {
        expect(foodMatch.similarity).toBeLessThan(result.data[0].similarity)
      }
    })

    it('respects threshold filter', async () => {
      await service.embed('bullet', 'b-1', 'Python web development with Django')

      const highThreshold = await service.findSimilar('Cooking pasta', 'bullet', 0.9)
      expect(highThreshold.ok).toBe(true)
      if (highThreshold.ok) {
        expect(highThreshold.data.length).toBe(0)
      }
    })

    it('respects limit parameter', async () => {
      for (let i = 0; i < 5; i++) {
        await service.embed('bullet', `b-${i}`, `Software engineering task number ${i}`)
      }

      const result = await service.findSimilar('Engineering', 'bullet', 0.0, 3)
      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data.length).toBeLessThanOrEqual(3)
      }
    })
  })

  describe('checkStale() + refreshStale()', () => {
    it('detects stale bullet after content update and refreshes it', async () => {
      // Seed a bullet in the DB and embed it
      db.run(
        `INSERT INTO bullets (id, content, source_content_snapshot, status, created_at)
         VALUES ('b-stale', 'Original content for stale test', '', 'approved', datetime('now'))`,
      )
      await service.embed('bullet', 'b-stale', 'Original content for stale test')

      // Verify it is not stale initially
      const freshResult = await service.checkStale()
      expect(freshResult.ok).toBe(true)
      if (freshResult.ok) {
        const staleBullet = freshResult.data.find(
          s => s.entity_type === 'bullet' && s.entity_id === 'b-stale',
        )
        expect(staleBullet).toBeUndefined()
      }

      // Update bullet content directly via SQL (simulating an external edit)
      db.run(`UPDATE bullets SET content = 'Updated content for stale test' WHERE id = 'b-stale'`)

      // Now it should be stale
      const staleResult = await service.checkStale()
      expect(staleResult.ok).toBe(true)
      if (staleResult.ok) {
        const staleBullet = staleResult.data.find(
          s => s.entity_type === 'bullet' && s.entity_id === 'b-stale',
        )
        expect(staleBullet).toBeDefined()
      }

      // Refresh and verify it is no longer stale
      const refreshResult = await service.refreshStale()
      expect(refreshResult.ok).toBe(true)

      const afterRefresh = await service.checkStale()
      expect(afterRefresh.ok).toBe(true)
      if (afterRefresh.ok) {
        const staleBullet = afterRefresh.data.find(
          s => s.entity_type === 'bullet' && s.entity_id === 'b-stale',
        )
        expect(staleBullet).toBeUndefined()
      }
    })

    it('cleans up orphan embeddings for deleted entities', async () => {
      // Embed a bullet, then delete the bullet from the source table
      db.run(
        `INSERT INTO bullets (id, content, source_content_snapshot, status, created_at)
         VALUES ('b-orphan', 'Content that will be orphaned', '', 'approved', datetime('now'))`,
      )
      await service.embed('bullet', 'b-orphan', 'Content that will be orphaned')

      // Delete the source entity
      db.run(`DELETE FROM bullets WHERE id = 'b-orphan'`)

      // The embedding should still exist
      const beforeRefresh = EmbeddingRepository.findByEntity(db, 'bullet', 'b-orphan')
      expect(beforeRefresh).not.toBeNull()

      // refreshStale should clean up the orphan
      await service.refreshStale()

      const afterRefresh = EmbeddingRepository.findByEntity(db, 'bullet', 'b-orphan')
      expect(afterRefresh).toBeNull()
    })
  })

  describe('onBulletCreated()', () => {
    it('embeds bullet content without throwing', async () => {
      // M2 note: Use `as Bullet` cast for minimal fixture subset.
      // If createBulletFixture(overrides?) test factory exists, use that instead.
      const bullet = {
        id: 'b-test',
        content: 'Built CI/CD pipelines with GitHub Actions',
        source_content_snapshot: '',
        technologies: [],
        metrics: null,
        domain: null,
        status: 'pending_review' as const,
        rejection_reason: null,
        prompt_log_id: null,
        approved_at: null,
        approved_by: null,
        notes: null,
        created_at: new Date().toISOString(),
      } as Bullet

      // Should not throw
      await service.onBulletCreated(bullet)

      const row = EmbeddingRepository.findByEntity(db, 'bullet', 'b-test')
      expect(row).not.toBeNull()
    })
  })

  describe('onJDCreated()', () => {
    it('embeds each requirement with indexed entity ID', async () => {
      const jd = {
        id: 'jd-001',
        organization_id: null,
        title: 'Senior Engineer',
        url: null,
        raw_text: 'full text here',
        status: 'interested' as const,
        salary_range: null,
        location: null,
        notes: null,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      }

      const requirements = [
        '5+ years of experience with Python',
        'Experience with Kubernetes and Docker',
        'Strong understanding of distributed systems',
      ]

      await service.onJDCreated(jd, requirements)

      for (let i = 0; i < requirements.length; i++) {
        const row = EmbeddingRepository.findByEntity(db, 'jd_requirement', `jd-001:${i}`)
        expect(row).not.toBeNull()
      }
    })
  })
})
```

**Acceptance criteria:**
- `embed()` computes a vector via the model loader, hashes the content, and upserts into the repository.
- `embed()` is idempotent: calling with the same text and entity is a no-op (hash check).
- `findSimilar()` returns entities sorted by descending cosine similarity, filtered by threshold and limit.
- `checkStale()` identifies embeddings whose content has diverged from the source-of-truth tables, including JD requirement staleness.
- `refreshStale()` recomputes stale embeddings and removes orphans.
- `alignResume()` returns an `AlignmentReport` with strong/adjacent/gap classifications and an `overall_score`.
- `matchRequirements()` returns a `RequirementMatchReport` with candidates per requirement.
- All fire-and-forget hooks (`onBulletCreated`, `onPerspectiveCreated`, `onJDCreated`, `onJDUpdated`, `onSourceCreated`) catch errors internally and log to stderr.
- No embedding failure ever propagates as an exception to calling code.

**Failure criteria:**
- `findSimilar()` returns results sorted incorrectly (ascending instead of descending).
- `checkStale()` misses a changed entity or falsely reports a fresh entity as stale.
- `checkStale()` does not detect JD requirement staleness.
- Fire-and-forget hooks throw exceptions that propagate to the caller.
- Cosine similarity computation is incorrect (e.g., not handling normalized vectors).
- `alignResume()` overall_score formula does not correctly average with gaps contributing 0.0.

---

### T69.5: Create JD Requirement Parser

**File to create:** `packages/core/src/lib/jd-parser.ts`
**File to create:** `packages/core/src/lib/__tests__/jd-parser.test.ts`

A programmatic (non-AI) parser that splits raw JD text into individual requirement strings with confidence scores. High confidence means structured list format was detected; low confidence means prose paragraphs that may need manual review.

#### 69.5.1: Parser implementation

```typescript
// packages/core/src/lib/jd-parser.ts

/**
 * JD Requirement Parser — programmatic extraction of individual requirements
 * from raw job description text.
 *
 * Splits on bullet points, numbered lists, and line breaks within
 * recognized sections (Requirements, Qualifications, etc.).
 *
 * Confidence scoring:
 * - 0.9: Structured list with clear bullet points under requirement sections
 * - 0.7: Structured list under responsibility sections (lower weight for alignment)
 * - 0.7-0.9: Semi-structured (line breaks, mixed formatting)
 * - 0.5-0.7: Some structure detected but ambiguous
 * - < 0.5: Prose paragraphs, hard to parse reliably
 *
 * NOTE (M4): Responsibility-section requirements are scored at 0.7 instead of 0.9
 * because responsibilities are less precise signals for skills matching than explicit
 * requirements/qualifications sections.
 */

// ── Types ────────────────────────────────────────────────────────────

export interface ParsedRequirement {
  text: string
  confidence: number
  section: string | null  // which section it was found in (e.g., 'Requirements')
}

export interface ParsedRequirements {
  requirements: ParsedRequirement[]
  overall_confidence: number
}

// ── Section Detection ────────────────────────────────────────────────

/**
 * Known section headers that typically contain requirements.
 * Order matters: earlier entries are preferred when sections overlap.
 */
const REQUIREMENT_SECTIONS = [
  /^#{1,3}\s*(requirements|required\s+qualifications|minimum\s+qualifications|must[\s-]haves?)/im,
  /^#{1,3}\s*(qualifications|preferred\s+qualifications|desired\s+qualifications)/im,
  /^#{1,3}\s*(what\s+you('ll|\s+will)\s+(need|bring)|what\s+we('re|\s+are)\s+looking\s+for)/im,
  /^#{1,3}\s*(responsibilities|key\s+responsibilities|role\s+responsibilities)/im,
  /^#{1,3}\s*(nice[\s-]to[\s-]haves?|preferred|bonus|plus)/im,
  /^#{1,3}\s*(skills|technical\s+skills|required\s+skills)/im,
  /^\*{0,2}(requirements|qualifications|responsibilities|skills|what\s+you.+need)\*{0,2}\s*:?\s*$/im,
  /^(requirements|qualifications|responsibilities|skills|what\s+you.+need)\s*:?\s*$/im,
]

/** Patterns that match responsibility-section headers specifically. */
const RESPONSIBILITY_SECTION_PATTERNS = [
  /responsibilities/i,
]

/**
 * Section headers that indicate the end of requirements (e.g., Benefits, About Us).
 */
const NON_REQUIREMENT_SECTIONS = [
  /^#{1,3}\s*(benefits|perks|compensation|salary|about\s+(us|the\s+company|the\s+team))/im,
  /^#{1,3}\s*(how\s+to\s+apply|application\s+process|equal\s+opportunity)/im,
  /^#{1,3}\s*(company\s+(overview|description)|our\s+(mission|values|culture))/im,
  /^\*{0,2}(benefits|perks|about\s+(us|the))\*{0,2}\s*:?\s*$/im,
  /^(benefits|perks|about\s+(us|the))\s*:?\s*$/im,
]

// ── Bullet/List Detection ────────────────────────────────────────────

/** Matches lines starting with bullet characters or numbered list markers. */
const BULLET_PATTERN = /^[\s]*(?:[-*+]|\d+[.)]\s|[a-z][.)]\s|>\s)/
const SEMICOLON_LIST_PATTERN = /;\s*/

// ── Core Parser ──────────────────────────────────────────────────────

/**
 * Parse requirements from raw job description text.
 *
 * Strategy:
 * 1. Guard against excessively long input (max 100,000 chars).
 * 2. Detect requirement sections by header patterns.
 * 3. Extract content between requirement headers and next section header.
 * 4. Split section content on bullet points, numbered lists, or line breaks.
 * 5. Score each requirement based on how structured its source was.
 * 6. If no sections detected, attempt to parse the entire text.
 *
 * NOTE (IN3): If rawText exceeds 100,000 characters, return empty requirements
 * with confidence 0. This is a safety guard against pathologically long inputs
 * that could cause excessive processing time in the synchronous request path.
 */
export function parseRequirements(rawText: string): ParsedRequirements {
  if (!rawText || rawText.trim().length === 0) {
    return { requirements: [], overall_confidence: 0 }
  }

  // IN3: Max-length guard
  if (rawText.length > 100_000) {
    return { requirements: [], overall_confidence: 0 }
  }

  const lines = rawText.split('\n')
  const sections = detectSections(lines)

  let requirements: ParsedRequirement[]

  if (sections.length > 0) {
    // Parse structured sections
    requirements = []
    for (const section of sections) {
      const isResponsibilitySection = RESPONSIBILITY_SECTION_PATTERNS.some(p => p.test(section.name))
      const parsed = parseSectionContent(section.content, section.name, isResponsibilitySection)
      requirements.push(...parsed)
    }
  } else {
    // No sections detected -- try parsing the whole text
    requirements = parseSectionContent(rawText, null, false)
    // Lower confidence since we could not find section boundaries
    for (const req of requirements) {
      req.confidence *= 0.6
    }
  }

  // Filter out empty or too-short requirements
  requirements = requirements.filter(r => r.text.length >= 10)

  // Deduplicate by normalized text
  const seen = new Set<string>()
  requirements = requirements.filter(r => {
    const key = r.text.toLowerCase().trim()
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })

  const overall_confidence = requirements.length > 0
    ? requirements.reduce((sum, r) => sum + r.confidence, 0) / requirements.length
    : 0

  return { requirements, overall_confidence }
}

// ── Internal Helpers ─────────────────────────────────────────────────

interface DetectedSection {
  name: string
  content: string
  startLine: number
}

function detectSections(lines: string[]): DetectedSection[] {
  const sections: DetectedSection[] = []
  let currentSection: { name: string; startLine: number; lines: string[] } | null = null

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]

    // Check if this line is a non-requirement section header (end current section)
    const isEndSection = NON_REQUIREMENT_SECTIONS.some(pat => pat.test(line))
    if (isEndSection && currentSection) {
      sections.push({
        name: currentSection.name,
        content: currentSection.lines.join('\n'),
        startLine: currentSection.startLine,
      })
      currentSection = null
      continue
    }

    // Check if this line is a requirement section header (start new section)
    for (const pat of REQUIREMENT_SECTIONS) {
      const match = line.match(pat)
      if (match) {
        // Close previous section if any
        if (currentSection) {
          sections.push({
            name: currentSection.name,
            content: currentSection.lines.join('\n'),
            startLine: currentSection.startLine,
          })
        }
        currentSection = {
          name: match[1] || 'Requirements',
          startLine: i,
          lines: [],
        }
        break
      }
    }

    // Accumulate lines into current section
    if (currentSection && !REQUIREMENT_SECTIONS.some(p => p.test(line))) {
      currentSection.lines.push(line)
    }
  }

  // Close final section
  if (currentSection) {
    sections.push({
      name: currentSection.name,
      content: currentSection.lines.join('\n'),
      startLine: currentSection.startLine,
    })
  }

  return sections
}

function parseSectionContent(
  content: string,
  sectionName: string | null,
  isResponsibilitySection: boolean,
): ParsedRequirement[] {
  const requirements: ParsedRequirement[] = []
  const lines = content.split('\n')

  // M4: Responsibility sections get lower base confidence (0.7 vs 0.9)
  const baseConfidence = isResponsibilitySection ? 0.7 : 0.9

  // Count how many lines look like bullets
  const bulletLines = lines.filter(l => BULLET_PATTERN.test(l))
  const isBulletList = bulletLines.length >= 2

  if (isBulletList) {
    // Structured bullet list -- high confidence
    for (const line of lines) {
      const trimmed = line.replace(BULLET_PATTERN, '').trim()
      if (trimmed.length === 0) continue

      // Check for semicolon-delimited sub-items
      if (SEMICOLON_LIST_PATTERN.test(trimmed) && trimmed.split(';').length >= 3) {
        for (const part of trimmed.split(';')) {
          const sub = part.trim()
          if (sub.length >= 10) {
            requirements.push({ text: sub, confidence: baseConfidence - 0.1, section: sectionName })
          }
        }
      } else {
        requirements.push({ text: trimmed, confidence: baseConfidence, section: sectionName })
      }
    }
  } else {
    // No clear bullet structure -- try splitting on line breaks
    const nonEmpty = lines.map(l => l.trim()).filter(l => l.length > 0)

    if (nonEmpty.length === 1) {
      // Single block of text -- try splitting on sentences
      // AP3: Use forward-looking split instead of lookbehind regex for JSC compatibility.
      // Tradeoff: This pattern may incorrectly split on abbreviations like "U.S. Army"
      // or "Dr. Smith", but is more resilient across JS engines than lookbehind.
      const sentences = nonEmpty[0].split(/\.\s+(?=[A-Z])/)
      for (const sentence of sentences) {
        const trimmed = sentence.trim()
        if (trimmed.length >= 10) {
          requirements.push({ text: trimmed, confidence: 0.4, section: sectionName })
        }
      }
    } else {
      // Multiple lines without bullet markers -- medium confidence
      for (const line of nonEmpty) {
        requirements.push({ text: line, confidence: 0.6, section: sectionName })
      }
    }
  }

  return requirements
}
```

#### 69.5.2: Parser tests

```typescript
// packages/core/src/lib/__tests__/jd-parser.test.ts

import { describe, it, expect } from 'bun:test'
import { parseRequirements } from '../jd-parser'

describe('parseRequirements', () => {
  it('returns empty for empty input', () => {
    const result = parseRequirements('')
    expect(result.requirements).toEqual([])
    expect(result.overall_confidence).toBe(0)
  })

  it('returns empty for excessively long input (>100k chars)', () => {
    const longText = 'x'.repeat(100_001)
    const result = parseRequirements(longText)
    expect(result.requirements).toEqual([])
    expect(result.overall_confidence).toBe(0)
  })

  it('parses a structured bullet list under a "Requirements" header', () => {
    const jd = `
## About the Role
We are looking for a senior engineer to join our team.

## Requirements
- 5+ years of experience with Python or Go
- Experience designing distributed systems
- Strong knowledge of AWS (EC2, ECS, Lambda)
- Familiarity with CI/CD pipelines (GitHub Actions, Jenkins)
- Excellent communication skills

## Benefits
- Competitive salary
- Health insurance
`
    const result = parseRequirements(jd)

    expect(result.requirements.length).toBe(5)
    expect(result.overall_confidence).toBeGreaterThan(0.7)
    expect(result.requirements[0].text).toContain('Python or Go')
    expect(result.requirements[0].section).toMatch(/requirements/i)
    expect(result.requirements[0].confidence).toBeGreaterThanOrEqual(0.9)
  })

  it('parses a numbered list', () => {
    const jd = `
Qualifications:
1. Bachelor's degree in Computer Science or equivalent
2. 3+ years working with React and TypeScript
3. Experience with SQL databases (PostgreSQL preferred)
4. Understanding of RESTful API design
`
    const result = parseRequirements(jd)

    expect(result.requirements.length).toBe(4)
    expect(result.overall_confidence).toBeGreaterThan(0.7)
    expect(result.requirements[1].text).toContain('React and TypeScript')
  })

  it('handles mixed sections with both required and preferred', () => {
    const jd = `
## Required Qualifications
- BS/MS in Computer Science
- 5+ years building backend services
- Proficiency in Java or Kotlin

## Preferred Qualifications
- Experience with Kafka or RabbitMQ
- Kubernetes certification (CKA/CKAD)
`
    const result = parseRequirements(jd)

    expect(result.requirements.length).toBe(5)
    // Should have requirements from both sections
    const sections = new Set(result.requirements.map(r => r.section))
    expect(sections.size).toBe(2)
  })

  it('assigns lower confidence to prose paragraphs (no section headers)', () => {
    const jd = `We need someone who knows Python and has experience with cloud infrastructure. The ideal candidate will have worked with Docker and Kubernetes in production environments. Strong problem-solving skills are essential.`

    const result = parseRequirements(jd)

    expect(result.requirements.length).toBeGreaterThan(0)
    // Prose gets lower confidence (< 0.5 due to no sections + sentence splitting)
    expect(result.overall_confidence).toBeLessThan(0.5)
  })

  it('deduplicates identical requirements', () => {
    const jd = `
## Requirements
- Experience with Python
- Knowledge of AWS

## Qualifications
- Experience with Python
- Familiarity with Docker
`
    const result = parseRequirements(jd)

    const pythonReqs = result.requirements.filter(r =>
      r.text.toLowerCase().includes('experience with python'),
    )
    expect(pythonReqs.length).toBe(1)
  })

  it('filters out very short lines', () => {
    const jd = `
## Requirements
- Python
- Go
- Experience designing and operating large-scale distributed systems
`
    const result = parseRequirements(jd)

    // "Python" and "Go" are < 10 chars, should be filtered
    expect(result.requirements.length).toBe(1)
    expect(result.requirements[0].text).toContain('distributed systems')
  })

  it('handles "What you will need" style headers', () => {
    const jd = `
## What You'll Need
- Strong background in machine learning
- Experience with PyTorch or TensorFlow
- Published research is a plus
`
    const result = parseRequirements(jd)

    expect(result.requirements.length).toBe(3)
    expect(result.overall_confidence).toBeGreaterThan(0.7)
  })

  it('stops parsing at non-requirement sections', () => {
    const jd = `
## Responsibilities
- Design and implement APIs
- Mentor junior engineers

## About Us
We are a fast-growing startup building the future of AI.
Our mission is to democratize machine learning.
`
    const result = parseRequirements(jd)

    // "About Us" content should not appear as requirements
    const hasAboutUs = result.requirements.some(r => r.text.includes('fast-growing'))
    expect(hasAboutUs).toBe(false)
  })

  it('scores responsibility-section requirements lower than requirement-section (M4)', () => {
    const jd = `
## Requirements
- 5+ years of experience with Python or Go

## Responsibilities
- Design and implement APIs
- Mentor junior engineers
`
    const result = parseRequirements(jd)

    const reqItems = result.requirements.filter(r => r.section?.match(/requirements/i))
    const respItems = result.requirements.filter(r => r.section?.match(/responsibilities/i))

    expect(reqItems.length).toBeGreaterThan(0)
    expect(respItems.length).toBeGreaterThan(0)

    // Requirement-section items should have higher confidence (0.9) than responsibility-section (0.7)
    for (const item of reqItems) {
      expect(item.confidence).toBeGreaterThanOrEqual(0.9)
    }
    for (const item of respItems) {
      expect(item.confidence).toBeLessThanOrEqual(0.7)
    }
  })

  it('handles "Nice to Have" (no s) header variant (G4)', () => {
    const jd = `
## Nice to Have
- Experience with GraphQL and federation
- Familiarity with event-driven architecture
`
    const result = parseRequirements(jd)
    expect(result.requirements.length).toBe(2)
  })

  it('handles "Nice-to-Have" (hyphenated) header variant (G4)', () => {
    const jd = `
## Nice-to-Have
- Experience with GraphQL and federation
- Familiarity with event-driven architecture
`
    const result = parseRequirements(jd)
    expect(result.requirements.length).toBe(2)
  })
})
```

#### 69.5.3: Example input/output

**Input (structured):**
```
## Requirements
- 5+ years of experience with Python or Go
- Experience designing distributed systems
- Strong knowledge of AWS (EC2, ECS, Lambda)
```

**Output:**
```json
{
  "requirements": [
    {"text": "5+ years of experience with Python or Go", "confidence": 0.9, "section": "Requirements"},
    {"text": "Experience designing distributed systems", "confidence": 0.9, "section": "Requirements"},
    {"text": "Strong knowledge of AWS (EC2, ECS, Lambda)", "confidence": 0.9, "section": "Requirements"}
  ],
  "overall_confidence": 0.9
}
```

**Input (prose, no headers):**
```
We need someone who knows Python and has experience with cloud infrastructure. The ideal candidate will have worked with Docker and Kubernetes in production environments.
```

**Output:**
```json
{
  "requirements": [
    {"text": "We need someone who knows Python and has experience with cloud infrastructure", "confidence": 0.24, "section": null},
    {"text": "The ideal candidate will have worked with Docker and Kubernetes in production environments.", "confidence": 0.24, "section": null}
  ],
  "overall_confidence": 0.24
}
```

**Acceptance criteria:**
- `parseRequirements()` returns `ParsedRequirements` with an array of `{text, confidence, section}` objects.
- Structured bullet lists under recognized headers produce confidence >= 0.9 (or 0.7 for responsibility sections).
- Prose paragraphs without headers produce confidence < 0.5.
- Empty input returns `{requirements: [], overall_confidence: 0}`.
- Input exceeding 100,000 characters returns `{requirements: [], overall_confidence: 0}`.
- Requirements shorter than 10 characters are filtered out.
- Duplicate requirements (case-insensitive) are deduplicated.
- Content under non-requirement headers (Benefits, About Us) is excluded.
- Parser handles markdown headers (`##`), plain text headers (`Requirements:`), and bold headers (`**Requirements**`).
- Sentence splitting uses `split(/\.\s+(?=[A-Z])/)` (JSC-safe, no lookbehind).

**Failure criteria:**
- Parser includes "Benefits" or "About Us" content as requirements.
- Confidence scores are not in [0, 1] range.
- Parser crashes on malformed/unusual JD text.
- Parser returns requirements from non-requirement sections.

---

### T69.6: Integrate Fire-and-Forget Hooks

**File to modify:** `packages/core/src/services/derivation-service.ts`
**File to modify:** `packages/core/src/services/job-description-service.ts`
**File to modify:** `packages/core/src/services/source-service.ts`

After entity creation succeeds and the transaction is committed, queue an async embedding computation that does not block the response. The pattern uses `queueMicrotask` to schedule the embedding after the current synchronous operation completes.

> **I4 note:** `queueMicrotask` schedules the embedding after the response is sent, but ONNX inference is synchronous CPU work. For <1s single-user request rate, this is acceptable. If throughput becomes a concern, move embedding to a Worker thread.

#### 69.6.1: Hook into DerivationService (bullet creation)

In `packages/core/src/services/derivation-service.ts`, after `txn()` in `deriveBulletsFromSource`:

```typescript
// Existing code (keep as-is through txn()):
      txn()

      // NEW: Fire-and-forget embedding for each created bullet
      if (this.embeddingService) {
        for (const bullet of bullets) {
          queueMicrotask(() =>
            this.embeddingService!.onBulletCreated(bullet).catch(err =>
              // TODO: Replace with structured logger (Phase 23)
              console.error(`[DerivationService] Embedding hook failed for bullet ${bullet.id}:`, err)
            )
          )
        }
      }

      return { ok: true, data: bullets }
```

The constructor must accept an optional `EmbeddingService`:

```typescript
export class DerivationService {
  constructor(
    private db: Database,
    private derivingBullets: Set<string>,
    private embeddingService?: EmbeddingService,
  ) {}
```

Similarly, after `txn()` in `derivePerspectivesFromBullet`:

```typescript
      txn()
      this.derivingBullets.delete(bulletId)

      // NEW: Fire-and-forget embedding for the created perspective
      if (this.embeddingService) {
        queueMicrotask(() =>
          this.embeddingService!.onPerspectiveCreated(perspective).catch(err =>
            // TODO: Replace with structured logger (Phase 23)
            console.error(`[DerivationService] Embedding hook failed for perspective ${perspective.id}:`, err)
          )
        )
      }

      return { ok: true, data: perspective }
```

#### 69.6.2: Hook into JobDescriptionService (JD creation and update)

In `packages/core/src/services/job-description-service.ts`, after JD creation:

```typescript
import { parseRequirements } from '../lib/jd-parser'
import type { EmbeddingService } from './embedding-service'

export class JobDescriptionService {
  private embeddingService?: EmbeddingService

  constructor(private db: Database) {}

  /**
   * Set the embedding service for fire-and-forget hooks.
   *
   * NOTE (I6): Constructor injection would be preferred but requires careful
   * ordering in createServices(). The setter pattern is used here because
   * EmbeddingService initialization is async (model loading) and may complete
   * after createServices(). This avoids circular dependency issues.
   */
  setEmbeddingService(svc: EmbeddingService): void {
    this.embeddingService = svc
  }

  create(input: CreateJobDescription): Result<JobDescriptionWithOrg> {
    // ... existing validation ...

    const jd = JDRepo.create(this.db, input)

    // NEW: Fire-and-forget requirement parsing and embedding
    if (this.embeddingService) {
      const parsed = parseRequirements(input.raw_text)
      const requirementTexts = parsed.requirements.map(r => r.text)
      queueMicrotask(() =>
        this.embeddingService!.onJDCreated(jd, requirementTexts).catch(err =>
          // TODO: Replace with structured logger (Phase 23)
          console.error(`[JobDescriptionService] Embedding hook failed for JD ${jd.id}:`, err)
        )
      )
    }

    return { ok: true, data: jd }
  }

  // When raw_text is updated, re-embed requirements (S1)
  update(id: string, input: UpdateJobDescription): Result<JobDescriptionWithOrg> {
    // ... existing update logic ...

    const jd = JDRepo.update(this.db, id, input)

    // If raw_text was updated, re-embed requirements
    if (input.raw_text && this.embeddingService) {
      queueMicrotask(() =>
        this.embeddingService!.onJDUpdated(jd).catch(err =>
          // TODO: Replace with structured logger (Phase 23)
          console.error(`[JobDescriptionService] Re-embedding hook failed for JD ${jd.id}:`, err)
        )
      )
    }

    return { ok: true, data: jd }
  }
```

#### 69.6.3: Hook into SourceService (source creation)

In `packages/core/src/services/source-service.ts`, after source creation:

```typescript
import type { EmbeddingService } from './embedding-service'

export class SourceService {
  private embeddingService?: EmbeddingService

  // ... existing constructor ...

  setEmbeddingService(svc: EmbeddingService): void {
    this.embeddingService = svc
  }

  create(input: CreateSource): Result<Source> {
    // ... existing creation logic ...

    const source = SourceRepo.create(this.db, input)

    // NEW: Fire-and-forget embedding for source description
    if (this.embeddingService) {
      queueMicrotask(() =>
        this.embeddingService!.onSourceCreated(source).catch(err =>
          // TODO: Replace with structured logger (Phase 23)
          console.error(`[SourceService] Embedding hook failed for source ${source.id}:`, err)
        )
      )
    }

    return { ok: true, data: source }
  }
```

**Acceptance criteria:**
- `DerivationService` constructor accepts an optional `EmbeddingService` parameter.
- After bullet creation in `deriveBulletsFromSource`, `onBulletCreated` is called for each bullet via `queueMicrotask`.
- After perspective creation in `derivePerspectivesFromBullet`, `onPerspectiveCreated` is called via `queueMicrotask`.
- After JD creation in `JobDescriptionService.create`, `onJDCreated` is called with parsed requirements via `queueMicrotask`.
- After JD raw_text update in `JobDescriptionService.update`, `onJDUpdated` is called via `queueMicrotask`.
- After source creation in `SourceService.create`, `onSourceCreated` is called via `queueMicrotask`.
- All hook calls use `.catch()` to swallow errors. No embedding failure can crash the server or fail the HTTP response.
- Entity creation behavior is completely unaffected when `embeddingService` is undefined (backward compatible).

**Failure criteria:**
- Embedding hook throws an unhandled promise rejection.
- Entity creation fails or slows down due to embedding computation.
- The `.catch()` handler is missing, allowing unhandled rejections.
- Constructor change breaks existing test suites that don't provide an embedding service.

**Smoke test (G3):** Add one integration test that calls `DerivationService.deriveBulletsFromSource()` with a real `EmbeddingService`, awaits a microtask tick (`await new Promise(r => setTimeout(r, 100))`), and asserts that an embedding row exists for the created bullet.

---

### T69.7: Export from Services Index and Types

**File to modify:** `packages/core/src/services/index.ts`
**File to modify:** `packages/core/src/types/index.ts`

#### 69.7.1: Update types/index.ts

Add the following types at the end of the file, before any closing comments:

```typescript
// ── Embedding Types ──────────────────────────────────────────────────

/** Valid entity types for vector embeddings. */
export type EmbeddingEntityType = 'bullet' | 'perspective' | 'jd_requirement' | 'source'

/** A stored vector embedding for an entity. */
export interface Embedding {
  id: string
  entity_type: EmbeddingEntityType
  entity_id: string
  content_hash: string
  vector: Float32Array
  created_at: string
}

/** A parsed requirement from a job description. */
export interface ParsedRequirement {
  text: string
  confidence: number
  section: string | null
}

/** Result of parsing a job description into requirements. */
export interface ParsedRequirements {
  requirements: ParsedRequirement[]
  overall_confidence: number
}

/** An embedding that is stale (content changed since vector was computed). */
export interface StaleEmbedding {
  entity_type: EmbeddingEntityType
  entity_id: string
  stored_hash: string | null
  current_hash: string
}

/** A similar entity returned from a vector search. */
export interface SimilarEntity {
  entity_id: string
  similarity: number
}

// ── Alignment Types ──────────────────────────────────────────────────

/** Report from aligning a resume against a job description. */
export interface AlignmentReport {
  job_description_id: string
  resume_id: string
  /** Mean of best_match.similarity per requirement. Gaps contribute 0.0. */
  overall_score: number
  matches: Array<{
    requirement_text: string
    classification: 'strong' | 'adjacent' | 'gap'
    best_match: {
      entity_id: string
      similarity: number
    } | null
  }>
  strong_count: number
  adjacent_count: number
  gap_count: number
}

/** A single requirement match with its candidate entities. */
export interface RequirementMatch {
  requirement_text: string
  candidates: Array<{
    entity_id: string
    content: string
    similarity: number
  }>
}

/** An entity that exists in the inventory but was not matched to any requirement. */
export interface UnmatchedEntry {
  entity_id: string
  content: string
}

/** Report from matching JD requirements against entity inventory. */
export interface RequirementMatchReport {
  job_description_id: string
  matches: RequirementMatch[]
}
```

#### 69.7.2: Update services/index.ts

```typescript
import { EmbeddingService } from './embedding-service'

// Add to Services interface:
export interface Services {
  // ... existing services ...
  embeddings: EmbeddingService
}

// Update createServices:
export function createServices(db: Database, dbPath: string): Services {
  const derivingBullets = new Set<string>()
  const embeddingService = new EmbeddingService(db)

  const jobDescriptions = new JobDescriptionService(db)
  jobDescriptions.setEmbeddingService(embeddingService)

  const sources = new SourceService(db)
  sources.setEmbeddingService(embeddingService)

  return {
    sources,
    bullets: new BulletService(db),
    perspectives: new PerspectiveService(db),
    derivation: new DerivationService(db, derivingBullets, embeddingService),
    resumes: new ResumeService(db),
    audit: new AuditService(db),
    review: new ReviewService(db),
    organizations: new OrganizationService(db),
    notes: new NoteService(db),
    integrity: new IntegrityService(db),
    domains: new DomainService(db),
    archetypes: new ArchetypeService(db),
    profile: new ProfileService(db),
    jobDescriptions,
    templates: new TemplateService(db),
    export: new ExportService(db, dbPath),
    summaries: new SummaryService(db),
    embeddings: embeddingService,
  }
}

// Add re-export:
export { EmbeddingService } from './embedding-service'
```

**Acceptance criteria:**
- `EmbeddingService` is accessible via `services.embeddings` from any route handler or consumer.
- All new types (`Embedding`, `ParsedRequirements`, `StaleEmbedding`, `SimilarEntity`, `EmbeddingEntityType`, `AlignmentReport`, `RequirementMatch`, `UnmatchedEntry`, `RequirementMatchReport`) are exported from `@forge/core/types`.
- `EmbeddingEntityType` is defined ONLY in `types/index.ts` -- no duplicate definition in `embedding-repository.ts` (it imports from types).
- `createServices()` constructs the `EmbeddingService` and wires it into `DerivationService`, `JobDescriptionService`, and `SourceService`.
- The `EmbeddingService` re-export is available from `@forge/core/services`.
- Existing service tests still pass (no breaking constructor changes to non-embedding services).

**Failure criteria:**
- `Services` interface is missing the `embeddings` field.
- `createServices` does not pass `embeddingService` to `DerivationService`.
- `createServices` does not call `jobDescriptions.setEmbeddingService()`.
- `createServices` does not call `sources.setEmbeddingService()`.
- Import cycles between services/embedding-service and other service files.

---

### T69.8: Implement `alignResume()` and `matchRequirements()` Methods

**File to modify:** `packages/core/src/services/embedding-service.ts` (methods already defined in T69.4 above)
**File to modify:** `packages/core/src/services/__tests__/embedding-service.test.ts`

This task adds the `alignResume()` and `matchRequirements()` methods to `EmbeddingService`. The implementation is included in T69.4's code above for completeness, but this task covers the specific acceptance testing and verification of the alignment and matching logic.

#### 69.8.1: Method signatures

```typescript
alignResume(
  jdId: string,
  resumeId: string,
  opts?: { strongThreshold?: number; adjacentThreshold?: number },
): Promise<Result<AlignmentReport>>

matchRequirements(
  jdId: string,
  entityType: 'bullet' | 'perspective',
  opts?: { threshold?: number; limit?: number },
): Promise<Result<RequirementMatchReport>>
```

#### 69.8.2: `overall_score` formula

`overall_score` = mean of `best_match.similarity` per requirement, where requirements classified as `gap` (no match above `adjacentThreshold`) contribute **0.0** to the mean.

Example: 3 requirements with best similarities [0.85, 0.60, 0.20]. With `strongThreshold=0.75`, `adjacentThreshold=0.50`:
- Requirement 1: strong (0.85) -- contributes 0.85
- Requirement 2: adjacent (0.60) -- contributes 0.60
- Requirement 3: gap (0.20 < 0.50) -- contributes 0.0
- `overall_score` = (0.85 + 0.60 + 0.0) / 3 = 0.483

#### 69.8.3: Type definitions

These types are exported from `types/index.ts` (see T69.7):
- `AlignmentReport` — full alignment report with matches, classifications, and overall_score
- `RequirementMatch` — a single requirement with its candidate entities
- `UnmatchedEntry` — an inventory entity not matched to any requirement
- `RequirementMatchReport` — full report from `matchRequirements()`

#### 69.8.4: Additional tests

Add the following test cases to `embedding-service.test.ts`:

```typescript
describe('alignResume()', () => {
  it('returns an AlignmentReport with strong/adjacent/gap classifications', async () => {
    // Setup: seed JD requirements and resume perspective embeddings
    // Then call alignResume and verify the report structure
    // Verify overall_score formula (mean with gaps as 0.0)
  })

  it('returns error when no JD requirement embeddings exist', async () => {
    const result = await service.alignResume('nonexistent-jd', 'some-resume')
    expect(result.ok).toBe(false)
  })
})

describe('matchRequirements()', () => {
  it('returns candidates per requirement sorted by similarity', async () => {
    // Setup: seed JD requirements and bullet embeddings
    // Call matchRequirements and verify candidate ordering
  })

  it('respects threshold and limit parameters', async () => {
    // Verify filtering and limiting behavior
  })
})
```

**Acceptance criteria:**
- `alignResume()` returns an `AlignmentReport` with correct `strong_count`, `adjacent_count`, `gap_count`.
- `overall_score` is computed as the mean of `best_match.similarity` per requirement, with gaps contributing 0.0.
- `matchRequirements()` returns candidates sorted by descending similarity, filtered by threshold and limited.
- Both methods return `Result` with descriptive errors when embeddings are missing.
- Types `AlignmentReport`, `RequirementMatch`, `UnmatchedEntry`, `RequirementMatchReport` are exported from `types/index.ts`.

**Failure criteria:**
- `overall_score` formula does not correctly handle gaps (contributing 0.0).
- Candidates are not sorted by descending similarity.
- Method throws instead of returning a `Result` error.

---

## Documentation Tasks

### G2: Model Download Behavior Documentation

Create or update `dev-environment.md` (or equivalent) to document:
- First-use model download (~80MB from Hugging Face Hub)
- Cache location: `~/.cache/huggingface/hub` (or `HF_HOME` if set)
- Offline pre-download command: `npx @huggingface/transformers download Xenova/all-MiniLM-L6-v2`
- `resetPipeline()` is for testing only -- forces model reload on next call
- CI caching: cache `~/.cache/huggingface/` between CI runs, or set `SKIP_MODEL_TESTS=1` to skip model-dependent tests

---

## Acceptance Criteria (Phase-Level)

1. `bun test` passes for all new test files:
   - `packages/core/src/lib/__tests__/model-loader.test.ts`
   - `packages/core/src/lib/__tests__/jd-parser.test.ts`
   - `packages/core/src/db/repositories/__tests__/embedding-repository.test.ts`
   - `packages/core/src/services/__tests__/embedding-service.test.ts`
2. Migration N+1 (from T69.0 audit) applies cleanly on a fresh database and on an existing database with all prior migrations.
3. `EmbeddingService.embed()` produces a 384-dim vector, hashes the content, and stores it in the `embeddings` table.
4. `EmbeddingService.findSimilar()` returns semantically related entities sorted by descending cosine similarity.
5. `EmbeddingService.checkStale()` identifies embeddings whose content has diverged from the source-of-truth tables, including JD requirement staleness.
6. `EmbeddingService.refreshStale()` recomputes all stale embeddings and removes orphans.
7. `EmbeddingService.alignResume()` returns an `AlignmentReport` with correct classifications and `overall_score`.
8. `EmbeddingService.matchRequirements()` returns a `RequirementMatchReport` with candidates per requirement.
9. `parseRequirements()` extracts individual requirements from structured JD text with confidence >= 0.9 (0.7 for responsibility sections), and from prose with confidence < 0.5.
10. Fire-and-forget hooks are wired into `DerivationService`, `JobDescriptionService`, and `SourceService`. Embedding failures are caught and logged, never propagated.
11. `services.embeddings` is accessible from the services object returned by `createServices()`.
12. All existing tests continue to pass (no regressions from constructor changes or new imports).
13. `@huggingface/transformers` (v3+) is the installed package, not the superseded `@xenova/transformers`.
14. `EmbeddingEntityType` is defined exactly once in `types/index.ts`.
