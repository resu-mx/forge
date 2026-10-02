# Phase 0: Storage Adapter Interface Design

**Status:** Draft
**Created:** 2026-04-09
**Depends on:** Nothing (foundational)
**Informs:** Phases 1-10 (all subsequent storage work)

## Goal

Design and implement the storage abstraction boundary for Forge: a thin **Storage Adapter** for raw CRUD, an **Entity Lifecycle Manager** (integrity layer) for constraint enforcement and lifecycle hooks, and a **Relationship Map** that drives both. Ship with SQLite as the only configured backend — zero behavior change, pure refactor.

## Architecture

```
┌─────────────────────────────────┐
│         Services (26)           │
│  Status FSM, computed values,   │
│  business orchestration         │
├─────────────────────────────────┤
│   Entity Lifecycle Manager      │
│  Constraints: FK, unique, enum  │
│  Hooks: before/after CRUD       │
│  Cascade / Restrict / SetNull   │
│  Relationship Map (declarative) │
├─────────────────────────────────┤
│      Storage Adapter (thin)     │
│  create, get, list, update,     │
│  delete, deleteWhere, count,    │
│  updateWhere, transaction       │
│  + named queries (10-15)        │
├──────┬──────────┬───────────────┤
│SQLite│ HelixDB  │ GraphQLite    │
│(now) │ (future) │ (future)      │
└──────┴──────────┴───────────────┘
```

### Design Principles

1. **Adapter is dumb.** It knows how to read/write entities in its native format. No relational awareness, no business logic, no constraint enforcement.
2. **Integrity layer is smart.** It knows the relationship graph, enforces all constraints, manages entity lifecycle. Backend-agnostic.
3. **Services stay focused.** Business rules, status machines, orchestration. They call through the integrity layer, never directly to the adapter.
4. **SQLite double-cascades are fine.** The integrity layer cascades explicitly. SQLite's native FK cascades find nothing left to do. Harmless redundancy.
5. **Relationship map is hand-maintained.** Validated by a test that introspects the actual SQLite schema via PRAGMA. Auto-generation is a future enhancement.

---

## 1. Storage Adapter Interface

The adapter is the lowest layer. It speaks in entities (type + id + data), not domain objects. Every method operates on a single entity type.

### 1.1 Core CRUD

```typescript
interface StorageAdapter {
  // --- Single entity operations ---
  create(entityType: string, data: Record<string, unknown>): Promise<{ id: string }>
  get(entityType: string, id: string): Promise<Record<string, unknown> | null>
  update(entityType: string, id: string, data: Record<string, unknown>): Promise<void>
  delete(entityType: string, id: string): Promise<void>

  // --- Bulk operations ---
  list(entityType: string, opts?: ListOptions): Promise<ListResult>
  count(entityType: string, where?: WhereClause): Promise<number>
  deleteWhere(entityType: string, where: WhereClause): Promise<number>  // returns deleted count
  updateWhere(entityType: string, where: WhereClause, data: Record<string, unknown>): Promise<number>

  // --- Transactions ---
  beginTransaction(): Promise<Transaction>

  // --- Named queries (complex, backend-optimized) ---
  executeNamedQuery(name: string, params: Record<string, unknown>): Promise<unknown>

  // --- Lifecycle ---
  connect(): Promise<void>
  disconnect(): Promise<void>
  isConnected(): boolean
}

interface Transaction {
  create(entityType: string, data: Record<string, unknown>): Promise<{ id: string }>
  get(entityType: string, id: string): Promise<Record<string, unknown> | null>
  update(entityType: string, id: string, data: Record<string, unknown>): Promise<void>
  delete(entityType: string, id: string): Promise<void>
  deleteWhere(entityType: string, where: WhereClause): Promise<number>
  updateWhere(entityType: string, where: WhereClause, data: Record<string, unknown>): Promise<number>
  count(entityType: string, where?: WhereClause): Promise<number>
  commit(): Promise<void>
  rollback(): Promise<void>
}

interface ListOptions {
  where?: WhereClause
  orderBy?: { field: string; direction: 'asc' | 'desc' }[]
  limit?: number
  offset?: number
}

interface ListResult {
  rows: Record<string, unknown>[]
  total: number
}

// WhereClause supports basic filtering — not a full query language.
// Complex queries go through named queries.
// Full definition in Section 6.
type WhereClause = SimpleWhere | CompoundWhere
```

### 1.2 Named Queries

Named queries handle the ~10-15 complex query patterns that can't be expressed as simple CRUD. Each backend implements them using its native strengths. The contract is **"return this shape"** not **"use this mechanism."**

```typescript
interface NamedQueryRegistry {
  // Each named query has a typed input and output
  'traceChain': {
    params: { perspectiveId: string }
    result: { perspective: Perspective, bullet: Bullet, sources: Source[] }
  }
  'getWithEntries': {
    params: { resumeId: string }
    result: { resume: Resume, sections: (Section & { entries: Entry[] })[] }
  }
  'findSimilar': {
    params: { query: string, entityType: string, topK: number, threshold: number }
    result: { entityId: string, score: number }[]
  }
  'listBulletsFiltered': {
    params: { status?: string, sourceId?: string, skillId?: string, limit?: number, offset?: number }
    result: ListResult
  }
  'listPerspectivesFiltered': {
    params: { bulletId?: string, sourceId?: string, archetype?: string, domain?: string, status?: string }
    result: ListResult
  }
  'reviewPending': {
    params: {}
    result: { bullets: BulletWithChain[], perspectives: PerspectiveWithChain[] }
  }
  'resumeCompilerIR': {
    params: { resumeId: string }
    result: ResumeDocument  // the IR format used by markdown/latex renderers
  }
  'alignmentScore': {
    params: { jdId: string, resumeId: string }
    result: AlignmentReport
  }
  'matchRequirements': {
    params: { jdId: string, entityType: string, topK: number }
    result: RequirementMatch[]
  }
  'checkStale': {
    params: {}
    result: { stale: StaleEntity[], orphans: OrphanEntity[] }
  }
  'skillCoverage': {
    params: { resumeId: string, jdId: string }
    result: { matching: Skill[], missing: Skill[], extra: Skill[] }
  }
  'sourceExtension': {
    params: { sourceId: string, sourceType: string }
    result: Record<string, unknown>  // polymorphic extension data
  }
}
```

The SQLite adapter implements these as the existing JOIN/subquery patterns from the current repositories. Future adapters implement them as graph traversals, Cypher queries, or HQL queries.

### 1.3 Optional Sub-Interfaces

Not all backends support all capabilities. These are separate interfaces that an adapter may or may not implement.

```typescript
interface GraphCapableAdapter extends StorageAdapter {
  traverse(from: { entityType: string, id: string }, edgeType: string, depth?: number): Promise<TraversalResult>
  shortestPath(from: EntityRef, to: EntityRef): Promise<PathResult | null>
  subgraph(rootType: string, rootId: string, depth: number): Promise<SubgraphResult>
}

interface VectorCapableAdapter extends StorageAdapter {
  embed(entityType: string, entityId: string, content: string): Promise<void>
  findSimilar(query: string, entityType: string, topK: number, threshold: number): Promise<SimilarityResult[]>
  checkEmbeddingStale(entityType: string, entityId: string, contentHash: string): Promise<boolean>
}

// Capability detection
interface AdapterCapabilities {
  graph: boolean
  vector: boolean
  transactions: boolean
  nativeCascade: boolean  // hint: integrity layer can skip cascade if adapter handles it
}
```

---

## 2. Entity Lifecycle Manager

The integrity layer. All service writes go through this. It wraps the adapter with constraint enforcement and lifecycle hooks.

### 2.1 Interface

```typescript
interface EntityLifecycleManager {
  // --- CRUD (delegates to adapter after constraint checks + hooks) ---
  create(entityType: string, data: Record<string, unknown>): Promise<Result<{ id: string }>>
  get(entityType: string, id: string, opts?: ReadOptions): Promise<Result<Record<string, unknown>>>
  list(entityType: string, opts?: ListOptions & ReadOptions): Promise<Result<ListResult>>
  update(entityType: string, id: string, data: Record<string, unknown>): Promise<Result<void>>
  delete(entityType: string, id: string): Promise<Result<void>>

  // --- Bulk operations (constraint-aware) ---
  // Used heavily for junction table management (see Section 8.3).
  // These still run constraint checks and lifecycle hooks per affected row.
  count(entityType: string, where?: WhereClause): Promise<Result<number>>
  deleteWhere(entityType: string, where: WhereClause): Promise<Result<number>>
  updateWhere(entityType: string, where: WhereClause, data: Record<string, unknown>): Promise<Result<number>>

  // --- Named queries (pass-through to adapter) ---
  query<T extends keyof NamedQueryRegistry>(
    name: T,
    params: NamedQueryRegistry[T]['params']
  ): Promise<Result<NamedQueryRegistry[T]['result']>>

  // --- Transaction scope ---
  transaction<T>(fn: (tx: TransactionScope) => Promise<T>): Promise<Result<T>>

  // --- Adapter access (escape hatch for services that need direct adapter) ---
  readonly adapter: StorageAdapter
  readonly capabilities: AdapterCapabilities
}

// Opt-in flags for reads (lazy field materialization — see Section 8.1)
interface ReadOptions {
  includeLazy?: boolean | string[]  // true = all lazy, string[] = named fields
}

// TransactionScope mirrors EntityLifecycleManager but operates within a transaction
interface TransactionScope {
  create(entityType: string, data: Record<string, unknown>): Promise<{ id: string }>
  get(entityType: string, id: string, opts?: ReadOptions): Promise<Record<string, unknown> | null>
  update(entityType: string, id: string, data: Record<string, unknown>): Promise<void>
  delete(entityType: string, id: string): Promise<void>
  deleteWhere(entityType: string, where: WhereClause): Promise<number>
  updateWhere(entityType: string, where: WhereClause, data: Record<string, unknown>): Promise<number>
  count(entityType: string, where?: WhereClause): Promise<number>
}
```

### 2.2 Create Flow

```
service calls: elm.create('source_roles', { source_id, organization_id, ... })

1. APPLY DEFAULTS
   → for each field in entity map with a `default` value:
     → if data[field] is undefined: set data[field] = default (or default() if function)
   → e.g., created_at = isoNow(), status = 'draft', source_type = 'general'

2. VALIDATE required fields
   → for each field with required: true:
     → if data[field] is null or undefined: REQUIRED_VIOLATION
   → e.g., source_roles.source_id must be present

3. VALIDATE field types & custom checks
   → for each field in data:
     → type matches FieldDefinition.type (string for text, etc.)
     → if field has `check` function: run it, reject on false
     → unknown fields (not in entity map): VALIDATION_ERROR

4. VALIDATE enum constraints
   → for each field with `enum` defined:
     → if data[field] not in enum: ENUM_VIOLATION
   → e.g., sources.source_type must be in ['role','project','education','general','presentation']
   → (source_roles has no enum fields — step is a noop for this entity)

5. VALIDATE FK existence
   → for each field with `fk` defined and data[field] provided:
     → adapter.get(fk.entity, data[field]) must return non-null, else FK_VIOLATION
     → e.g., source_roles.source_id → sources table
     → e.g., source_roles.organization_id → organizations (nullable: only check if provided)

6. VALIDATE uniqueness
   → for each field with `unique: true`:
     → adapter.count(entityType, { [field]: data[field] }) must be 0, else UNIQUE_VIOLATION
   → (source_roles has no unique fields — step is a noop for this entity)

7. RUN beforeCreate hooks
   → registered hooks for 'source_roles' execute in registration order
   → hooks receive HookContext with mutable data
   → if a hook throws: reject with VALIDATION_ERROR, do not proceed

8. DELEGATE to adapter
   → adapter.create('source_roles', data) → { id }

9. RUN afterCreate hooks
   → registered hooks execute in registration order
   → hooks receive HookContext with id set
   → hook errors are LOGGED but do not reject the operation (side effects are fire-and-forget)

10. RETURN Result<{ id }>
```

### 2.3 Delete Flow

```
service calls: elm.delete('skills', id)

1. CHECK restrict rules
   → relationship map: skills has no restrict rules
   → (if it did: adapter.count(restrictedTable, { fk: id }) — reject if > 0)

2. RUN beforeDelete hooks
   → registered hooks for 'skills' (if any)

3. BEGIN transaction

4. CASCADE deletes (recursive, depth-first)
   → relationship map: skills cascades to:
     bullet_skills (skill_id), source_skills (skill_id), jd_skills (skill_id),
     resume_skills (skill_id), skill_domains (skill_id), summary_skills (skill_id),
     certification_skills (skill_id), perspective_skills (skill_id)
   → for each: tx.deleteWhere(table, { skill_id: id })
   → for each cascaded table: check if IT has cascade rules, recurse if so
     (none of these junctions have further cascades)

5. SET NULL updates
   → relationship map: skills has no setNull rules
   → (if it did: tx.updateWhere(table, { fk: id }, { fk: null }))

6. DELETE entity
   → tx.delete('skills', id)

7. COMMIT transaction

8. RUN afterDelete hooks
   → registered hooks for 'skills' (if any)

9. RETURN Result<void>
```

### 2.4 Update Flow

```
service calls: elm.update('organizations', id, { industry_id, name, ... })

1. VALIDATE entity exists
   → adapter.get('organizations', id) — must exist, else NOT_FOUND

2. VALIDATE constraints on changed fields
   → if industry_id changed: FK check against industries
   → if name changed: uniqueness check (if applicable — orgs don't have unique name)
   → enum checks on org_type, employment_type, status (if provided)

3. RUN beforeUpdate hooks
   → e.g., set updated_at = now()

4. DELEGATE to adapter
   → adapter.update('organizations', id, data)

5. RUN afterUpdate hooks
   → e.g., re-embed if content field changed

6. RETURN Result<void>
```

---

## 3. Relationship Map

The declarative schema that drives the integrity layer. Hand-maintained, validated by tests.

### 3.1 Structure

```typescript
interface EntityDefinition {
  // --- Fields ---
  fields: Record<string, FieldDefinition>

  // --- Relationships (for delete coordination) ---
  cascade: CascadeRule[]
  restrict: RestrictRule[]
  setNull: SetNullRule[]

  // --- Lifecycle hooks ---
  hooks?: {
    beforeCreate?: HookFn[]
    afterCreate?: HookFn[]
    beforeUpdate?: HookFn[]
    afterUpdate?: HookFn[]
    beforeDelete?: HookFn[]
    afterDelete?: HookFn[]
  }
}

interface FieldDefinition {
  type: 'text' | 'integer' | 'real' | 'blob' | 'json'
  required?: boolean         // NOT NULL
  unique?: boolean           // UNIQUE constraint
  default?: unknown          // DEFAULT value (or () => unknown for computed)
  fk?: { entity: string, field: string, nullable?: boolean }  // FK reference
  enum?: string[] | (() => string[])  // allowed values
  check?: (value: unknown) => boolean  // custom validation
  boolean?: boolean          // flag integer field as boolean (SQLite has no bool type — see Section 8.1)
  lazy?: boolean             // don't deserialize unless explicitly requested (vectors, large JSON — see Section 8.1)
}

interface CascadeRule {
  entity: string   // target table
  field: string    // FK field on target that references this entity
}

interface RestrictRule {
  entity: string   // table that blocks deletion
  field: string    // FK field on that table
  message?: string // human-readable error ("Cannot delete bullet: has N perspectives")
}

interface SetNullRule {
  entity: string   // target table
  field: string    // FK field to null out
}

type HookFn = (ctx: HookContext) => Promise<void>

interface HookContext {
  entityType: string
  id?: string                           // null on create
  data: Record<string, unknown>         // entity data (mutable on before* hooks)
  adapter: StorageAdapter               // for lookups in hooks (e.g., snapshot capture)
  previous?: Record<string, unknown>    // previous state (on update/delete)
}
```

### 3.2 Example Entries

```typescript
const ENTITY_MAP: Record<string, EntityDefinition> = {

  sources: {
    fields: {
      id:             { type: 'text', required: true },
      title:          { type: 'text', required: true },
      description:    { type: 'text', required: true },
      source_type:    { type: 'text', required: true, default: 'general',
                        enum: ['role', 'project', 'education', 'general', 'presentation'] },
      status:         { type: 'text', required: true, default: 'draft',
                        enum: ['draft', 'in_review', 'approved', 'rejected', 'archived', 'deriving'] },
      updated_by:     { type: 'text', required: true, default: 'human', enum: ['human', 'ai'] },
      start_date:     { type: 'text' },
      end_date:       { type: 'text' },
      last_derived_at:{ type: 'text' },
      notes:          { type: 'text' },
      created_at:     { type: 'text', required: true, default: () => isoNow() },
      updated_at:     { type: 'text', required: true, default: () => isoNow() },
    },
    cascade: [
      { entity: 'source_roles', field: 'source_id' },
      { entity: 'source_projects', field: 'source_id' },
      { entity: 'source_education', field: 'source_id' },
      { entity: 'source_presentations', field: 'source_id' },
      { entity: 'bullet_sources', field: 'source_id' },
      { entity: 'source_skills', field: 'source_id' },
    ],
    restrict: [],
    setNull: [
      { entity: 'resume_entries', field: 'source_id' },
    ],
    hooks: {
      beforeUpdate: [setUpdatedAt],
      afterCreate: [fireAndForgetEmbed('source')],
    },
  },

  bullets: {
    fields: {
      id:                       { type: 'text', required: true },
      content:                  { type: 'text', required: true },
      source_content_snapshot:  { type: 'text', required: true },
      metrics:                  { type: 'text' },
      status:                   { type: 'text', required: true, default: 'in_review',
                                  enum: ['draft', 'in_review', 'approved', 'rejected', 'archived'] },
      rejection_reason:         { type: 'text' },
      prompt_log_id:            { type: 'text', fk: { entity: 'prompt_logs', field: 'id', nullable: true } },
      approved_at:              { type: 'text' },
      approved_by:              { type: 'text' },
      notes:                    { type: 'text' },
      domain:                   { type: 'text' },
      created_at:               { type: 'text', required: true, default: () => isoNow() },
    },
    cascade: [
      { entity: 'bullet_sources', field: 'bullet_id' },
      { entity: 'bullet_skills', field: 'bullet_id' },
    ],
    restrict: [
      { entity: 'perspectives', field: 'bullet_id',
        message: 'Cannot delete bullet: has {count} perspective(s)' },
    ],
    setNull: [],
    hooks: {
      afterCreate: [fireAndForgetEmbed('bullet')],
    },
  },

  perspectives: {
    fields: {
      id:                       { type: 'text', required: true },
      bullet_id:                { type: 'text', required: true,
                                  fk: { entity: 'bullets', field: 'id' } },
      content:                  { type: 'text', required: true },
      bullet_content_snapshot:  { type: 'text', required: true },
      target_archetype:         { type: 'text' },
      domain:                   { type: 'text' },
      framing:                  { type: 'text', required: true,
                                  enum: ['accomplishment', 'responsibility', 'context'] },
      status:                   { type: 'text', required: true, default: 'in_review',
                                  enum: ['draft', 'in_review', 'approved', 'rejected', 'archived'] },
      rejection_reason:         { type: 'text' },
      prompt_log_id:            { type: 'text', fk: { entity: 'prompt_logs', field: 'id', nullable: true } },
      approved_at:              { type: 'text' },
      approved_by:              { type: 'text' },
      notes:                    { type: 'text' },
      created_at:               { type: 'text', required: true, default: () => isoNow() },
    },
    cascade: [
      { entity: 'perspective_skills', field: 'perspective_id' },
    ],
    restrict: [
      { entity: 'resume_entries', field: 'perspective_id',
        message: 'Cannot delete perspective: used in {count} resume(s)' },
    ],
    setNull: [],
    hooks: {
      afterCreate: [fireAndForgetEmbed('perspective')],
    },
  },

  resume_entries: {
    fields: {
      id:                            { type: 'text', required: true },
      resume_id:                     { type: 'text', required: true,
                                       fk: { entity: 'resumes', field: 'id' } },
      section_id:                    { type: 'text', required: true,
                                       fk: { entity: 'resume_sections', field: 'id' } },
      perspective_id:                { type: 'text',
                                       fk: { entity: 'perspectives', field: 'id', nullable: true } },
      source_id:                     { type: 'text',
                                       fk: { entity: 'sources', field: 'id', nullable: true } },
      content:                       { type: 'text' },
      perspective_content_snapshot:  { type: 'text' },
      position:                      { type: 'integer', required: true, default: 0 },
      notes:                         { type: 'text' },
      created_at:                    { type: 'text', required: true, default: () => isoNow() },
      updated_at:                    { type: 'text', required: true, default: () => isoNow() },
    },
    cascade: [],
    restrict: [],
    setNull: [],
    hooks: {
      beforeCreate: [captureSnapshotHook],
      beforeUpdate: [setUpdatedAt, captureSnapshotHook],
    },
  },

  // --- Junction table example (no hooks, cascade-only) ---
  bullet_skills: {
    fields: {
      bullet_id: { type: 'text', required: true, fk: { entity: 'bullets', field: 'id' } },
      skill_id:  { type: 'text', required: true, fk: { entity: 'skills', field: 'id' } },
    },
    cascade: [],
    restrict: [],
    setNull: [],
  },

  // ... remaining 42 entities follow same pattern
}
```

### 3.3 Common Hook Implementations

```typescript
// Set updated_at timestamp
const setUpdatedAt: HookFn = async (ctx) => {
  ctx.data.updated_at = isoNow()
}

// Capture perspective content snapshot on resume_entry create/update
const captureSnapshotHook: HookFn = async (ctx) => {
  if (ctx.data.perspective_id && !ctx.data.content) {
    const perspective = await ctx.adapter.get('perspectives', ctx.data.perspective_id as string)
    if (perspective) {
      ctx.data.perspective_content_snapshot = perspective.content
    }
  }
}

// Fire-and-forget embedding after entity creation
function fireAndForgetEmbed(entityType: string): HookFn {
  return async (ctx) => {
    if (ctx.id) {
      queueMicrotask(() => {
        embeddingService.embed(entityType, ctx.id!, ctx.data.content as string)
          .catch(err => console.error(`Embedding failed for ${entityType}/${ctx.id}:`, err))
      })
    }
  }
}

// Generate ISO timestamp
function isoNow(): string {
  return new Date().toISOString().replace(/\.\d{3}Z$/, 'Z')
}
```

### 3.4 Schema Validation Test

A test that keeps the relationship map honest:

```typescript
test('relationship map matches SQLite schema', () => {
  for (const [entityType, def] of Object.entries(ENTITY_MAP)) {
    // Check table exists
    const columns = db.pragma(`table_info(${entityType})`)
    expect(columns.length).toBeGreaterThan(0)

    // Check FK rules match
    const fks = db.pragma(`foreign_key_list(${entityType})`)
    for (const fk of fks) {
      const fieldDef = def.fields[fk.from]
      expect(fieldDef?.fk?.entity).toBe(fk.table)
      expect(fieldDef?.fk?.field).toBe(fk.to)
    }

    // Check cascade/restrict/setNull rules have corresponding FKs
    for (const rule of def.cascade) {
      const targetFks = db.pragma(`foreign_key_list(${rule.entity})`)
      const match = targetFks.find(fk => fk.from === rule.field && fk.table === entityType)
      expect(match).toBeDefined()
      expect(match.on_delete).toBe('CASCADE')
    }

    for (const rule of def.restrict) {
      const targetFks = db.pragma(`foreign_key_list(${rule.entity})`)
      const match = targetFks.find(fk => fk.from === rule.field && fk.table === entityType)
      expect(match).toBeDefined()
      expect(match.on_delete).toBe('RESTRICT')
    }

    for (const rule of def.setNull) {
      const targetFks = db.pragma(`foreign_key_list(${rule.entity})`)
      const match = targetFks.find(fk => fk.from === rule.field && fk.table === entityType)
      expect(match).toBeDefined()
      expect(match.on_delete).toBe('SET NULL')
    }
  }
})
```

---

## 4. SQLite Adapter (Phase 0 Implementation)

The first and only adapter for Phase 0. Wraps the existing `better-sqlite3` database connection.

### 4.1 Implementation Strategy

The SQLite adapter translates generic CRUD operations into SQL statements. It does NOT reuse existing repository classes — it replaces them with generic SQL generation.

```typescript
class SqliteAdapter implements StorageAdapter {
  constructor(private db: Database) {}

  async create(entityType: string, data: Record<string, unknown>): Promise<{ id: string }> {
    const id = data.id as string ?? generateUUID()
    const fields = Object.keys(data)
    const placeholders = fields.map(() => '?')
    const sql = `INSERT INTO ${entityType} (${fields.join(', ')}) VALUES (${placeholders.join(', ')})`
    this.db.prepare(sql).run(...fields.map(f => serialize(data[f])))
    return { id }
  }

  async get(entityType: string, id: string): Promise<Record<string, unknown> | null> {
    const row = this.db.prepare(`SELECT * FROM ${entityType} WHERE id = ?`).get(id)
    return row ?? null  // raw row; integrity layer deserializes using entity map
  }

  async list(entityType: string, opts?: ListOptions): Promise<ListResult> {
    // Use separate param arrays to avoid cryptic slicing for count query
    const whereParams: unknown[] = []
    const pageParams: unknown[] = []

    let sql = `SELECT * FROM ${entityType}`
    let countSql = `SELECT COUNT(*) as total FROM ${entityType}`

    if (opts?.where) {
      const { clause, values } = buildWhereClause(opts.where)
      sql += ` WHERE ${clause}`
      countSql += ` WHERE ${clause}`
      whereParams.push(...values)
    }

    if (opts?.orderBy?.length) {
      sql += ` ORDER BY ${opts.orderBy.map(o => `${o.field} ${o.direction}`).join(', ')}`
    }

    if (opts?.limit) {
      sql += ` LIMIT ?`
      pageParams.push(opts.limit)
      if (opts?.offset) {
        sql += ` OFFSET ?`
        pageParams.push(opts.offset)
      }
    }

    const rows = this.db.prepare(sql).all(...whereParams, ...pageParams) as Record<string, unknown>[]
    const { total } = this.db.prepare(countSql).get(...whereParams) as { total: number }
    return { rows, total }  // raw rows; integrity layer deserializes
  }

  // ... update, delete, deleteWhere, updateWhere, count follow same pattern

  async beginTransaction(): Promise<Transaction> {
    return new SqliteTransaction(this.db)
  }

  async executeNamedQuery(name: string, params: Record<string, unknown>): Promise<unknown> {
    const handler = this.namedQueries[name]
    if (!handler) throw new Error(`Unknown named query: ${name}`)
    return handler(this.db, params)
  }
}
```

### 4.2 Named Query Implementations

Each named query is the existing repository JOIN logic, extracted into a standalone function:

```typescript
// Maps named query names to SQLite-specific implementations
const SQLITE_NAMED_QUERIES: Record<string, (db: Database, params: any) => any> = {

  traceChain: (db, { perspectiveId }) => {
    // Current: PerspectiveRepository.getWithChain()
    // 3-table JOIN: perspectives → bullets → bullet_sources → sources
    const sql = `
      SELECT p.*, b.*, s.*
      FROM perspectives p
      JOIN bullets b ON p.bullet_id = b.id
      LEFT JOIN bullet_sources bs ON b.id = bs.bullet_id
      LEFT JOIN sources s ON bs.source_id = s.id
      WHERE p.id = ?
    `
    // ... hydrate nested objects
  },

  getWithEntries: (db, { resumeId }) => {
    // Current: ResumeRepository.getWithEntries()
    // Multi-table JOIN: resume_entries → resume_sections LEFT JOIN perspectives
    // ... existing implementation
  },

  resumeCompilerIR: (db, { resumeId }) => {
    // Current: compileResumeIR()
    // 6+ table JOIN for full resume intermediate representation
    // ... existing implementation
  },

  // ... remaining 8-12 named queries
}
```

### 4.3 Data Serialization

Per the decision in Section 8.1, the SQLite adapter serializes canonical types to SQLite storage on WRITE, but returns raw storage values on READ. The integrity layer handles deserialization using the entity map.

```typescript
// WRITE: canonical -> SQLite storage
function serialize(value: unknown): unknown {
  if (typeof value === 'boolean') return value ? 1 : 0
  if (value instanceof Float32Array || value instanceof Uint8Array) return Buffer.from(value.buffer)
  if (typeof value === 'object' && value !== null && !(value instanceof Buffer)) return JSON.stringify(value)
  return value
}

// READ: return raw SQLite values unchanged
// The integrity layer is responsible for deserialization using the entity map:
//   - INTEGER with boolean:true flag -> boolean
//   - TEXT with type:'json' -> parsed object
//   - BLOB with type:'blob' -> Uint8Array (or Float32Array for embedding vectors)
// This keeps the adapter maximally simple and puts all type awareness in one place.
```

The adapter never calls `deserializeRow` — it returns rows exactly as `better-sqlite3` produces them. Type conversion is handled by `EntityLifecycleManager` in a single pass driven by field definitions. This matches Section 8.1's resolved decision.

---

## 5. Migration Path

### 5.1 File Structure

```
packages/core/src/
  storage/
    adapter.ts              # StorageAdapter + Transaction interfaces
    adapter-types.ts        # WhereClause, ListOptions, ListResult, ReadOptions,
                            #   StorageError, AdapterCapabilities (adapter-facing)
    entity-types.ts         # Bullet, Perspective, ResumeDocument, AlignmentReport,
                            #   etc. (domain-facing; see Section 8.4)
    entity-map.ts           # buildEntityMap(deps), ENTITY_MAP_SHAPE,
                            #   EntityDefinition, FieldDefinition interfaces
    entity-map.test.ts      # PRAGMA validation test
    named-queries.ts        # NamedQueryRegistry type definitions
    lifecycle-manager.ts    # EntityLifecycleManager implementation
    lifecycle-manager.test.ts
    errors.ts               # StorageErrorCode, Result<T> envelope
    hooks/
      common.ts             # setUpdatedAt, captureSnapshot, createEmbedHook
    adapters/
      sqlite-adapter.ts     # SqliteAdapter implementation
      sqlite-adapter.test.ts
      sqlite-named-queries.ts  # SQLite implementations of named queries
      sqlite-transaction.ts # SqliteTransaction implementation
```

**Naming rationale:** `adapter-types.ts` contains the thin-adapter interface types (what the adapter speaks). `entity-types.ts` contains the domain types consumers work with (inferred from the entity map for simple entities, hand-written for composed query results). Splitting prevents confusion about "which types file."

### 5.2 Incremental Rewiring (Phase 0 → Phase 1)

Phase 0 delivers the interfaces, entity map, integrity layer, SQLite adapter, and tests. Phase 1 rewires services to use the integrity layer instead of raw repositories. This can be done **one service at a time**:

1. Start with a simple service (e.g., `domain-service`) — few methods, clear CRUD
2. Replace `this.repos.domains.create(...)` with `this.elm.create('domains', ...)`
3. Remove domain-specific validation that the integrity layer now handles
4. Verify all existing tests pass
5. Move to next service
6. After all services are rewired, deprecate and remove old repository classes

### 5.3 What Doesn't Change

- Route handlers (still call services)
- MCP tools (still call SDK → API → routes → services)
- SDK (still HTTP client)
- WebUI (still calls API)
- Database file (still forge.db, still SQLite)
- Migration system (still SQL migration files)
- Test database setup (still uses migration runner)

---

## 6. WhereClause Design

The WhereClause needs to be expressive enough for the integrity layer's needs (FK checks, cascade deletes) and basic service queries, without becoming a full query language.

```typescript
type WhereClause = SimpleWhere | CompoundWhere

// Equality: { status: 'draft', source_type: 'role' }
type SimpleWhere = Record<string, WhereValue>

type WhereValue =
  | string | number | boolean | null      // equality: field = value
  | { $in: (string | number)[] }          // field IN (...)
  | { $ne: string | number | null }       // field != value
  | { $like: string }                     // field LIKE pattern
  | { $gt: number }                       // field > value
  | { $gte: number }                      // field >= value
  | { $lt: number }                       // field < value
  | { $lte: number }                      // field <= value
  | { $isNull: true }                     // field IS NULL
  | { $isNotNull: true }                  // field IS NOT NULL

// Compound (for complex filters — rarely needed)
type CompoundWhere = {
  $and?: WhereClause[]
  $or?: WhereClause[]
}
```

---

## 7. Error Types

```typescript
type StorageErrorCode =
  | 'NOT_FOUND'           // entity doesn't exist
  | 'FK_VIOLATION'        // referenced entity doesn't exist
  | 'UNIQUE_VIOLATION'    // duplicate value on unique field
  | 'RESTRICT_VIOLATION'  // can't delete, children exist
  | 'ENUM_VIOLATION'      // value not in allowed set
  | 'REQUIRED_VIOLATION'  // required field missing/null
  | 'VALIDATION_ERROR'    // custom check failed
  | 'TRANSACTION_ERROR'   // transaction failed
  | 'ADAPTER_ERROR'       // backend-specific error

interface StorageError {
  code: StorageErrorCode
  message: string
  entityType?: string
  field?: string
  details?: Record<string, unknown>  // e.g., { count: 3 } for restrict violations
}
```

---

## 8. Resolved Design Decisions

### 8.1 Deserialization Timing

**Decision:** Adapter returns raw backend values. Integrity layer deserializes using the entity map.

The adapter stays maximally simple. The integrity layer already iterates over entity map field definitions on every read (for relationship awareness) — deserialization is one more step in the same loop. This keeps adapter surface area minimal, which matters most for HelixDB and GraphQLite implementations.

**Serialization contract:**
- Adapter WRITES: accepts canonical types, serializes to backend format internally
- Adapter READS: returns raw backend values
- Integrity layer READS: deserializes raw values to canonical types using entity map

**Canonical type mapping:**

| Entity map type | Canonical JS type | SQLite storage |
|---|---|---|
| `text` | `string` | `TEXT` |
| `integer` | `number` | `INTEGER` |
| `real` | `number` | `REAL` |
| `blob` | `Uint8Array \| Float32Array` | `BLOB` |
| `json` | `object` | `TEXT` (JSON string) |
| `boolean` (stored as integer) | `boolean` | `INTEGER` (0/1) |

Boolean fields are flagged explicitly in the entity map since SQLite has no native boolean type:

```typescript
fields: {
  in_progress: { type: 'integer', boolean: true, default: false },
  is_current:  { type: 'integer', boolean: true, default: false },
}
```

**Lazy deserialization for expensive fields:** Vector blobs and large JSON payloads can be marked `lazy: true`. The integrity layer returns a placeholder (or omits the field entirely) unless the caller opts in:

```typescript
fields: {
  vector: { type: 'blob', lazy: true },
}

// Caller explicitly requests lazy fields:
elm.get('embeddings', id, { includeLazy: ['vector'] })
elm.list('embeddings', { includeLazy: true })  // all lazy fields
```

Lazy is a Phase 0 feature because embeddings are already used heavily and the cost of deserializing a 1536-byte Float32Array on every read adds up for alignment queries that touch thousands of rows.

### 8.2 Hook Dependency Injection

**Decision:** Entity map is built by a factory function at app startup. Hooks close over injected services.

The entity map is NOT a static module import — it's the output of `buildEntityMap(deps)` called during service wiring.

```typescript
// packages/core/src/storage/entity-map.ts
export interface EntityMapDeps {
  embeddingService: EmbeddingService
  // Future: auditService, eventBus, etc.
}

export function buildEntityMap(deps: EntityMapDeps): Record<string, EntityDefinition> {
  return {
    bullets: {
      fields: { /* ... */ },
      cascade: [/* ... */],
      restrict: [/* ... */],
      setNull: [],
      hooks: {
        afterCreate: [createEmbedHook(deps.embeddingService, 'bullet')],
      },
    },
    // ... 46 more entities
  }
}

function createEmbedHook(service: EmbeddingService, entityType: string): HookFn {
  return async (ctx) => {
    queueMicrotask(() =>
      service.embed(entityType, ctx.id!, ctx.data.content as string)
        .catch(err => console.error(`Embedding failed:`, err))
    )
  }
}
```

**App wiring** (in `packages/core/src/index.ts`):

```typescript
const adapter = new SqliteAdapter(db)
const entityMap = buildEntityMap({ embeddingService: services.embedding })
const elm = new EntityLifecycleManager(adapter, entityMap)
services.storage = elm
```

**Test wiring** (mock services):

```typescript
const mockEmbed = { embed: vi.fn(), /* ... */ }
const entityMap = buildEntityMap({ embeddingService: mockEmbed })
const elm = new EntityLifecycleManager(testAdapter, entityMap)
```

This matches the existing dependency injection pattern in Forge. The entity map joins the service container rather than being a static module, but that's a minor tradeoff for clean closures.

### 8.3 Composite Primary Keys

**Decision:** Adapter interface uses single-field IDs only. Junction tables are accessed via `list`/`count`/`deleteWhere`, never `get`-by-ID.

Junction tables are link records, not entities with identity. There is no meaningful operation "get the bullet_skill with ID X" — the semantics are "does a link exist between bullet Y and skill Z?" The adapter's `count(entityType, where)` and `deleteWhere(entityType, where)` handle this naturally.

**Schema audit confirming this decision:**

| Pure composite PK (uses this pattern) | Surrogate key + UNIQUE (normal entity) |
|---|---|
| `bullet_skills (bullet_id, skill_id)` | `resume_skills (id, UNIQUE(section_id, skill_id))` |
| `bullet_sources (bullet_id, source_id)` | `resume_certifications (id, UNIQUE(resume_id, certification_id))` |
| `perspective_skills (perspective_id, skill_id)` | `org_aliases (id, UNIQUE(organization_id, alias))` |
| `source_skills (source_id, skill_id)` | |
| `jd_skills (jd_id, skill_id)` | |
| `summary_skills (summary_id, skill_id)` | |
| `certification_skills (certification_id, skill_id)` | |
| `skill_domains (skill_id, domain_id)` | |
| `archetype_domains (archetype_id, domain_id)` | |
| `contact_organizations (contact_id, org_id, relationship)` | |
| `contact_job_descriptions (contact_id, jd_id, relationship)` | |
| `contact_resumes (contact_id, resume_id, relationship)` | |
| `org_tags (organization_id, tag)` | |
| `job_description_resumes (jd_id, resume_id)` | |
| `note_references (note_id, entity_type, entity_id)` | |
| `v1_import_map (v1_entity_type, v1_id)` | |

All pure composite PK tables are junctions. No surrogate-key entity needs composite access. The decision holds.

**Junction operations via standard interface:**

```typescript
// Check if link exists
const exists = (await elm.count('bullet_skills', { bullet_id, skill_id })) > 0

// Add link (integrity layer handles UNIQUE violation as CONFLICT error)
await elm.create('bullet_skills', { bullet_id, skill_id })

// Remove link
await elm.deleteWhere('bullet_skills', { bullet_id, skill_id })

// List all links for a bullet
await elm.list('bullet_skills', { where: { bullet_id } })
```

### 8.4 Named Query Return Types

**Decision:** Types live in `packages/core/src/storage/types.ts`. Entity types are inferred from the entity map where possible. Composed query results are hand-written.

**Entity types (inferred):**

```typescript
// Utility type that reads the entity map's field definitions
type EntityOf<M, K extends keyof M> = {
  [F in keyof M[K]['fields']]: InferFieldType<M[K]['fields'][F]>
}

type InferFieldType<F> =
  F extends { type: 'text' } ? string :
  F extends { type: 'integer', boolean: true } ? boolean :
  F extends { type: 'integer' } ? number :
  F extends { type: 'real' } ? number :
  F extends { type: 'blob' } ? Uint8Array :
  F extends { type: 'json' } ? Record<string, unknown> :
  never

// Then, entity types are mechanically derived:
export type Bullet = EntityOf<typeof ENTITY_MAP_SHAPE, 'bullets'>
export type Perspective = EntityOf<typeof ENTITY_MAP_SHAPE, 'perspectives'>
export type Source = EntityOf<typeof ENTITY_MAP_SHAPE, 'sources'>
// ... etc
```

Because the entity map is a factory function (`buildEntityMap(deps)`), we need a separate static shape for type inference — hence `ENTITY_MAP_SHAPE`, which contains just the field definitions without hooks. The factory function augments this shape with bound hooks at runtime.

**Composed query types (hand-written):**

```typescript
// Multi-entity compositions can't be inferred — write them explicitly
export interface TraceChainResult {
  perspective: Perspective
  bullet: Bullet
  sources: Source[]
}

export interface ResumeDocument {
  resume: Resume
  sections: Array<ResumeSection & {
    entries: Array<ResumeEntry & {
      perspective?: Perspective
      source?: Source
    }>
  }>
  profile: UserProfile
}

export interface AlignmentReport {
  overall_score: number
  strong_matches: Match[]
  adjacent_matches: Match[]
  gaps: JDRequirement[]
  unmatched_entries: ResumeEntry[]
}

export interface Match {
  requirement: JDRequirement
  entry: ResumeEntry
  similarity: number
  classification: 'strong' | 'adjacent'
}
```

**Named query registry references these types:**

```typescript
interface NamedQueryRegistry {
  traceChain: {
    params: { perspectiveId: string }
    result: TraceChainResult
  }
  resumeCompilerIR: {
    params: { resumeId: string }
    result: ResumeDocument
  }
  alignmentScore: {
    params: { jdId: string, resumeId: string }
    result: AlignmentReport
  }
  // ...
}
```

This gives end-to-end type safety without duplicate type definitions. Adding a field to an entity automatically updates every consumer. Composed query types remain explicit because their shape is a design choice, not a mechanical derivation.

**Migration note:** Current type definitions scattered across `packages/core/src/types/`, repository files, and service files must be consolidated into `packages/core/src/storage/types.ts` as part of Phase 0. This is straightforward but touches many files — the PRAGMA validation test catches any drift.

## 9. Remaining Open Questions (flagged for implementation)

1. **Performance of integrity layer cascade.** For a deep cascade (delete resume → sections → entries + skills + certs), the integrity layer issues many individual queries. Acceptable for Phase 0 with SQLite (same file, <1ms per query). May need optimization (batch `deleteWhere`) for remote backends like HelixDB.

2. **Transaction isolation across named queries.** Named queries execute outside explicit transactions by default. Services that need "read-modify-write" across a named query + CRUD may need to wrap the whole sequence in `elm.transaction()`. The adapter's named query implementations must accept an optional transaction scope.

3. **Error context enrichment.** When a cascade delete fails mid-traversal, which entity failed? The integrity layer needs to include cascade path in error details (e.g., "failed at bullet_skills during skills cascade"). Implementation detail, not a design question.

---

## 10. Success Criteria

Phase 0 is complete when:

- [ ] `StorageAdapter` interface defined and exported
- [ ] `EntityLifecycleManager` implemented with constraint enforcement + hooks
- [ ] Relationship map covers all 47 entities with fields, FK, cascade, restrict, setNull
- [ ] PRAGMA validation test passes (map matches actual SQLite schema)
- [ ] `SqliteAdapter` implements full CRUD + transactions + named queries
- [ ] Lifecycle hooks implemented: setUpdatedAt, captureSnapshot, fireAndForgetEmbed
- [ ] Integration tests: create/read/update/delete through lifecycle manager for representative entities
- [ ] Integration tests: cascade delete (resume → sections → entries), restrict (delete bullet with perspectives), setNull (delete org with source_roles)
- [ ] No services rewired yet (that's Phase 1) — but a proof-of-concept test demonstrates the full stack
