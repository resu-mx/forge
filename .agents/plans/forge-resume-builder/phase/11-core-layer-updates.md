# Phase 11: Core Layer Updates (Repositories + Services)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update all existing repositories and services to work with the evolved v2 schema (organizations, polymorphic sources, bullet_sources junction, resume_entries), and create new repositories/services for organizations, notes, resume entries, and integrity checking.

**Architecture:** Repositories are pure data-access functions (no business logic). Services validate input, enforce business rules, and compose repository calls. Both layers take a `Database` instance via constructor injection. The `createServices()` factory creates all services and is the entry point for route handlers. New repositories follow the same patterns as existing ones: static functions or object literals with CRUD methods, parameterized queries, `RETURNING *` clauses.

**Tech Stack:** TypeScript, SQLite (via `bun:sqlite`), `bun:test`, `crypto.randomUUID()`

**Depends on:** Phase 10 (schema migration + updated types + test helpers)
**Blocks:** Phase 12 (API/routes), Phase 13 (SDK/CLI)
**Parallelizable:** Two layers with dependencies between them:
- **Layer 1 (repositories):** T11.1-T11.6 can run in parallel. Each repository task is independent (different files, different tables).
- **Layer 2 (services):** T11.7-T11.14 can run in parallel AFTER Layer 1 is complete. T11.7-T11.10 depend on repo methods from T11.1-T11.3 (e.g., DerivationService uses BulletRepository.create with source_ids, AuditService uses BulletRepository.getPrimarySource, ReviewService uses bullet_sources JOIN, ResumeService uses ResumeRepository.addEntry).

---

## Context

After Phase 10, the database schema has been evolved but the repository and service code still references the old schema. Specifically:
- `SourceRepository` references `employer_id`, `project_id` columns that no longer exist
- `BulletRepository` references `source_id` column that no longer exist; uses `CreateBulletInput.source_id`
- `ResumeRepository` references `resume_perspectives` table that no longer exists
- `PerspectiveRepository.getWithChain()` JOINs through `bullets.source_id` which no longer exists
- `DerivationService` inserts `source_id` into bullets
- `AuditService` reads `bullet.source_id` to find the source
- `ReviewService` JOINs `bullets.source_id` to `sources.id`
- `ResumeService` calls `addPerspective`/`removePerspective`/`reorderPerspectives` on `ResumeRepository`
- `EmployerRepository` and `ProjectRepository` reference tables that no longer exist

Type interfaces have been updated in Phase 10 (T10.4), so the new field names are available. Test helpers have been updated in Phase 10 (T10.2), so seed functions use the new schema.

## Goals

- Update all existing repositories to use the new schema
- Update all existing services to use the new schema
- Create new repositories: OrganizationRepository, NoteRepository, ResumeEntryRepository
- Create new services: OrganizationService, NoteService, IntegrityService
- Update the `createServices` factory
- Delete `EmployerRepository` and `ProjectRepository` (tables no longer exist)
- Write comprehensive tests for all changes
- All `test.skip` markers from Phase 10 are removed and tests pass

## Non-Goals

- Updating HTTP routes (Phase 12)
- Updating SDK client code (Phase 13)
- Writing the v1 import command (separate phase)
- Implementing the Chain View graph (UI phase)

## Fallback Strategies

- If a JOIN-based query performs poorly, add a database index and re-test. All junction tables already have indexes on both sides.
- If the polymorphic extension JOIN becomes unwieldy, consider using a UNION ALL approach for `list()` queries that need extension data.
- If `RETURNING *` fails after a complex multi-table insert (source + extension), use separate INSERT then SELECT.
- If transaction nesting is needed and SQLite doesn't support it, use SAVEPOINT/RELEASE instead.

---

## Tasks

### Task 11.1: Update SourceRepository for polymorphic sources

**Files:**
- Modify: `packages/core/src/db/repositories/source-repository.ts`
- Modify: `packages/core/src/db/repositories/__tests__/source-repository.test.ts`

**Goal:** Make SourceRepository work with polymorphic sources (source_type + extension tables), remove all references to `employer_id`/`project_id`.

- [ ] **Update `SourceFilter` interface.** Remove `employer_id`, `project_id`. Add `source_type`, `organization_id`.

```typescript
export interface SourceFilter {
  source_type?: SourceType
  organization_id?: string  // filters via JOIN to source_roles or source_projects
  status?: SourceStatus
}
```

- [ ] **Update `create()` function.** Accept `source_type` and extension fields. Atomically create base source row + extension row in a transaction.

```typescript
export function create(db: Database, input: CreateSource): SourceWithExtension {
  const id = crypto.randomUUID()
  const now = new Date().toISOString()
  const sourceType = input.source_type ?? 'general'

  const txn = db.transaction(() => {
    // Insert base source row
    db.run(
      `INSERT INTO sources (id, title, description, source_type, start_date, end_date, status, updated_by, created_at, updated_at)
       VALUES (?, ?, ?, ?, ?, ?, 'draft', 'human', ?, ?)`,
      [id, input.title, input.description, sourceType, input.start_date ?? null, input.end_date ?? null, now, now]
    )

    // Insert extension row based on source_type
    if (sourceType === 'role') {
      db.run(
        `INSERT INTO source_roles (source_id, organization_id, start_date, end_date, is_current, work_arrangement, base_salary, total_comp_notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
        [id, input.organization_id ?? null, input.start_date ?? null, input.end_date ?? null,
         input.is_current ?? 0, input.work_arrangement ?? null, input.base_salary ?? null, input.total_comp_notes ?? null]
      )
    } else if (sourceType === 'project') {
      db.run(
        `INSERT INTO source_projects (source_id, organization_id, is_personal, url, start_date, end_date)
         VALUES (?, ?, ?, ?, ?, ?)`,
        [id, input.organization_id ?? null, input.is_personal ?? 0, input.url ?? null,
         input.start_date ?? null, input.end_date ?? null]
      )
    } else if (sourceType === 'education') {
      db.run(
        `INSERT INTO source_education (source_id, education_type, institution, field, start_date, end_date, is_in_progress, credential_id, expiration_date, issuing_body, url)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
        [id, input.education_type ?? 'certificate', input.institution ?? null, input.field ?? null,
         input.start_date ?? null, input.end_date ?? null, input.is_in_progress ?? 0,
         input.credential_id ?? null, input.expiration_date ?? null, input.issuing_body ?? null, input.url ?? null]
      )
    } else if (sourceType === 'clearance') {
      db.run(
        `INSERT INTO source_clearances (source_id, level, polygraph, status, sponsoring_agency, investigation_date, adjudication_date, reinvestigation_date, read_on)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
        [id, input.level ?? '', input.polygraph ?? null, input.clearance_status ?? null,
         input.sponsoring_agency ?? null, null, null, null, null]
      )
    }
    // 'general' type has no extension table
  })

  txn()
  return get(db, id)!
}
```

- [ ] **Update `get()` function.** LEFT JOIN the appropriate extension table based on `source_type`. Return `SourceWithExtension`.

```typescript
export function get(db: Database, id: string): SourceWithExtension | null {
  const source = db
    .query('SELECT * FROM sources WHERE id = ?')
    .get(id) as Source | null

  if (!source) return null

  const extension = getExtension(db, source.id, source.source_type)
  return { ...source, extension }
}

function getExtension(db: Database, sourceId: string, sourceType: string): SourceRole | SourceProject | SourceEducation | SourceClearance | null {
  switch (sourceType) {
    case 'role':
      return db.query('SELECT * FROM source_roles WHERE source_id = ?').get(sourceId) as SourceRole | null
    case 'project':
      return db.query('SELECT * FROM source_projects WHERE source_id = ?').get(sourceId) as SourceProject | null
    case 'education':
      return db.query('SELECT * FROM source_education WHERE source_id = ?').get(sourceId) as SourceEducation | null
    case 'clearance':
      return db.query('SELECT * FROM source_clearances WHERE source_id = ?').get(sourceId) as SourceClearance | null
    default:
      return null
  }
}
```

- [ ] **Update `list()` function.** Replace `employer_id`/`project_id` filters with `source_type` and `organization_id` filters. The `organization_id` filter requires a JOIN to `source_roles` or `source_projects`.

```typescript
export function list(
  db: Database,
  filter: SourceFilter,
  offset: number,
  limit: number,
): SourceListResult {
  const conditions: string[] = []
  const params: unknown[] = []
  let joinClause = ''

  if (filter.source_type !== undefined) {
    conditions.push('s.source_type = ?')
    params.push(filter.source_type)
  }
  if (filter.status !== undefined) {
    conditions.push('s.status = ?')
    params.push(filter.status)
  }
  if (filter.organization_id !== undefined) {
    // JOIN to both source_roles and source_projects, match either
    joinClause = `LEFT JOIN source_roles sr ON s.id = sr.source_id
                  LEFT JOIN source_projects sp ON s.id = sp.source_id`
    conditions.push('(sr.organization_id = ? OR sp.organization_id = ?)')
    params.push(filter.organization_id, filter.organization_id)
  }

  const where = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : ''

  const countRow = db
    .query(`SELECT COUNT(DISTINCT s.id) AS total FROM sources s ${joinClause} ${where}`)
    .get(...params) as { total: number }

  const dataParams = [...params, limit, offset]
  const data = db
    .query(`SELECT DISTINCT s.* FROM sources s ${joinClause} ${where} ORDER BY s.created_at DESC LIMIT ? OFFSET ?`)
    .all(...dataParams) as Source[]

  return { data, total: countRow.total }
}
```

- [ ] **Update `update()` function.** Remove `employer_id`/`project_id` handling. Add `notes` and extension field updates.

```typescript
export function update(db: Database, id: string, input: UpdateSource): SourceWithExtension | null {
  const existing = get(db, id)
  if (!existing) return null

  // Update base source fields
  const sets: string[] = []
  const params: unknown[] = []

  if (input.title !== undefined) { sets.push('title = ?'); params.push(input.title) }
  if (input.description !== undefined) { sets.push('description = ?'); params.push(input.description) }
  if ('start_date' in input) { sets.push('start_date = ?'); params.push(input.start_date ?? null) }
  if ('end_date' in input) { sets.push('end_date = ?'); params.push(input.end_date ?? null) }
  if ('notes' in input) { sets.push('notes = ?'); params.push(input.notes ?? null) }

  const now = new Date().toISOString()
  sets.push('updated_at = ?')
  params.push(now)
  params.push(id)

  db.run(`UPDATE sources SET ${sets.join(', ')} WHERE id = ?`, params)

  // Update extension table if applicable
  updateExtension(db, id, existing.source_type, input)

  return get(db, id)!
}
```

- [ ] **`del()` remains unchanged** (CASCADE handles extension tables).

- [ ] **Update `acquireDerivingLock()` and `releaseDerivingLock()`** -- these operate on `sources` base table columns only, so they need no schema changes. But their return type should be updated to `SourceWithExtension` (or keep `Source` since the lock doesn't need extension data).

- [ ] **Write tests.** Update `source-repository.test.ts`:

```typescript
test('create with source_type=role creates base + extension', () => {
  const orgId = seedOrganization(db)
  const source = SourceRepo.create(db, {
    title: 'Senior Engineer',
    description: 'Led cloud migration.',
    source_type: 'role',
    organization_id: orgId,
    is_current: 1,
  })

  expect(source.source_type).toBe('role')
  expect(source.extension).not.toBeNull()
  const ext = source.extension as SourceRole
  expect(ext.organization_id).toBe(orgId)
  expect(ext.is_current).toBe(1)
})

test('create with source_type=general has no extension', () => {
  const source = SourceRepo.create(db, {
    title: 'General Note',
    description: 'A general source.',
  })
  expect(source.source_type).toBe('general')
  expect(source.extension).toBeNull()
})

test('list filters by source_type', () => {
  seedSource(db, { sourceType: 'role' })
  seedSource(db, { sourceType: 'education' })
  seedSource(db, { sourceType: 'general' })

  const result = SourceRepo.list(db, { source_type: 'role' }, 0, 50)
  expect(result.total).toBe(1)
  expect(result.data[0].source_type).toBe('role')
})

test('list filters by organization_id via extension JOIN', () => {
  const orgId = seedOrganization(db)
  seedSource(db, { sourceType: 'role', organizationId: orgId })
  seedSource(db, { sourceType: 'role' })  // no org

  const result = SourceRepo.list(db, { organization_id: orgId }, 0, 50)
  expect(result.total).toBe(1)
})
```

---

### Task 11.2: Update BulletRepository for junction-based sources

**Files:**
- Modify: `packages/core/src/db/repositories/bullet-repository.ts`
- Modify: `packages/core/src/db/repositories/__tests__/bullet-repository.test.ts`

**Goal:** Remove `source_id` from bullets. Use `bullet_sources` junction table for source associations. Add `domain` and `notes` fields.

- [ ] **Update `Bullet` interface (local to repository).** Remove `source_id`. Add `domain`, `notes`.

```typescript
export interface Bullet {
  id: string
  content: string
  source_content_snapshot: string
  metrics: string | null
  domain: string | null
  status: BulletStatus
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  notes: string | null
  created_at: string
  technologies: string[]
}
```

- [ ] **Update `CreateBulletInput`.** Remove `source_id`. Add optional `source_ids` array and `domain`.

```typescript
export interface CreateBulletInput {
  content: string
  source_content_snapshot: string
  technologies: string[]
  metrics: string | null
  domain?: string | null
  status?: string
  prompt_log_id?: string
  source_ids?: Array<{ id: string; is_primary?: boolean }>
}
```

- [ ] **Update `BulletFilter`.** `source_id` filter becomes a JOIN through `bullet_sources`.

```typescript
export interface BulletFilter {
  source_id?: string   // filters via JOIN through bullet_sources
  status?: string
  technology?: string
  domain?: string
}
```

- [ ] **Update `BulletRow` interface.** Remove `source_id`.

- [ ] **Update `rowToBullet` helper.** Remove `source_id`. Add `domain`, `notes`.

```typescript
function rowToBullet(row: BulletRow, technologies: string[]): Bullet {
  return {
    id: row.id,
    content: row.content,
    source_content_snapshot: row.source_content_snapshot,
    metrics: row.metrics,
    domain: row.domain,
    status: row.status as BulletStatus,
    rejection_reason: row.rejection_reason,
    prompt_log_id: row.prompt_log_id,
    approved_at: row.approved_at,
    approved_by: row.approved_by,
    notes: row.notes,
    created_at: row.created_at,
    technologies,
  }
}
```

- [ ] **Update `create()`.** No longer inserts `source_id`. Instead, inserts `bullet_sources` rows after creating the bullet.

```typescript
create(db: Database, input: CreateBulletInput): Bullet {
  const id = crypto.randomUUID()
  const status = input.status ?? 'pending_review'

  const row = db
    .query(
      `INSERT INTO bullets (id, content, source_content_snapshot, metrics, domain, status, prompt_log_id)
       VALUES (?, ?, ?, ?, ?, ?, ?)
       RETURNING *`,
    )
    .get(id, input.content, input.source_content_snapshot, input.metrics,
         input.domain ?? null, status, input.prompt_log_id ?? null) as BulletRow

  // Insert bullet_sources junction rows
  if (input.source_ids) {
    for (const src of input.source_ids) {
      db.run(
        'INSERT INTO bullet_sources (bullet_id, source_id, is_primary) VALUES (?, ?, ?)',
        [id, src.id, src.is_primary !== false ? 1 : 0]
      )
    }
  }

  insertTechnologies(db, id, input.technologies)
  const technologies = getTechnologies(db, id)
  return rowToBullet(row, technologies)
},
```

- [ ] **Update `list()`.** The `source_id` filter becomes a JOIN through `bullet_sources`.

```typescript
list(db: Database, filter: BulletFilter = {}, offset = 0, limit = 50): BulletListResult {
  const conditions: string[] = []
  const params: unknown[] = []
  const joins: string[] = []

  if (filter.source_id) {
    joins.push('JOIN bullet_sources bs ON bs.bullet_id = b.id')
    conditions.push('bs.source_id = ?')
    params.push(filter.source_id)
  }
  if (filter.status) {
    conditions.push('b.status = ?')
    params.push(filter.status)
  }
  if (filter.technology) {
    joins.push('JOIN bullet_technologies bt ON bt.bullet_id = b.id')
    conditions.push('bt.technology = ?')
    params.push(filter.technology.toLowerCase().trim())
  }
  if (filter.domain) {
    conditions.push('b.domain = ?')
    params.push(filter.domain)
  }

  const joinClause = joins.join(' ')
  const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : ''

  const countRow = db
    .query(`SELECT COUNT(DISTINCT b.id) as total FROM bullets b ${joinClause} ${whereClause}`)
    .get(...params) as { total: number }

  const rows = db
    .query(
      `SELECT DISTINCT b.* FROM bullets b ${joinClause} ${whereClause}
       ORDER BY b.created_at DESC LIMIT ? OFFSET ?`,
    )
    .all(...params, limit, offset) as BulletRow[]

  const data = rows.map(row => {
    const technologies = getTechnologies(db, row.id)
    return rowToBullet(row, technologies)
  })

  return { data, total: countRow.total }
},
```

- [ ] **Add `getSources()` helper method** to retrieve all sources for a bullet via the junction table.

```typescript
/** Get all sources associated with a bullet via bullet_sources junction */
getSources(db: Database, bulletId: string): Array<{ id: string; title: string; is_primary: number }> {
  return db
    .query(
      `SELECT s.id, s.title, bs.is_primary
       FROM bullet_sources bs
       JOIN sources s ON bs.source_id = s.id
       WHERE bs.bullet_id = ?
       ORDER BY bs.is_primary DESC, s.title ASC`,
    )
    .all(bulletId) as Array<{ id: string; title: string; is_primary: number }>
},

/** Get the primary source for a bullet */
getPrimarySource(db: Database, bulletId: string): { id: string; title: string } | null {
  return db
    .query(
      `SELECT s.id, s.title
       FROM bullet_sources bs
       JOIN sources s ON bs.source_id = s.id
       WHERE bs.bullet_id = ? AND bs.is_primary = 1`,
    )
    .get(bulletId) as { id: string; title: string } | null
},

/** Add a source association to a bullet */
addSource(db: Database, bulletId: string, sourceId: string, isPrimary = false): void {
  db.run(
    'INSERT INTO bullet_sources (bullet_id, source_id, is_primary) VALUES (?, ?, ?)',
    [bulletId, sourceId, isPrimary ? 1 : 0]
  )
},

/** Remove a source association from a bullet */
removeSource(db: Database, bulletId: string, sourceId: string): boolean {
  const result = db.run(
    'DELETE FROM bullet_sources WHERE bullet_id = ? AND source_id = ?',
    [bulletId, sourceId]
  )
  return result.changes > 0
},
```

- [ ] **Write tests.**

```typescript
test('create without source_ids creates bullet with no sources', () => {
  const bullet = BulletRepository.create(db, {
    content: 'Test bullet',
    source_content_snapshot: 'snapshot',
    technologies: ['typescript'],
    metrics: null,
  })
  expect(bullet.id).toHaveLength(36)
  expect(bullet.domain).toBeNull()
  const sources = BulletRepository.getSources(db, bullet.id)
  expect(sources).toHaveLength(0)
})

test('create with source_ids creates junction rows', () => {
  const srcId = seedSource(db)
  const bullet = BulletRepository.create(db, {
    content: 'Test bullet',
    source_content_snapshot: 'snapshot',
    technologies: [],
    metrics: null,
    source_ids: [{ id: srcId, is_primary: true }],
  })
  const sources = BulletRepository.getSources(db, bullet.id)
  expect(sources).toHaveLength(1)
  expect(sources[0].is_primary).toBe(1)
})

test('list filters by source_id via junction', () => {
  const src1 = seedSource(db)
  const src2 = seedSource(db, { title: 'Source 2' })
  seedBullet(db, [{ id: src1 }])
  seedBullet(db, [{ id: src2 }])
  seedBullet(db, [{ id: src1 }, { id: src2, isPrimary: false }])

  const result = BulletRepository.list(db, { source_id: src1 })
  expect(result.total).toBe(2)  // bullet 1 + bullet 3
})

test('getPrimarySource returns the primary source', () => {
  const src1 = seedSource(db, { title: 'Primary' })
  const src2 = seedSource(db, { title: 'Secondary' })
  const bulletId = seedBullet(db, [
    { id: src1, isPrimary: true },
    { id: src2, isPrimary: false },
  ])
  const primary = BulletRepository.getPrimarySource(db, bulletId)
  expect(primary).not.toBeNull()
  expect(primary!.title).toBe('Primary')
})

test('addSource with isPrimary=true demotes existing primary', () => {
  // When a second source is added as primary, the first primary should be demoted.
  // Implementation note: addSource(bulletId, sourceId, isPrimary=true) should
  // UPDATE bullet_sources SET is_primary = 0 WHERE bullet_id = ? AND is_primary = 1
  // before inserting the new row with is_primary = 1.
  const src1 = seedSource(db, { title: 'First Primary' })
  const src2 = seedSource(db, { title: 'New Primary' })
  const bulletId = seedBullet(db, [{ id: src1, isPrimary: true }])

  BulletRepository.addSource(db, bulletId, src2, true)

  const sources = BulletRepository.getSources(db, bulletId)
  const primaries = sources.filter(s => s.is_primary === 1)
  expect(primaries).toHaveLength(1)
  expect(primaries[0].title).toBe('New Primary')
})
```

> **Implementation note for `addSource()`:** When `isPrimary=true`, the method must first demote any existing primary source for that bullet by running `UPDATE bullet_sources SET is_primary = 0 WHERE bullet_id = ? AND is_primary = 1` before inserting the new junction row. This ensures the invariant that each bullet has at most one primary source.

---

### Task 11.3: Update ResumeRepository for resume_entries

**Files:**
- Modify: `packages/core/src/db/repositories/resume-repository.ts`
- Modify: `packages/core/src/db/repositories/__tests__/resume-repository.test.ts`

**Goal:** Replace all `resume_perspectives` references with `resume_entries`. Implement copy-on-write semantics for entry content.

- [ ] **Update imports.** Replace `ResumeWithPerspectives`, `AddResumePerspective`, `ReorderPerspectives` with `ResumeWithEntries`, `AddResumeEntry`, `ResumeEntry`.

- [ ] **Update `PerspectiveJoinRow` -> `EntryJoinRow`.**

```typescript
interface EntryJoinRow {
  // resume_entries fields
  entry_id: string
  section: string
  position: number
  entry_content: string | null
  perspective_content_snapshot: string | null
  entry_notes: string | null
  entry_created_at: string
  entry_updated_at: string
  // perspectives fields
  perspective_id: string
  bullet_id: string
  perspective_content: string
  // ...other perspective fields
}
```

- [ ] **Replace `getWithPerspectives` with `getWithEntries`.**

```typescript
getWithEntries(db: Database, id: string): ResumeWithEntries | null {
  const resume = ResumeRepository.get(db, id)
  if (!resume) return null

  const rows = db
    .query(
      `SELECT
         re.id AS entry_id,
         re.section,
         re.position,
         re.content AS entry_content,
         re.perspective_content_snapshot,
         re.notes AS entry_notes,
         re.created_at AS entry_created_at,
         re.updated_at AS entry_updated_at,
         re.perspective_id,
         p.bullet_id,
         p.content AS perspective_content
       FROM resume_entries re
       JOIN perspectives p ON p.id = re.perspective_id
       WHERE re.resume_id = ?
       ORDER BY re.section, re.position`,
    )
    .all(id) as EntryJoinRow[]

  const sections: Record<string, ResumeEntry[]> = {}

  for (const row of rows) {
    if (!sections[row.section]) {
      sections[row.section] = []
    }
    sections[row.section].push({
      id: row.entry_id,
      resume_id: id,
      perspective_id: row.perspective_id,
      content: row.entry_content,
      perspective_content_snapshot: row.perspective_content_snapshot,
      section: row.section,
      position: row.position,
      notes: row.entry_notes,
      created_at: row.entry_created_at,
      updated_at: row.entry_updated_at,
    })
  }

  return { ...resume, sections }
},
```

- [ ] **Replace `addPerspective` with `addEntry`.**

```typescript
addEntry(db: Database, resumeId: string, input: AddResumeEntry): ResumeEntry {
  const id = crypto.randomUUID()
  db.run(
    `INSERT INTO resume_entries (id, resume_id, perspective_id, content, section, position, notes)
     VALUES (?, ?, ?, ?, ?, ?, ?)`,
    [id, resumeId, input.perspective_id, input.content ?? null,
     input.section, input.position, input.notes ?? null],
  )
  return db.query('SELECT * FROM resume_entries WHERE id = ?').get(id) as ResumeEntry
},
```

- [ ] **Replace `removePerspective` with `removeEntry`.**

```typescript
removeEntry(db: Database, resumeId: string, entryId: string): boolean {
  const result = db.run(
    'DELETE FROM resume_entries WHERE id = ? AND resume_id = ?',
    [entryId, resumeId],
  )
  return result.changes > 0
},
```

- [ ] **Replace `reorderPerspectives` with `reorderEntries`.**

```typescript
reorderEntries(db: Database, resumeId: string, entries: Array<{ id: string; section: string; position: number }>): void {
  const txn = db.transaction(() => {
    const stmt = db.prepare(
      `UPDATE resume_entries SET section = ?, position = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
       WHERE id = ? AND resume_id = ?`,
    )
    for (const entry of entries) {
      stmt.run(entry.section, entry.position, entry.id, resumeId)
    }
  })
  txn()
},
```

- [ ] **Add `updateEntry` method** for copy-on-write.

```typescript
updateEntry(db: Database, entryId: string, input: {
  content?: string | null
  section?: string
  position?: number
  notes?: string | null
}): ResumeEntry | null {
  const sets: string[] = []
  const params: unknown[] = []

  // content: undefined = no change, null = reset to reference mode, string = clone/edit
  if ('content' in input) {
    sets.push('content = ?')
    params.push(input.content)
    // If setting content (clone), capture snapshot
    if (input.content !== null && input.content !== undefined) {
      // Capture the perspective's current content as snapshot
      sets.push('perspective_content_snapshot = (SELECT content FROM perspectives WHERE id = (SELECT perspective_id FROM resume_entries WHERE id = ?))')
      params.push(entryId)
    } else {
      // Reset to reference mode, clear snapshot
      sets.push('perspective_content_snapshot = NULL')
    }
  }
  if (input.section !== undefined) { sets.push('section = ?'); params.push(input.section) }
  if (input.position !== undefined) { sets.push('position = ?'); params.push(input.position) }
  if ('notes' in input) { sets.push('notes = ?'); params.push(input.notes ?? null) }

  if (sets.length === 0) return db.query('SELECT * FROM resume_entries WHERE id = ?').get(entryId) as ResumeEntry | null

  sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
  params.push(entryId)

  const row = db
    .query(`UPDATE resume_entries SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
    .get(...params) as ResumeEntry | null

  return row
},
```

- [ ] **Write tests.**

```typescript
test('addEntry creates a resume entry in reference mode (content=NULL)', () => {
  const resumeId = seedResume(db)
  const srcId = seedSource(db)
  const bulletId = seedBullet(db, [{ id: srcId }])
  const perspId = seedPerspective(db, bulletId)

  const entry = ResumeRepository.addEntry(db, resumeId, {
    perspective_id: perspId,
    section: 'work_history',
    position: 0,
  })
  expect(entry.content).toBeNull()
  expect(entry.perspective_id).toBe(perspId)
})

test('updateEntry with content string enters clone mode', () => {
  const resumeId = seedResume(db)
  const srcId = seedSource(db)
  const bulletId = seedBullet(db, [{ id: srcId }])
  const perspId = seedPerspective(db, bulletId)
  const entryId = seedResumeEntry(db, resumeId, perspId)

  const updated = ResumeRepository.updateEntry(db, entryId, {
    content: 'Customized content for this resume',
  })
  expect(updated!.content).toBe('Customized content for this resume')
  expect(updated!.perspective_content_snapshot).not.toBeNull()
})

test('updateEntry with content=null resets to reference mode', () => {
  // First clone, then reset
  const resumeId = seedResume(db)
  const srcId = seedSource(db)
  const bulletId = seedBullet(db, [{ id: srcId }])
  const perspId = seedPerspective(db, bulletId)
  const entryId = seedResumeEntry(db, resumeId, perspId)

  ResumeRepository.updateEntry(db, entryId, { content: 'Cloned' })
  const reset = ResumeRepository.updateEntry(db, entryId, { content: null })
  expect(reset!.content).toBeNull()
  expect(reset!.perspective_content_snapshot).toBeNull()
})

test('getWithEntries returns entries grouped by section', () => {
  const resumeId = seedResume(db)
  const srcId = seedSource(db)
  const bulletId = seedBullet(db, [{ id: srcId }])
  const p1 = seedPerspective(db, bulletId)
  const p2 = seedPerspective(db, bulletId, { domain: 'security' })
  seedResumeEntry(db, resumeId, p1, { section: 'work_history', position: 0 })
  seedResumeEntry(db, resumeId, p2, { section: 'skills', position: 0 })

  const result = ResumeRepository.getWithEntries(db, resumeId)!
  expect(Object.keys(result.sections)).toContain('work_history')
  expect(Object.keys(result.sections)).toContain('skills')
  expect(result.sections['work_history']).toHaveLength(1)
})
```

---

### Task 11.4: New OrganizationRepository

**Files:**
- Create: `packages/core/src/db/repositories/organization-repository.ts`
- Create: `packages/core/src/db/repositories/__tests__/organization-repository.test.ts`

**Goal:** CRUD for the `organizations` table.

- [ ] **Create repository with standard CRUD pattern.**

```typescript
import type { Database } from 'bun:sqlite'
import type { Organization } from '../../types'

export interface CreateOrganizationInput {
  name: string
  org_type?: string
  industry?: string
  size?: string
  worked?: number
  employment_type?: string
  location?: string
  headquarters?: string
  website?: string
  linkedin_url?: string
  glassdoor_url?: string
  glassdoor_rating?: number
  reputation_notes?: string
  notes?: string
}

export interface OrganizationFilter {
  org_type?: string
  worked?: number
}

export function create(db: Database, input: CreateOrganizationInput): Organization { ... }
export function get(db: Database, id: string): Organization | null { ... }
export function list(db: Database, filter?: OrganizationFilter, offset?: number, limit?: number): { data: Organization[]; total: number } { ... }
export function update(db: Database, id: string, input: Partial<CreateOrganizationInput>): Organization | null { ... }
export function del(db: Database, id: string): boolean { ... }
```

- [ ] **Write tests:** create, get, get-null, list-all, list-filter-by-org_type, list-filter-by-worked, update, delete.

---

### Task 11.5: New NoteRepository

**Files:**
- Create: `packages/core/src/db/repositories/note-repository.ts`
- Create: `packages/core/src/db/repositories/__tests__/note-repository.test.ts`

**Goal:** CRUD for `user_notes` + `note_references` tables.

- [ ] **Create repository.**

```typescript
import type { Database } from 'bun:sqlite'
import type { UserNote, NoteReference, UserNoteWithReferences } from '../../types'

export interface CreateNoteInput {
  title?: string
  content: string
}

export function create(db: Database, input: CreateNoteInput): UserNote { ... }
export function get(db: Database, id: string): UserNote | null { ... }
export function getWithReferences(db: Database, id: string): UserNoteWithReferences | null { ... }
export function list(db: Database, search?: string, offset?: number, limit?: number): { data: UserNote[]; total: number } { ... }
export function update(db: Database, id: string, input: Partial<CreateNoteInput>): UserNote | null { ... }
export function del(db: Database, id: string): boolean { ... }
export function addReference(db: Database, noteId: string, entityType: string, entityId: string): void { ... }
export function removeReference(db: Database, noteId: string, entityType: string, entityId: string): boolean { ... }
export function getByEntity(db: Database, entityType: string, entityId: string): UserNote[] { ... }
```

- [ ] **Key query for `list` with search:**

```sql
SELECT * FROM user_notes
WHERE (? IS NULL OR content LIKE '%' || ? || '%' OR title LIKE '%' || ? || '%')
ORDER BY updated_at DESC
LIMIT ? OFFSET ?
```

- [ ] **Key query for `getByEntity`:**

```sql
SELECT un.* FROM user_notes un
JOIN note_references nr ON un.id = nr.note_id
WHERE nr.entity_type = ? AND nr.entity_id = ?
ORDER BY un.updated_at DESC
```

- [ ] **Write tests:** create, get, getWithReferences, list, list-with-search, update, delete, addReference, removeReference, getByEntity.

---

### Task 11.6: New ResumeEntryRepository

**Files:**
- Create: `packages/core/src/db/repositories/resume-entry-repository.ts`
- Create: `packages/core/src/db/repositories/__tests__/resume-entry-repository.test.ts`

**Goal:** Direct CRUD for `resume_entries` table (independent of ResumeRepository), including content resolution.

- [ ] **Create repository.**

```typescript
import type { Database } from 'bun:sqlite'
import type { ResumeEntry } from '../../types'

export function create(db: Database, input: {
  resume_id: string
  perspective_id: string
  content?: string | null
  section: string
  position?: number
  notes?: string | null
}): ResumeEntry { ... }

export function get(db: Database, id: string): ResumeEntry | null { ... }

export function update(db: Database, id: string, input: {
  content?: string | null
  section?: string
  position?: number
  notes?: string | null
}): ResumeEntry | null { ... }

export function del(db: Database, id: string): boolean { ... }

export function listByResume(db: Database, resumeId: string): ResumeEntry[] { ... }

/**
 * Resolve the effective content for an entry.
 * If entry.content is non-null, return it (clone mode).
 * If entry.content is null, return the perspective's content (reference mode).
 */
export function resolveContent(db: Database, entryId: string): string | null {
  const row = db
    .query(
      `SELECT COALESCE(re.content, p.content) AS resolved_content
       FROM resume_entries re
       JOIN perspectives p ON re.perspective_id = p.id
       WHERE re.id = ?`,
    )
    .get(entryId) as { resolved_content: string } | null

  return row?.resolved_content ?? null
}
```

- [ ] **Write tests:** create, get, update-content (clone mode), update-content-null (reference reset), delete, listByResume, resolveContent (both modes).

---

### Task 11.7: Update DerivationService

**Files:**
- Modify: `packages/core/src/services/derivation-service.ts`
- Modify: `packages/core/src/services/__tests__/derivation-service.test.ts`

**Goal:** Update `deriveBulletsFromSource` to create `bullet_sources` rows instead of setting `source_id` on bullets.

- [ ] **Update the transaction in `deriveBulletsFromSource`.** After creating the bullet, insert a `bullet_sources` row.

```typescript
// Inside the transaction, replace:
const bullet = BulletRepository.create(this.db, {
  source_id: sourceId,
  content: item.content,
  source_content_snapshot: snapshot,
  technologies: item.technologies,
  metrics: item.metrics,
  status: 'pending_review',
})

// With:
const bullet = BulletRepository.create(this.db, {
  content: item.content,
  source_content_snapshot: snapshot,
  technologies: item.technologies,
  metrics: item.metrics,
  status: 'pending_review',
  source_ids: [{ id: sourceId, is_primary: true }],
})
```

- [ ] **Update tests.** Verify that after derivation, `bullet_sources` contains a row with `is_primary=1`:

```typescript
test('deriveBulletsFromSource creates bullet_sources row with is_primary=1', async () => {
  // ... setup source, mock AI response ...
  const result = await service.deriveBulletsFromSource(sourceId)
  expect(result.ok).toBe(true)
  if (!result.ok) return

  const sources = BulletRepository.getSources(db, result.data[0].id)
  expect(sources).toHaveLength(1)
  expect(sources[0].id).toBe(sourceId)
  expect(sources[0].is_primary).toBe(1)
})
```

---

### Task 11.8: Update AuditService

**Files:**
- Modify: `packages/core/src/services/audit-service.ts`
- Modify: `packages/core/src/services/__tests__/audit-service.test.ts`

**Goal:** Update `traceChain` and `checkIntegrity` to find the source via `bullet_sources` junction instead of `bullet.source_id`.

- [ ] **Update `traceChain`.** Replace `SourceRepo.get(this.db, bullet.source_id)` with a junction query.

```typescript
traceChain(perspectiveId: string): Result<ChainTrace> {
  const perspective = PerspectiveRepository.get(this.db, perspectiveId)
  if (!perspective) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Perspective ${perspectiveId} not found` } }
  }

  const bullet = BulletRepository.get(this.db, perspective.bullet_id)
  if (!bullet) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Bullet ${perspective.bullet_id} not found (chain broken)` } }
  }

  // Get primary source via junction table
  const primarySource = BulletRepository.getPrimarySource(this.db, bullet.id)
  if (!primarySource) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `No primary source for bullet ${bullet.id} (chain broken)` } }
  }

  const source = SourceRepo.get(this.db, primarySource.id)
  if (!source) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Source ${primarySource.id} not found (chain broken)` } }
  }

  return { ok: true, data: { perspective, bullet, source } }
}
```

- [ ] **Update `checkIntegrity`.** Same change -- source lookup via `bullet_sources`.

- [ ] **Update tests.** Seed data now uses junction:

```typescript
test('traceChain resolves primary source via junction', () => {
  const srcId = seedSource(db)
  const bulletId = seedBullet(db, [{ id: srcId }])
  const perspId = seedPerspective(db, bulletId)

  const result = service.traceChain(perspId)
  expect(result.ok).toBe(true)
  if (!result.ok) return
  expect(result.data.source.id).toBe(srcId)
})
```

---

### Task 11.9: Update ReviewService

**Files:**
- Modify: `packages/core/src/services/review-service.ts`
- Modify: `packages/core/src/services/__tests__/review-service.test.ts`

**Goal:** Update source title JOINs to go through `bullet_sources` junction.

- [ ] **Update pending bullets query.**

```sql
-- Before:
SELECT b.*, s.title AS source_title
FROM bullets b
JOIN sources s ON b.source_id = s.id
WHERE b.status = 'pending_review'

-- After:
SELECT b.*, s.title AS source_title
FROM bullets b
JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
JOIN sources s ON bs.source_id = s.id
WHERE b.status = 'pending_review'
ORDER BY b.created_at DESC
```

- [ ] **Update pending perspectives query.**

```sql
-- Before:
SELECT p.*, b.content AS bullet_content, s.title AS source_title
FROM perspectives p
JOIN bullets b ON p.bullet_id = b.id
JOIN sources s ON b.source_id = s.id
WHERE p.status = 'pending_review'

-- After:
SELECT p.*, b.content AS bullet_content, s.title AS source_title
FROM perspectives p
JOIN bullets b ON p.bullet_id = b.id
JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
JOIN sources s ON bs.source_id = s.id
WHERE p.status = 'pending_review'
ORDER BY p.created_at DESC
```

- [ ] **Update `BulletReviewRow`.** Remove `source_id` field.

- [ ] **Update the mapping in `getPendingReview`.** Remove `source_id` from the bullet review item construction.

- [ ] **Update tests.** Seed bullets with junction:

```typescript
test('getPendingReview returns bullets with source title from junction', () => {
  const srcId = seedSource(db, { title: 'Cloud Migration' })
  seedBullet(db, [{ id: srcId }], { status: 'pending_review' })

  const result = service.getPendingReview()
  expect(result.ok).toBe(true)
  if (!result.ok) return
  expect(result.data.bullets.items[0].source_title).toBe('Cloud Migration')
})
```

---

### Task 11.10: Update ResumeService

**Files:**
- Modify: `packages/core/src/services/resume-service.ts`
- Modify: `packages/core/src/services/__tests__/resume-service.test.ts`

**Goal:** Replace perspective-based methods with entry-based methods. Update gap analysis to use junction.

- [ ] **Replace `addPerspective` with `addEntry`.**

```typescript
addEntry(resumeId: string, input: AddResumeEntry): Result<ResumeEntry> {
  const resume = ResumeRepository.get(this.db, resumeId)
  if (!resume) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
  }

  const perspective = PerspectiveRepository.get(this.db, input.perspective_id)
  if (!perspective) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Perspective ${input.perspective_id} not found` } }
  }
  if (perspective.status !== 'approved') {
    return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Only approved perspectives can be added to resumes' } }
  }

  try {
    const entry = ResumeRepository.addEntry(this.db, resumeId, input)
    return { ok: true, data: entry }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err)
    if (message.includes('UNIQUE constraint')) {
      return { ok: false, error: { code: 'CONFLICT', message: 'Perspective already in this resume' } }
    }
    throw err
  }
}
```

- [ ] **Replace `removePerspective` with `removeEntry`.**

- [ ] **Replace `reorderPerspectives` with `reorderEntries`.**

- [ ] **Update `getResume` to return `ResumeWithEntries`.**

```typescript
getResume(id: string): Result<ResumeWithEntries> {
  const resume = ResumeRepository.getWithEntries(this.db, id)
  if (!resume) {
    return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${id} not found` } }
  }
  return { ok: true, data: resume }
}
```

- [ ] **Update `analyzeGaps` — source title via junction.**

```typescript
// Replace:
private getSourceTitle(sourceId: string): string {
  const row = this.db
    .query('SELECT title FROM sources WHERE id = ?')
    .get(sourceId) as { title: string } | null
  return row?.title ?? 'Unknown Source'
}

// With:
private getSourceTitleForBullet(bulletId: string): string {
  const row = this.db
    .query(
      `SELECT s.title FROM sources s
       JOIN bullet_sources bs ON s.id = bs.source_id
       WHERE bs.bullet_id = ? AND bs.is_primary = 1`,
    )
    .get(bulletId) as { title: string } | null
  return row?.title ?? 'Unknown Source'
}
```

- [ ] **Update `findBulletsForGap` query** to use junction instead of `b.source_id`:

```sql
SELECT b.id, b.content, s.title AS source_title
FROM bullets b
JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
JOIN sources s ON bs.source_id = s.id
WHERE b.status = 'approved'
AND b.id NOT IN (
  SELECT p.bullet_id FROM perspectives p
  WHERE p.target_archetype = ?
  AND p.domain = ?
  AND p.status = 'approved'
)
```

- [ ] **Update gap analysis unused bullets loop** to call `getSourceTitleForBullet(bullet.id)` instead of `getSourceTitle(bullet.source_id)`.

- [ ] **Write tests.**

---

### Task 11.11: New OrganizationService

**Files:**
- Create: `packages/core/src/services/organization-service.ts`
- Create: `packages/core/src/services/__tests__/organization-service.test.ts`

**Goal:** CRUD service for organizations with validation (name required, org_type valid).

- [ ] **Create service class.**

```typescript
import type { Database } from 'bun:sqlite'
import type { Organization, Result, PaginatedResult } from '../types'
import * as OrgRepo from '../db/repositories/organization-repository'

const VALID_ORG_TYPES = ['company', 'nonprofit', 'government', 'military', 'education', 'volunteer', 'freelance', 'other']

export class OrganizationService {
  constructor(private db: Database) {}

  create(input: OrgRepo.CreateOrganizationInput): Result<Organization> {
    if (!input.name || input.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }
    if (input.org_type && !VALID_ORG_TYPES.includes(input.org_type)) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Invalid org_type: ${input.org_type}. Must be one of: ${VALID_ORG_TYPES.join(', ')}` } }
    }
    const org = OrgRepo.create(this.db, input)
    return { ok: true, data: org }
  }

  get(id: string): Result<Organization> {
    const org = OrgRepo.get(this.db, id)
    if (!org) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Organization ${id} not found` } }
    }
    return { ok: true, data: org }
  }

  list(filter?: OrgRepo.OrganizationFilter, offset?: number, limit?: number): PaginatedResult<Organization> {
    const result = OrgRepo.list(this.db, filter, offset, limit)
    return { ok: true, data: result.data, pagination: { total: result.total, offset: offset ?? 0, limit: limit ?? 50 } }
  }

  update(id: string, input: Partial<OrgRepo.CreateOrganizationInput>): Result<Organization> {
    if (input.name !== undefined && input.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }
    if (input.org_type && !VALID_ORG_TYPES.includes(input.org_type)) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Invalid org_type: ${input.org_type}` } }
    }
    const org = OrgRepo.update(this.db, id, input)
    if (!org) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Organization ${id} not found` } }
    }
    return { ok: true, data: org }
  }

  delete(id: string): Result<void> {
    const deleted = OrgRepo.del(this.db, id)
    if (!deleted) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Organization ${id} not found` } }
    }
    return { ok: true, data: undefined }
  }
}
```

- [ ] **Write tests.**

```typescript
// packages/core/src/services/__tests__/organization-service.test.ts
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { OrganizationService } from '../organization-service'
import { createTestDb } from '../../db/__tests__/helpers'

describe('OrganizationService', () => {
  let db: Database
  let service: OrganizationService

  beforeEach(() => {
    db = createTestDb()
    service = new OrganizationService(db)
  })
  afterEach(() => db.close())

  test('create with valid name succeeds', () => {
    const result = service.create({ name: 'Anthropic', org_type: 'company' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.name).toBe('Anthropic')
    expect(result.data.org_type).toBe('company')
  })

  test('create with empty name fails validation', () => {
    const result = service.create({ name: '' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('create with invalid org_type fails validation', () => {
    const result = service.create({ name: 'Test', org_type: 'invalid_type' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('get returns NOT_FOUND for missing org', () => {
    const result = service.get('nonexistent-id')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  test('get returns existing org', () => {
    const created = service.create({ name: 'Test' })
    if (!created.ok) return
    const result = service.get(created.data.id)
    expect(result.ok).toBe(true)
  })

  test('list returns all orgs', () => {
    service.create({ name: 'Org A' })
    service.create({ name: 'Org B' })
    const result = service.list()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(2)
  })

  test('update with valid input succeeds', () => {
    const created = service.create({ name: 'Old Name' })
    if (!created.ok) return
    const result = service.update(created.data.id, { name: 'New Name' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.name).toBe('New Name')
  })

  test('delete removes org', () => {
    const created = service.create({ name: 'To Delete' })
    if (!created.ok) return
    const result = service.delete(created.data.id)
    expect(result.ok).toBe(true)
    const check = service.get(created.data.id)
    expect(check.ok).toBe(false)
  })
})
```

**Acceptance Criteria:**
- [ ] Service validates `name` is non-empty on create and update
- [ ] Service validates `org_type` is in the allowed list
- [ ] CRUD operations delegate to `OrganizationRepository`
- [ ] NOT_FOUND returned for get/update/delete of nonexistent ID
- [ ] All tests pass

---

### Task 11.12: New NoteService

**Files:**
- Create: `packages/core/src/services/note-service.ts`
- Create: `packages/core/src/services/__tests__/note-service.test.ts`

**Goal:** CRUD service for user notes + reference linking/unlinking. `getByEntity(entityType, entityId)` finds notes linked to an entity.

- [ ] **Create service class.**

```typescript
import type { Database } from 'bun:sqlite'
import type { UserNote, UserNoteWithReferences, Result, PaginatedResult } from '../types'
import * as NoteRepo from '../db/repositories/note-repository'

const VALID_ENTITY_TYPES = ['source', 'bullet', 'perspective', 'resume_entry', 'resume', 'skill', 'organization']

export class NoteService {
  constructor(private db: Database) {}

  create(input: { title?: string; content: string }): Result<UserNote> {
    if (!input.content || input.content.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Content must not be empty' } }
    }
    const note = NoteRepo.create(this.db, input)
    return { ok: true, data: note }
  }

  get(id: string): Result<UserNoteWithReferences> {
    const note = NoteRepo.getWithReferences(this.db, id)
    if (!note) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Note ${id} not found` } }
    }
    return { ok: true, data: note }
  }

  list(search?: string, offset?: number, limit?: number): PaginatedResult<UserNote> {
    const result = NoteRepo.list(this.db, search, offset, limit)
    return { ok: true, data: result.data, pagination: { total: result.total, offset: offset ?? 0, limit: limit ?? 50 } }
  }

  update(id: string, input: { title?: string; content?: string }): Result<UserNote> {
    if (input.content !== undefined && input.content.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Content must not be empty' } }
    }
    const note = NoteRepo.update(this.db, id, input)
    if (!note) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Note ${id} not found` } }
    }
    return { ok: true, data: note }
  }

  delete(id: string): Result<void> {
    const deleted = NoteRepo.del(this.db, id)
    if (!deleted) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Note ${id} not found` } }
    }
    return { ok: true, data: undefined }
  }

  addReference(noteId: string, entityType: string, entityId: string): Result<void> {
    if (!VALID_ENTITY_TYPES.includes(entityType)) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Invalid entity_type: ${entityType}. Must be one of: ${VALID_ENTITY_TYPES.join(', ')}` } }
    }
    const note = NoteRepo.get(this.db, noteId)
    if (!note) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Note ${noteId} not found` } }
    }
    NoteRepo.addReference(this.db, noteId, entityType, entityId)
    return { ok: true, data: undefined }
  }

  removeReference(noteId: string, entityType: string, entityId: string): Result<void> {
    const removed = NoteRepo.removeReference(this.db, noteId, entityType, entityId)
    if (!removed) {
      return { ok: false, error: { code: 'NOT_FOUND', message: 'Reference not found' } }
    }
    return { ok: true, data: undefined }
  }

  getNotesForEntity(entityType: string, entityId: string): Result<UserNote[]> {
    if (!VALID_ENTITY_TYPES.includes(entityType)) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Invalid entity_type: ${entityType}` } }
    }
    const notes = NoteRepo.getByEntity(this.db, entityType, entityId)
    return { ok: true, data: notes }
  }
}
```

- [ ] **Validation rules:**
  - `content` must not be empty on create, and if provided on update
  - `entityType` must be one of: `'source', 'bullet', 'perspective', 'resume_entry', 'resume', 'skill', 'organization'`

- [ ] **Write tests.**

```typescript
// packages/core/src/services/__tests__/note-service.test.ts
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { NoteService } from '../note-service'
import { createTestDb, seedSource } from '../../db/__tests__/helpers'

describe('NoteService', () => {
  let db: Database
  let service: NoteService

  beforeEach(() => {
    db = createTestDb()
    service = new NoteService(db)
  })
  afterEach(() => db.close())

  test('create with valid content succeeds', () => {
    const result = service.create({ content: 'Remember to mention K8s' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.content).toBe('Remember to mention K8s')
  })

  test('create with empty content fails validation', () => {
    const result = service.create({ content: '' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('get returns note with references', () => {
    const created = service.create({ title: 'Prep', content: 'Notes here' })
    if (!created.ok) return
    const result = service.get(created.data.id)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.references).toEqual([])
  })

  test('list searches by content', () => {
    service.create({ content: 'Kubernetes deployment' })
    service.create({ content: 'Python scripting' })
    const result = service.list('kubernetes')
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(1)
  })

  test('addReference links note to entity', () => {
    const created = service.create({ content: 'Related to source' })
    if (!created.ok) return
    const sourceId = seedSource(db)

    const result = service.addReference(created.data.id, 'source', sourceId)
    expect(result.ok).toBe(true)

    const fetched = service.get(created.data.id)
    if (!fetched.ok) return
    expect(fetched.data.references.length).toBe(1)
    expect(fetched.data.references[0].entity_type).toBe('source')
  })

  test('addReference with invalid entity_type fails validation', () => {
    const created = service.create({ content: 'test' })
    if (!created.ok) return
    const result = service.addReference(created.data.id, 'invalid_type', 'some-id')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('removeReference unlinks note from entity', () => {
    const created = service.create({ content: 'test' })
    if (!created.ok) return
    const sourceId = seedSource(db)
    service.addReference(created.data.id, 'source', sourceId)

    const result = service.removeReference(created.data.id, 'source', sourceId)
    expect(result.ok).toBe(true)

    const fetched = service.get(created.data.id)
    if (!fetched.ok) return
    expect(fetched.data.references.length).toBe(0)
  })

  test('getNotesForEntity finds notes linked to a specific entity', () => {
    const sourceId = seedSource(db)
    const n1 = service.create({ content: 'Note about source' })
    const n2 = service.create({ content: 'Unrelated note' })
    if (!n1.ok || !n2.ok) return
    service.addReference(n1.data.id, 'source', sourceId)

    const result = service.getNotesForEntity('source', sourceId)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.length).toBe(1)
    expect(result.data[0].id).toBe(n1.data.id)
  })
})
```

**Acceptance Criteria:**
- [ ] Service validates content is non-empty on create/update
- [ ] Service validates entityType is in the allowed list for addReference/getNotesForEntity
- [ ] `getByEntity(entityType, entityId)` returns all notes linked to a specific entity
- [ ] CRUD operations delegate to `NoteRepository`
- [ ] Reference linking/unlinking works correctly
- [ ] All tests pass

---

### Task 11.13: New IntegrityService

**Files:**
- Create: `packages/core/src/services/integrity-service.ts`
- Create: `packages/core/src/services/__tests__/integrity-service.test.ts`

**Goal:** Scan all bullets and perspectives for stale content snapshots (content drift). `getDriftedEntities()` returns an array of `{ entity_type, entity_id, snapshot_field, snapshot_value, current_value }` objects.

**Key queries:**

Bullet drift detection (source_content_snapshot vs primary source description):
```sql
SELECT b.id AS bullet_id, b.source_content_snapshot, s.description AS current_description
FROM bullets b
JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
JOIN sources s ON bs.source_id = s.id
WHERE b.source_content_snapshot != s.description
```

Perspective drift detection (bullet_content_snapshot vs bullet content):
```sql
SELECT p.id AS perspective_id, p.bullet_content_snapshot, b.content AS current_content
FROM perspectives p
JOIN bullets b ON p.bullet_id = b.id
WHERE p.bullet_content_snapshot != b.content
```

- [ ] **Create service class.**

```typescript
export interface DriftedEntity {
  entity_type: 'bullet' | 'perspective'
  entity_id: string
  snapshot_field: string
  snapshot_value: string
  current_value: string
}

export class IntegrityService {
  constructor(private db: Database) {}

  /**
   * Find all entities with stale snapshots.
   *
   * Bullets: source_content_snapshot != primary source's description
   * Perspectives: bullet_content_snapshot != bullet's content
   */
  getDriftedEntities(): Result<DriftedEntity[]> {
    const drifted: DriftedEntity[] = []

    // Check bullets against primary source
    const bulletDrifts = this.db
      .query(
        `SELECT b.id AS bullet_id, b.source_content_snapshot, s.description AS current_description
         FROM bullets b
         JOIN bullet_sources bs ON b.id = bs.bullet_id AND bs.is_primary = 1
         JOIN sources s ON bs.source_id = s.id
         WHERE b.source_content_snapshot != s.description`,
      )
      .all() as Array<{ bullet_id: string; source_content_snapshot: string; current_description: string }>

    for (const row of bulletDrifts) {
      drifted.push({
        entity_type: 'bullet',
        entity_id: row.bullet_id,
        snapshot_field: 'source_content_snapshot',
        snapshot_value: row.source_content_snapshot,
        current_value: row.current_description,
      })
    }

    // Check perspectives against bullet content
    const perspectiveDrifts = this.db
      .query(
        `SELECT p.id AS perspective_id, p.bullet_content_snapshot, b.content AS current_content
         FROM perspectives p
         JOIN bullets b ON p.bullet_id = b.id
         WHERE p.bullet_content_snapshot != b.content`,
      )
      .all() as Array<{ perspective_id: string; bullet_content_snapshot: string; current_content: string }>

    for (const row of perspectiveDrifts) {
      drifted.push({
        entity_type: 'perspective',
        entity_id: row.perspective_id,
        snapshot_field: 'bullet_content_snapshot',
        snapshot_value: row.bullet_content_snapshot,
        current_value: row.current_content,
      })
    }

    return { ok: true, data: drifted }
  }
}
```

- [ ] **Write tests.**

```typescript
test('getDriftedEntities returns empty when all snapshots match', () => {
  const srcId = seedSource(db, { description: 'Original description' })
  seedBullet(db, [{ id: srcId }], { content: 'Bullet content' })
  // source_content_snapshot matches source description

  const result = service.getDriftedEntities()
  expect(result.ok).toBe(true)
  if (!result.ok) return
  expect(result.data).toHaveLength(0)
})

test('getDriftedEntities detects bullet with stale source snapshot', () => {
  const srcId = seedSource(db, { description: 'Original description' })
  const bulletId = seedBullet(db, [{ id: srcId }])

  // Update source description (drift!)
  db.run("UPDATE sources SET description = 'Updated description' WHERE id = ?", [srcId])

  const result = service.getDriftedEntities()
  expect(result.ok).toBe(true)
  if (!result.ok) return

  const bulletDrifts = result.data.filter(d => d.entity_type === 'bullet')
  expect(bulletDrifts).toHaveLength(1)
  expect(bulletDrifts[0].entity_id).toBe(bulletId)
  expect(bulletDrifts[0].snapshot_value).toContain('snapshot')  // from seedBullet default
  expect(bulletDrifts[0].current_value).toBe('Updated description')
})

test('getDriftedEntities detects perspective with stale bullet snapshot', () => {
  const srcId = seedSource(db)
  const bulletId = seedBullet(db, [{ id: srcId }])
  const perspId = seedPerspective(db, bulletId)

  // Update bullet content (drift!)
  db.run("UPDATE bullets SET content = 'Updated bullet' WHERE id = ?", [bulletId])

  const result = service.getDriftedEntities()
  expect(result.ok).toBe(true)
  if (!result.ok) return

  const perspDrifts = result.data.filter(d => d.entity_type === 'perspective')
  expect(perspDrifts).toHaveLength(1)
  expect(perspDrifts[0].entity_id).toBe(perspId)
})
```

---

### Task 11.14: Update createServices factory + cleanup

**Files:**
- Modify: `packages/core/src/services/index.ts`
- Delete: `packages/core/src/db/repositories/employer-repository.ts`
- Delete: `packages/core/src/db/repositories/project-repository.ts`
- Modify: `packages/core/src/db/repositories/__tests__/supporting-repositories.test.ts` (remove employer/project tests)
- Modify: `packages/core/src/index.ts` (update exports)

**Goal:** Add `organizations: new OrganizationService(db)`, `notes: new NoteService(db)`, `integrity: new IntegrityService(db)` to the factory. Update `Services` interface. Delete deprecated files. Remove all `test.skip` markers added in Phase 10 T10.3. Run full test suite: `bun test` -- ALL tests must pass.

- [ ] **Update `Services` interface.**

```typescript
export interface Services {
  sources: SourceService
  bullets: BulletService
  perspectives: PerspectiveService
  derivation: DerivationService
  resumes: ResumeService
  audit: AuditService
  review: ReviewService
  organizations: OrganizationService
  notes: NoteService
  integrity: IntegrityService
}
```

- [ ] **Update `createServices` function.**

```typescript
import { OrganizationService } from './organization-service'
import { NoteService } from './note-service'
import { IntegrityService } from './integrity-service'

export function createServices(db: Database): Services {
  const derivingBullets = new Set<string>()

  return {
    sources: new SourceService(db),
    bullets: new BulletService(db),
    perspectives: new PerspectiveService(db),
    derivation: new DerivationService(db, derivingBullets),
    resumes: new ResumeService(db),
    audit: new AuditService(db),
    review: new ReviewService(db),
    organizations: new OrganizationService(db),
    notes: new NoteService(db),
    integrity: new IntegrityService(db),
  }
}
```

- [ ] **Add re-exports.**

```typescript
export { OrganizationService } from './organization-service'
export { NoteService } from './note-service'
export { IntegrityService } from './integrity-service'
```

- [ ] **Delete deprecated repository files.**

```bash
rm packages/core/src/db/repositories/employer-repository.ts
rm packages/core/src/db/repositories/project-repository.ts
```

- [ ] **Update `packages/core/src/index.ts`** if it re-exports employer/project repositories -- remove those exports, add organization/note repository exports.

- [ ] **Run full test suite:**

```bash
cd /Users/adam/notes/job-hunting
bun test
```

- [ ] **Remove all `test.skip` / `describe.skip` markers** that were added in Phase 10 (T10.3). Every test should now pass.

---

## Testing Requirements

- **Unit tests for each repository:** Create, get, get-null, list, list-with-filters, update, update-null, delete, delete-not-found. Each task specifies its exact test cases above.
- **Unit tests for each service:** Validation errors (empty name, invalid status), happy paths, not-found cases, conflict cases.
- **Integration tests:** Full derivation chain with junction: source -> derive bullets -> verify bullet_sources -> derive perspectives -> verify chain trace.
- **Fixtures:** All test data created via `helpers.ts` seed functions (updated in Phase 10). No file-based fixtures.
- **Coverage target:** >80% line coverage for each new repository and service file. Verify with `bun test --coverage`.

## Documentation Requirements

- JSDoc on all public functions in new repository/service files
- Inline comments on complex JOIN queries explaining the junction pattern
- Update any in-code references to "employer" or "project" that should now say "organization" or "source"
- No markdown documentation files to create

## Acceptance Criteria

- [ ] `EmployerRepository` and `ProjectRepository` files are deleted
- [ ] `SourceRepository` handles polymorphic source creation (base + extension in transaction)
- [ ] `SourceRepository.list()` filters by `source_type` and `organization_id` (via JOIN)
- [ ] `BulletRepository` no longer references `source_id` column
- [ ] `BulletRepository.create()` inserts `bullet_sources` junction rows
- [ ] `BulletRepository.list()` filters by `source_id` via `bullet_sources` JOIN
- [ ] `BulletRepository.getSources()` and `getPrimarySource()` work correctly
- [ ] `ResumeRepository` uses `resume_entries` instead of `resume_perspectives`
- [ ] `ResumeRepository.addEntry()` supports copy-on-write via content field
- [ ] `OrganizationRepository` has full CRUD with `org_type` and `worked` filters
- [ ] `NoteRepository` has CRUD + reference linking + entity lookup
- [ ] `ResumeEntryRepository` has CRUD + `resolveContent()` with COALESCE
- [ ] `DerivationService` creates `bullet_sources` row with `is_primary=1`
- [ ] `AuditService` traces chain via `bullet_sources` junction
- [ ] `ReviewService` JOINs source title via `bullet_sources`
- [ ] `ResumeService` uses entry methods instead of perspective methods
- [ ] `IntegrityService.getDriftedEntities()` finds stale snapshots via junction
- [ ] `createServices()` returns all new services
- [ ] `bun test` has zero failures -- all tests pass, no skipped tests remain
- [ ] No TypeScript compilation errors (`bunx tsc --noEmit`)

## Failure Criteria

- **Junction JOIN returns wrong data** -- Check: `AND bs.is_primary = 1` is present in all queries that need the primary source. Without it, bullets with multiple sources return duplicate rows.
- **DerivationService transaction doesn't include bullet_sources** -- Data integrity issue: bullets exist without source linkage. Verify with: `SELECT b.id FROM bullets b WHERE b.id NOT IN (SELECT bullet_id FROM bullet_sources)` should return 0 rows after derivation.
- **Copy-on-write snapshot not captured** -- When `updateEntry` sets content, it must also set `perspective_content_snapshot`. Without it, there's no way to detect content drift for cloned entries.
- **Gap analysis query breaks** -- The `findBulletsForGap` query uses a correlated subquery that must still work after the source_id removal. The subquery operates on `perspectives.bullet_id` which hasn't changed.
- **Organization filter returns duplicates** -- The `organization_id` filter JOINs two extension tables. Use `DISTINCT` in the query to avoid duplicates when a source has both a role and project extension (shouldn't happen, but defensive).
- **Cascade delete breaks** -- Deleting a source should CASCADE to `source_roles`/`source_projects`/etc. and CASCADE to `bullet_sources`. Verify FK actions are correct.
