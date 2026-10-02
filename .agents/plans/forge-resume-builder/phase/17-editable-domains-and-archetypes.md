# Phase 17: Editable Domains & Archetypes

**Goal:** Replace hardcoded domain/archetype constants with DB-backed entities. Full CRUD repositories, services, routes, SDK resources, and WebUI views for both domains and archetypes, including archetype-domain association management. Update `ResumeService.analyzeGaps` and `DerivationService` to validate against DB rows instead of constant arrays.

**Architecture:** Two new repository/service/route stacks follow the established pattern (`OrganizationRepository` / `OrganizationService` / `organizationRoutes`). The `archetype_domains` junction table is managed through archetype endpoints. Gap analysis queries the junction instead of the `ARCHETYPE_EXPECTED_DOMAINS` constant. Constants file is trimmed to only `THIN_COVERAGE_THRESHOLD` and `RESUME_SECTIONS`.

**Tech Stack:** SQLite (via Bun), TypeScript, Hono, `bun:test`, SvelteKit (WebUI), `@forge/sdk`

**Depends on:** Phase 16 (schema migration with `domains`, `archetypes`, `archetype_domains` tables and seed data)
**Blocks:** Nothing (independent track)
**Parallelizable with:** Phase 18 (organization updates), Phase 19 (resume IR + compilers), Phase 20 (resume renderer views)

**Shared file warning:** Phase 17 (T17.4) and Phase 19 (T19.9, T19.10) both modify `packages/core/src/services/resume-service.ts`. Execute sequentially or in separate branches and merge carefully. Phase 17 and Phase 18 both modify `packages/sdk/src/types.ts` -- if running in parallel, coordinate or merge carefully.

**Reference:** `refs/specs/2026-03-29-resume-renderer-and-entity-updates.md` section 2 (Editable Domains & Archetypes), section 2.2-2.4 (Views, API)

---

## Context

The codebase currently has:
- Hardcoded `ARCHETYPES`, `DOMAINS`, `ARCHETYPE_EXPECTED_DOMAINS` constants in `packages/core/src/constants/archetypes.ts`
- `ResumeService.analyzeGaps` reads `ARCHETYPE_EXPECTED_DOMAINS[resume.archetype]` to get expected domains
- `DerivationService.derivePerspectivesFromBullet` accepts `params.archetype` and `params.domain` as free-form strings (no validation against known values)
- `PerspectiveRepository` stores `target_archetype` and `domain` as TEXT columns referencing names (not IDs)
- Phase 16 created `domains`, `archetypes`, `archetype_domains` tables with seed data matching the constants
- SDK has `Domain`, `Archetype`, `ArchetypeDomain` types (added in Phase 16)
- Established patterns: `OrganizationRepository` (namespace export), `OrganizationService` (class), `organizationRoutes` (Hono factory), `OrganizationsResource` (SDK class)

After this phase:
- `ARCHETYPES`, `DOMAINS`, `ARCHETYPE_EXPECTED_DOMAINS` constants are removed
- `THIN_COVERAGE_THRESHOLD` and `RESUME_SECTIONS` remain as constants
- `RESUME_SECTIONS` is updated to include `experience`, `certifications`, `clearance`, `presentations`, `custom`
- Full CRUD for domains and archetypes via API
- Domain associations managed through archetype endpoints
- Gap analysis queries DB instead of constants
- DerivationService validates archetype/domain against DB
- WebUI has `/domains` view and updated `/archetypes` view

## Goals

- Create `DomainRepository` with CRUD and usage-count queries
- Create `ArchetypeRepository` with CRUD and domain association management
- Create `DomainService` and `ArchetypeService` with validation and delete-protection
- Create domain and archetype routes, register in `server.ts`
- Update `ResumeService.analyzeGaps` to query `archetype_domains` instead of constants
- Update `DerivationService` to validate archetype/domain against DB
- Add `DomainsResource` and `ArchetypesResource` to the SDK
- Build Domains view (`/domains`) and update Archetypes view (`/archetypes`) in WebUI
- Remove deprecated constants, update `RESUME_SECTIONS`

## Non-Goals

- Changing the text-based references in `perspectives.target_archetype` and `perspectives.domain` to foreign key IDs (names remain as text)
- Adding archetype/domain filtering to the perspectives list endpoint (existing filter works)
- Building admin tools for bulk archetype/domain reassignment
- Migrating existing perspective rows if domain/archetype names change (the UI should prevent renaming seed values)

## Fallback Strategies

- If delete-protection queries are too slow, add indexes on `perspectives(target_archetype)` and `perspectives(domain)`
- If the WebUI tag picker is complex, start with a simple checkbox list for domain associations
- If removing constants breaks tests that import them directly, update test imports to use DB queries or keep a `TEST_ARCHETYPES` / `TEST_DOMAINS` constant in the test helpers file only

---

## Tasks

### Task 17.1: DomainRepository + DomainService

**Files:**
- Create: `packages/core/src/db/repositories/domain-repository.ts`
- Create: `packages/core/src/services/domain-service.ts`

**Goal:** CRUD for the `domains` table with delete-protection when referenced by `perspectives` or `archetype_domains`.

#### DomainRepository

- [ ] Create `packages/core/src/db/repositories/domain-repository.ts` as a namespace export (matching `organization-repository.ts` pattern):

```typescript
/**
 * DomainRepository — pure data access for the domains table.
 */

import type { Database } from 'bun:sqlite'
import type { Domain } from '../../types'

interface DomainRow {
  id: string
  name: string
  description: string | null
  created_at: string
}

interface DomainWithUsage extends Domain {
  perspective_count: number
  archetype_count: number
}

export interface CreateDomainInput {
  name: string
  description?: string
}

function rowToDomain(row: DomainRow): Domain {
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    created_at: row.created_at,
  }
}

export function create(db: Database, input: CreateDomainInput): Domain {
  const id = crypto.randomUUID()
  const row = db
    .query(
      `INSERT INTO domains (id, name, description)
       VALUES (?, ?, ?)
       RETURNING *`,
    )
    .get(id, input.name, input.description ?? null) as DomainRow

  return rowToDomain(row)
}

export function get(db: Database, id: string): Domain | null {
  const row = db
    .query('SELECT * FROM domains WHERE id = ?')
    .get(id) as DomainRow | null

  return row ? rowToDomain(row) : null
}

export function getByName(db: Database, name: string): Domain | null {
  const row = db
    .query('SELECT * FROM domains WHERE name = ?')
    .get(name) as DomainRow | null

  return row ? rowToDomain(row) : null
}

export function list(
  db: Database,
  offset = 0,
  limit = 50,
): { data: DomainWithUsage[]; total: number } {
  const countRow = db
    .query('SELECT COUNT(*) AS total FROM domains')
    .get() as { total: number }

  const rows = db
    .query(
      `SELECT d.*,
              (SELECT COUNT(*) FROM perspectives p WHERE p.domain = d.name) AS perspective_count,
              (SELECT COUNT(*) FROM archetype_domains ad WHERE ad.domain_id = d.id) AS archetype_count
       FROM domains d
       ORDER BY d.name ASC
       LIMIT ? OFFSET ?`,
    )
    .all(limit, offset) as Array<DomainRow & { perspective_count: number; archetype_count: number }>

  return {
    data: rows.map((row) => ({
      ...rowToDomain(row),
      perspective_count: row.perspective_count,
      archetype_count: row.archetype_count,
    })),
    total: countRow.total,
  }
}

export function update(
  db: Database,
  id: string,
  input: Partial<CreateDomainInput>,
): Domain | null {
  const existing = get(db, id)
  if (!existing) return null

  const sets: string[] = []
  const params: unknown[] = []

  if (input.name !== undefined) {
    sets.push('name = ?')
    params.push(input.name)
  }
  if (input.description !== undefined) {
    sets.push('description = ?')
    params.push(input.description)
  }

  if (sets.length === 0) return existing

  params.push(id)
  const row = db
    .query(`UPDATE domains SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
    .get(...params) as DomainRow | null

  return row ? rowToDomain(row) : null
}

/**
 * Count references to this domain from perspectives and archetype_domains.
 * Used to prevent deletion of referenced domains.
 */
export function countReferences(
  db: Database,
  id: string,
): { perspective_count: number; archetype_count: number } {
  const domain = get(db, id)
  if (!domain) return { perspective_count: 0, archetype_count: 0 }

  const perspCount = db
    .query('SELECT COUNT(*) AS c FROM perspectives WHERE domain = ?')
    .get(domain.name) as { c: number }

  const archCount = db
    .query('SELECT COUNT(*) AS c FROM archetype_domains WHERE domain_id = ?')
    .get(id) as { c: number }

  return {
    perspective_count: perspCount.c,
    archetype_count: archCount.c,
  }
}

export function del(db: Database, id: string): boolean {
  const result = db.run('DELETE FROM domains WHERE id = ?', [id])
  return result.changes > 0
}
```

#### DomainService

- [ ] Create `packages/core/src/services/domain-service.ts`:

```typescript
/**
 * DomainService — business logic for domain entities.
 *
 * Validates input, enforces delete-protection when referenced
 * by perspectives or archetype_domains.
 */

import type { Database } from 'bun:sqlite'
import type { Domain, Result, PaginatedResult } from '../types'
import * as DomainRepo from '../db/repositories/domain-repository'

export class DomainService {
  constructor(private db: Database) {}

  create(input: DomainRepo.CreateDomainInput): Result<Domain> {
    if (!input.name || input.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }
    // Validate name format: lowercase, underscores, no spaces
    if (!/^[a-z][a-z0-9_]*$/.test(input.name)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Domain name must be lowercase, start with a letter, and contain only letters, digits, and underscores',
        },
      }
    }
    try {
      const domain = DomainRepo.create(this.db, input)
      return { ok: true, data: domain }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err)
      if (message.includes('UNIQUE constraint')) {
        return { ok: false, error: { code: 'CONFLICT', message: `Domain '${input.name}' already exists` } }
      }
      throw err
    }
  }

  get(id: string): Result<Domain> {
    const domain = DomainRepo.get(this.db, id)
    if (!domain) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Domain ${id} not found` } }
    }
    return { ok: true, data: domain }
  }

  list(offset?: number, limit?: number): PaginatedResult<Domain> {
    const result = DomainRepo.list(this.db, offset, limit)
    return {
      ok: true,
      data: result.data,
      pagination: { total: result.total, offset: offset ?? 0, limit: limit ?? 50 },
    }
  }

  update(id: string, input: Partial<DomainRepo.CreateDomainInput>): Result<Domain> {
    if (input.name !== undefined && input.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }
    if (input.name !== undefined && !/^[a-z][a-z0-9_]*$/.test(input.name)) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Domain name must be lowercase with underscores only' },
      }
    }

    try {
      const domain = DomainRepo.update(this.db, id, input)
      if (!domain) {
        return { ok: false, error: { code: 'NOT_FOUND', message: `Domain ${id} not found` } }
      }
      return { ok: true, data: domain }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err)
      if (message.includes('UNIQUE constraint')) {
        return { ok: false, error: { code: 'CONFLICT', message: `Domain '${input.name}' already exists` } }
      }
      throw err
    }
  }

  delete(id: string): Result<void> {
    const domain = DomainRepo.get(this.db, id)
    if (!domain) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Domain ${id} not found` } }
    }

    const refs = DomainRepo.countReferences(this.db, id)
    if (refs.perspective_count > 0) {
      return {
        ok: false,
        error: {
          code: 'CONFLICT',
          message: `Cannot delete domain '${domain.name}': referenced by ${refs.perspective_count} perspective(s)`,
        },
      }
    }
    if (refs.archetype_count > 0) {
      return {
        ok: false,
        error: {
          code: 'CONFLICT',
          message: `Cannot delete domain '${domain.name}': associated with ${refs.archetype_count} archetype(s)`,
        },
      }
    }

    DomainRepo.del(this.db, id)
    return { ok: true, data: undefined }
  }
}
```

**Acceptance Criteria:**
- [ ] `DomainRepository` CRUD operations work against the `domains` table
- [ ] `DomainRepository.list` returns `perspective_count` and `archetype_count` per domain
- [ ] `DomainRepository.countReferences` correctly counts perspectives by name and archetype_domains by ID
- [ ] `DomainService.create` validates name format (lowercase, underscores) and uniqueness
- [ ] `DomainService.delete` blocks deletion when referenced by perspectives or archetype_domains
- [ ] `DomainService.delete` succeeds for unreferenced domains

**Testing:**
- Unit: Create domain, verify ID is UUID and name matches
- Unit: Get domain by ID, verify all fields
- Unit: Get domain by name, verify returned
- Unit: List domains returns usage counts
- Unit: Update domain name and description
- Unit: Update with duplicate name returns CONFLICT
- Unit: Delete unreferenced domain succeeds
- Unit: Delete domain referenced by perspective returns CONFLICT with count
- Unit: Delete domain referenced by archetype_domains returns CONFLICT with count
- Unit: Create with empty name returns VALIDATION_ERROR
- Unit: Create with invalid name format (spaces, uppercase) returns VALIDATION_ERROR

---

### Task 17.2: ArchetypeRepository + ArchetypeService

**Files:**
- Create: `packages/core/src/db/repositories/archetype-repository.ts`
- Create: `packages/core/src/services/archetype-service.ts`

**Goal:** CRUD for the `archetypes` table with domain association management via `archetype_domains` junction. Delete-protection when referenced by `resumes` or `perspectives`.

#### ArchetypeRepository

- [ ] Create `packages/core/src/db/repositories/archetype-repository.ts`:

```typescript
/**
 * ArchetypeRepository — pure data access for archetypes and archetype_domains.
 */

import type { Database } from 'bun:sqlite'
import type { Archetype, Domain, ArchetypeDomain } from '../../types'

interface ArchetypeRow {
  id: string
  name: string
  description: string | null
  created_at: string
}

export interface ArchetypeWithDomains extends Archetype {
  domains: Domain[]
}

export interface ArchetypeWithCounts extends Archetype {
  resume_count: number
  perspective_count: number
  domain_count: number
}

export interface CreateArchetypeInput {
  name: string
  description?: string
}

function rowToArchetype(row: ArchetypeRow): Archetype {
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    created_at: row.created_at,
  }
}

export function create(db: Database, input: CreateArchetypeInput): Archetype {
  const id = crypto.randomUUID()
  const row = db
    .query(
      `INSERT INTO archetypes (id, name, description)
       VALUES (?, ?, ?)
       RETURNING *`,
    )
    .get(id, input.name, input.description ?? null) as ArchetypeRow

  return rowToArchetype(row)
}

export function get(db: Database, id: string): Archetype | null {
  const row = db
    .query('SELECT * FROM archetypes WHERE id = ?')
    .get(id) as ArchetypeRow | null

  return row ? rowToArchetype(row) : null
}

export function getByName(db: Database, name: string): Archetype | null {
  const row = db
    .query('SELECT * FROM archetypes WHERE name = ?')
    .get(name) as ArchetypeRow | null

  return row ? rowToArchetype(row) : null
}

export function getWithDomains(db: Database, id: string): ArchetypeWithDomains | null {
  const archetype = get(db, id)
  if (!archetype) return null

  const domains = db
    .query(
      `SELECT d.* FROM domains d
       JOIN archetype_domains ad ON d.id = ad.domain_id
       WHERE ad.archetype_id = ?
       ORDER BY d.name ASC`,
    )
    .all(id) as Array<{ id: string; name: string; description: string | null; created_at: string }>

  return {
    ...archetype,
    domains: domains.map((d) => ({
      id: d.id,
      name: d.name,
      description: d.description,
      created_at: d.created_at,
    })),
  }
}

export function list(
  db: Database,
  offset = 0,
  limit = 50,
): { data: ArchetypeWithCounts[]; total: number } {
  const countRow = db
    .query('SELECT COUNT(*) AS total FROM archetypes')
    .get() as { total: number }

  const rows = db
    .query(
      `SELECT a.*,
              (SELECT COUNT(*) FROM resumes r WHERE r.archetype = a.name) AS resume_count,
              (SELECT COUNT(*) FROM perspectives p WHERE p.target_archetype = a.name) AS perspective_count,
              (SELECT COUNT(*) FROM archetype_domains ad WHERE ad.archetype_id = a.id) AS domain_count
       FROM archetypes a
       ORDER BY a.name ASC
       LIMIT ? OFFSET ?`,
    )
    .all(limit, offset) as Array<ArchetypeRow & { resume_count: number; perspective_count: number; domain_count: number }>

  return {
    data: rows.map((row) => ({
      ...rowToArchetype(row),
      resume_count: row.resume_count,
      perspective_count: row.perspective_count,
      domain_count: row.domain_count,
    })),
    total: countRow.total,
  }
}

export function update(
  db: Database,
  id: string,
  input: Partial<CreateArchetypeInput>,
): Archetype | null {
  const existing = get(db, id)
  if (!existing) return null

  const sets: string[] = []
  const params: unknown[] = []

  if (input.name !== undefined) {
    sets.push('name = ?')
    params.push(input.name)
  }
  if (input.description !== undefined) {
    sets.push('description = ?')
    params.push(input.description)
  }

  if (sets.length === 0) return existing

  params.push(id)
  const row = db
    .query(`UPDATE archetypes SET ${sets.join(', ')} WHERE id = ? RETURNING *`)
    .get(...params) as ArchetypeRow | null

  return row ? rowToArchetype(row) : null
}

export function countReferences(
  db: Database,
  id: string,
): { resume_count: number; perspective_count: number } {
  const archetype = get(db, id)
  if (!archetype) return { resume_count: 0, perspective_count: 0 }

  const resumeCount = db
    .query('SELECT COUNT(*) AS c FROM resumes WHERE archetype = ?')
    .get(archetype.name) as { c: number }

  const perspCount = db
    .query('SELECT COUNT(*) AS c FROM perspectives WHERE target_archetype = ?')
    .get(archetype.name) as { c: number }

  return {
    resume_count: resumeCount.c,
    perspective_count: perspCount.c,
  }
}

export function del(db: Database, id: string): boolean {
  const result = db.run('DELETE FROM archetypes WHERE id = ?', [id])
  return result.changes > 0
}

// ── Domain association management ────────────────────────────────────

export function addDomain(db: Database, archetypeId: string, domainId: string): void {
  db.run(
    'INSERT INTO archetype_domains (archetype_id, domain_id) VALUES (?, ?)',
    [archetypeId, domainId],
  )
}

export function removeDomain(db: Database, archetypeId: string, domainId: string): boolean {
  const result = db.run(
    'DELETE FROM archetype_domains WHERE archetype_id = ? AND domain_id = ?',
    [archetypeId, domainId],
  )
  return result.changes > 0
}

export function listDomains(db: Database, archetypeId: string): Domain[] {
  return db
    .query(
      `SELECT d.* FROM domains d
       JOIN archetype_domains ad ON d.id = ad.domain_id
       WHERE ad.archetype_id = ?
       ORDER BY d.name ASC`,
    )
    .all(archetypeId) as Domain[]
}

/**
 * Get expected domain names for a given archetype name.
 * This replaces ARCHETYPE_EXPECTED_DOMAINS[archetypeName].
 */
export function getExpectedDomainNames(db: Database, archetypeName: string): string[] {
  const rows = db
    .query(
      `SELECT d.name FROM domains d
       JOIN archetype_domains ad ON d.id = ad.domain_id
       JOIN archetypes a ON a.id = ad.archetype_id
       WHERE a.name = ?`,
    )
    .all(archetypeName) as Array<{ name: string }>

  return rows.map((r) => r.name)
}
```

#### ArchetypeService

- [ ] Create `packages/core/src/services/archetype-service.ts`:

```typescript
/**
 * ArchetypeService — business logic for archetype entities.
 *
 * Manages CRUD and domain associations. Blocks deletion when
 * the archetype is referenced by resumes or perspectives.
 */

import type { Database } from 'bun:sqlite'
import type { Archetype, Domain, Result, PaginatedResult } from '../types'
import * as ArchetypeRepo from '../db/repositories/archetype-repository'
import * as DomainRepo from '../db/repositories/domain-repository'

export class ArchetypeService {
  constructor(private db: Database) {}

  create(input: ArchetypeRepo.CreateArchetypeInput): Result<Archetype> {
    if (!input.name || input.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }
    // Validate name format: lowercase, hyphens allowed (e.g., "agentic-ai")
    if (!/^[a-z][a-z0-9-]*$/.test(input.name)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Archetype name must be lowercase, start with a letter, and contain only letters, digits, and hyphens',
        },
      }
    }
    try {
      const archetype = ArchetypeRepo.create(this.db, input)
      return { ok: true, data: archetype }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err)
      if (message.includes('UNIQUE constraint')) {
        return { ok: false, error: { code: 'CONFLICT', message: `Archetype '${input.name}' already exists` } }
      }
      throw err
    }
  }

  get(id: string): Result<Archetype> {
    const archetype = ArchetypeRepo.get(this.db, id)
    if (!archetype) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Archetype ${id} not found` } }
    }
    return { ok: true, data: archetype }
  }

  getWithDomains(id: string): Result<ArchetypeRepo.ArchetypeWithDomains> {
    const archetype = ArchetypeRepo.getWithDomains(this.db, id)
    if (!archetype) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Archetype ${id} not found` } }
    }
    return { ok: true, data: archetype }
  }

  list(offset?: number, limit?: number): PaginatedResult<Archetype> {
    const result = ArchetypeRepo.list(this.db, offset, limit)
    return {
      ok: true,
      data: result.data,
      pagination: { total: result.total, offset: offset ?? 0, limit: limit ?? 50 },
    }
  }

  update(id: string, input: Partial<ArchetypeRepo.CreateArchetypeInput>): Result<Archetype> {
    if (input.name !== undefined && input.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }
    if (input.name !== undefined && !/^[a-z][a-z0-9-]*$/.test(input.name)) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Archetype name must be lowercase with hyphens only' },
      }
    }

    try {
      const archetype = ArchetypeRepo.update(this.db, id, input)
      if (!archetype) {
        return { ok: false, error: { code: 'NOT_FOUND', message: `Archetype ${id} not found` } }
      }
      return { ok: true, data: archetype }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err)
      if (message.includes('UNIQUE constraint')) {
        return { ok: false, error: { code: 'CONFLICT', message: `Archetype '${input.name}' already exists` } }
      }
      throw err
    }
  }

  delete(id: string): Result<void> {
    const archetype = ArchetypeRepo.get(this.db, id)
    if (!archetype) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Archetype ${id} not found` } }
    }

    const refs = ArchetypeRepo.countReferences(this.db, id)
    if (refs.resume_count > 0) {
      return {
        ok: false,
        error: {
          code: 'CONFLICT',
          message: `Cannot delete archetype '${archetype.name}': referenced by ${refs.resume_count} resume(s)`,
        },
      }
    }
    if (refs.perspective_count > 0) {
      return {
        ok: false,
        error: {
          code: 'CONFLICT',
          message: `Cannot delete archetype '${archetype.name}': referenced by ${refs.perspective_count} perspective(s)`,
        },
      }
    }

    // archetype_domains will cascade-delete
    ArchetypeRepo.del(this.db, id)
    return { ok: true, data: undefined }
  }

  // ── Domain association management ────────────────────────────────

  addDomain(archetypeId: string, domainId: string): Result<void> {
    const archetype = ArchetypeRepo.get(this.db, archetypeId)
    if (!archetype) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Archetype ${archetypeId} not found` } }
    }

    const domain = DomainRepo.get(this.db, domainId)
    if (!domain) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Domain ${domainId} not found` } }
    }

    try {
      ArchetypeRepo.addDomain(this.db, archetypeId, domainId)
      return { ok: true, data: undefined }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err)
      if (message.includes('UNIQUE constraint') || message.includes('PRIMARY KEY')) {
        return { ok: false, error: { code: 'CONFLICT', message: 'Domain already associated with this archetype' } }
      }
      throw err
    }
  }

  removeDomain(archetypeId: string, domainId: string): Result<void> {
    const removed = ArchetypeRepo.removeDomain(this.db, archetypeId, domainId)
    if (!removed) {
      return { ok: false, error: { code: 'NOT_FOUND', message: 'Domain association not found' } }
    }
    return { ok: true, data: undefined }
  }

  listDomains(archetypeId: string): Result<Domain[]> {
    const archetype = ArchetypeRepo.get(this.db, archetypeId)
    if (!archetype) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Archetype ${archetypeId} not found` } }
    }

    const domains = ArchetypeRepo.listDomains(this.db, archetypeId)
    return { ok: true, data: domains }
  }
}
```

**Acceptance Criteria:**
- [ ] `ArchetypeRepository` CRUD works against the `archetypes` table
- [ ] `ArchetypeRepository.getWithDomains` returns archetype with its associated domains
- [ ] `ArchetypeRepository.list` returns `resume_count`, `perspective_count`, and `domain_count` per archetype
- [ ] `ArchetypeRepository.addDomain` / `removeDomain` manage the junction table
- [ ] `ArchetypeRepository.getExpectedDomainNames` returns domain names for an archetype by name (replaces constant lookup)
- [ ] `ArchetypeService.delete` blocks deletion when referenced by resumes or perspectives
- [ ] `ArchetypeService.addDomain` validates both archetype and domain exist
- [ ] `ArchetypeService.addDomain` returns CONFLICT on duplicate association

**Testing:**
- Unit: Create archetype, verify ID and name
- Unit: Get archetype by ID and by name
- Unit: getWithDomains returns associated domains
- Unit: List archetypes returns usage counts
- Unit: Update archetype name and description
- Unit: Duplicate name returns CONFLICT
- Unit: Delete unreferenced archetype succeeds
- Unit: Delete archetype referenced by resume returns CONFLICT
- Unit: Delete archetype referenced by perspective returns CONFLICT
- Unit: Delete archetype cascades archetype_domains rows
- Unit: addDomain creates junction row
- Unit: addDomain with nonexistent domain returns NOT_FOUND
- Unit: addDomain duplicate returns CONFLICT
- Unit: removeDomain removes junction row
- Unit: removeDomain nonexistent returns NOT_FOUND
- Unit: listDomains returns correct domains
- Unit: getExpectedDomainNames returns correct names for seeded archetypes

---

### Task 17.3: Domain + Archetype routes

**Files:**
- Create: `packages/core/src/routes/domains.ts`
- Create: `packages/core/src/routes/archetypes.ts`
- Modify: `packages/core/src/routes/server.ts`
- Modify: `packages/core/src/services/index.ts`

**Goal:** HTTP endpoints for domain and archetype CRUD, including archetype domain association management. Register in the app.

- [ ] Create `packages/core/src/routes/domains.ts`:

```typescript
/**
 * Domain routes — thin HTTP layer over DomainService.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function domainRoutes(services: Services) {
  const app = new Hono()

  app.post('/domains', async (c) => {
    const body = await c.req.json()
    const result = services.domains.create(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/domains', (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const result = services.domains.list(offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/domains/:id', (c) => {
    const result = services.domains.get(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/domains/:id', async (c) => {
    const body = await c.req.json()
    const result = services.domains.update(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/domains/:id', (c) => {
    const result = services.domains.delete(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

- [ ] Create `packages/core/src/routes/archetypes.ts`:

```typescript
/**
 * Archetype routes — CRUD + domain association management.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'

export function archetypeRoutes(services: Services) {
  const app = new Hono()

  app.post('/archetypes', async (c) => {
    const body = await c.req.json()
    const result = services.archetypes.create(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/archetypes', (c) => {
    const offset = Math.max(0, parseInt(c.req.query('offset') ?? '0', 10) || 0)
    const limit = Math.min(200, Math.max(1, parseInt(c.req.query('limit') ?? '50', 10) || 50))
    const result = services.archetypes.list(offset, limit)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data, pagination: result.pagination })
  })

  app.get('/archetypes/:id', (c) => {
    const result = services.archetypes.getWithDomains(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.patch('/archetypes/:id', async (c) => {
    const body = await c.req.json()
    const result = services.archetypes.update(c.req.param('id'), body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/archetypes/:id', (c) => {
    const result = services.archetypes.delete(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  // ── Domain associations ────────────────────────────────────────────

  app.get('/archetypes/:id/domains', (c) => {
    const result = services.archetypes.listDomains(c.req.param('id'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.post('/archetypes/:id/domains', async (c) => {
    const body = await c.req.json<{ domain_id: string }>()
    const result = services.archetypes.addDomain(c.req.param('id'), body.domain_id)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: null }, 201)
  })

  app.delete('/archetypes/:id/domains/:domainId', (c) => {
    const result = services.archetypes.removeDomain(c.req.param('id'), c.req.param('domainId'))
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

- [ ] Update `packages/core/src/services/index.ts` to add `DomainService` and `ArchetypeService`:

```typescript
// Add imports:
import { DomainService } from './domain-service'
import { ArchetypeService } from './archetype-service'

// Add to Services interface:
export interface Services {
  // ... existing ...
  domains: DomainService
  archetypes: ArchetypeService
}

// Add to createServices:
export function createServices(db: Database): Services {
  const derivingBullets = new Set<string>()
  return {
    // ... existing ...
    domains: new DomainService(db),
    archetypes: new ArchetypeService(db),
  }
}

// Add re-exports:
export { DomainService } from './domain-service'
export { ArchetypeService } from './archetype-service'
```

- [ ] Update `packages/core/src/routes/server.ts` to mount new routes:

```typescript
// Add imports:
import { domainRoutes } from './domains'
import { archetypeRoutes } from './archetypes'

// Add route registration (after existing routes):
app.route('/', domainRoutes(services))
app.route('/', archetypeRoutes(services))
```

**Acceptance Criteria:**
- [ ] `POST /api/domains` creates a domain (201)
- [ ] `GET /api/domains` lists domains with pagination and usage counts
- [ ] `GET /api/domains/:id` returns a single domain
- [ ] `PATCH /api/domains/:id` updates a domain
- [ ] `DELETE /api/domains/:id` deletes (204) or returns 409 if referenced
- [ ] `POST /api/archetypes` creates an archetype (201)
- [ ] `GET /api/archetypes` lists archetypes with pagination and usage counts
- [ ] `GET /api/archetypes/:id` returns archetype with associated domains
- [ ] `PATCH /api/archetypes/:id` updates an archetype
- [ ] `DELETE /api/archetypes/:id` deletes (204) or returns 409 if referenced
- [ ] `GET /api/archetypes/:id/domains` lists domains for an archetype
- [ ] `POST /api/archetypes/:id/domains` adds a domain association (201)
- [ ] `DELETE /api/archetypes/:id/domains/:domainId` removes a domain association (204)
- [ ] Routes are registered in `server.ts`
- [ ] Services are registered in `createServices`

**Testing:**
- Contract: POST /api/domains with valid body returns 201
- Contract: POST /api/domains with empty name returns 400
- Contract: GET /api/domains returns array with pagination
- Contract: DELETE /api/domains/:id with references returns 409
- Contract: POST /api/archetypes with valid body returns 201
- Contract: GET /api/archetypes/:id returns domains array
- Contract: POST /api/archetypes/:id/domains with valid domain_id returns 201
- Contract: DELETE /api/archetypes/:id/domains/:domainId returns 204
- Contract: DELETE /api/archetypes/:id with references returns 409

---

### Task 17.4: Update ResumeService.analyzeGaps

**Files:**
- Modify: `packages/core/src/services/resume-service.ts`

**Goal:** Replace the `ARCHETYPE_EXPECTED_DOMAINS[resume.archetype]` constant lookup with a DB query through `ArchetypeRepository.getExpectedDomainNames`.

- [ ] Remove import of `ARCHETYPE_EXPECTED_DOMAINS` from `resume-service.ts`:

```typescript
// Before:
import { ARCHETYPE_EXPECTED_DOMAINS, THIN_COVERAGE_THRESHOLD } from '../constants/archetypes'

// After:
import { THIN_COVERAGE_THRESHOLD } from '../constants/archetypes'
import * as ArchetypeRepo from '../db/repositories/archetype-repository'
```

- [ ] Update `analyzeGaps` to query DB for expected domains:

```typescript
// Before:
const expectedDomains = ARCHETYPE_EXPECTED_DOMAINS[resume.archetype] ?? []

// After:
const expectedDomains = ArchetypeRepo.getExpectedDomainNames(this.db, resume.archetype)
```

No other changes needed. The rest of the method uses `expectedDomains` as `string[]` which is exactly what `getExpectedDomainNames` returns.

**Acceptance Criteria:**
- [ ] `analyzeGaps` returns correct gaps using DB-backed domain mappings
- [ ] Gap analysis for seeded archetypes produces identical results to the previous constant-based approach
- [ ] Gap analysis for a non-existent archetype returns empty expected domains (no crash)

**Testing:**
- Unit: Create resume with archetype 'agentic-ai', run analyzeGaps, verify it checks ai_ml, software_engineering, leadership domains
- Unit: Create resume with non-existent archetype, run analyzeGaps, verify empty expectedDomains and no gaps of type missing_domain_coverage
- Unit: Add a new archetype with custom domains via DB, verify analyzeGaps uses those domains

---

### Task 17.5: Update DerivationService

**Files:**
- Modify: `packages/core/src/services/derivation-service.ts`

**Goal:** Add validation that the requested archetype and domain exist in the DB before proceeding with AI derivation.

- [ ] Add import for repositories:

```typescript
import * as ArchetypeRepo from '../db/repositories/archetype-repository'
import * as DomainRepo from '../db/repositories/domain-repository'
```

- [ ] Add validation at the start of `derivePerspectivesFromBullet`, before acquiring the lock:

```typescript
// After bullet existence + status checks, before lock acquisition:

// Validate archetype exists in DB
const archetypeExists = ArchetypeRepo.getByName(this.db, params.archetype)
if (!archetypeExists) {
  return {
    ok: false,
    error: {
      code: 'VALIDATION_ERROR',
      message: `Unknown archetype: '${params.archetype}'. Check /api/archetypes for valid values.`,
    },
  }
}

// Validate domain exists in DB
const domainExists = DomainRepo.getByName(this.db, params.domain)
if (!domainExists) {
  return {
    ok: false,
    error: {
      code: 'VALIDATION_ERROR',
      message: `Unknown domain: '${params.domain}'. Check /api/domains for valid values.`,
    },
  }
}
```

**Acceptance Criteria:**
- [ ] `derivePerspectivesFromBullet` with a valid archetype and domain proceeds normally
- [ ] `derivePerspectivesFromBullet` with an unknown archetype returns VALIDATION_ERROR before any AI call
- [ ] `derivePerspectivesFromBullet` with an unknown domain returns VALIDATION_ERROR before any AI call
- [ ] Validation happens before the in-memory lock is acquired (no lock leak on invalid input)

**Testing:**
- Unit: Call with archetype='nonexistent', verify VALIDATION_ERROR returned
- Unit: Call with domain='nonexistent', verify VALIDATION_ERROR returned
- Unit: Call with valid seeded archetype and domain, verify method proceeds past validation (will fail at AI call in test, which is expected)

---

### Task 17.6: SDK updates

**Files:**
- Create: `packages/sdk/src/resources/domains.ts`
- Create: `packages/sdk/src/resources/archetypes.ts`
- Modify: `packages/sdk/src/types.ts`
- Modify: `packages/sdk/src/client.ts`
- Modify: `packages/sdk/src/index.ts`

**Goal:** Add `DomainsResource` and `ArchetypesResource` classes to the SDK.

- [ ] Create `packages/sdk/src/resources/domains.ts`:

```typescript
import type {
  Domain,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
} from '../types'

export interface CreateDomain {
  name: string
  description?: string
}

export interface UpdateDomain {
  name?: string
  description?: string | null
}

export class DomainsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateDomain): Promise<Result<Domain>> {
    return this.request<Domain>('POST', '/api/domains', input)
  }

  list(params?: PaginationParams): Promise<PaginatedResult<Domain>> {
    const p: Record<string, string> = {}
    if (params?.offset !== undefined) p.offset = String(params.offset)
    if (params?.limit !== undefined) p.limit = String(params.limit)
    return this.requestList<Domain>('GET', '/api/domains', Object.keys(p).length > 0 ? p : undefined)
  }

  get(id: string): Promise<Result<Domain>> {
    return this.request<Domain>('GET', `/api/domains/${id}`)
  }

  update(id: string, input: UpdateDomain): Promise<Result<Domain>> {
    return this.request<Domain>('PATCH', `/api/domains/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/domains/${id}`)
  }
}
```

- [ ] Create `packages/sdk/src/resources/archetypes.ts`:

```typescript
import type {
  Archetype,
  Domain,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
} from '../types'

export interface CreateArchetype {
  name: string
  description?: string
}

export interface UpdateArchetype {
  name?: string
  description?: string | null
}

export interface ArchetypeWithDomains extends Archetype {
  domains: Domain[]
}

export class ArchetypesResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateArchetype): Promise<Result<Archetype>> {
    return this.request<Archetype>('POST', '/api/archetypes', input)
  }

  list(params?: PaginationParams): Promise<PaginatedResult<Archetype>> {
    const p: Record<string, string> = {}
    if (params?.offset !== undefined) p.offset = String(params.offset)
    if (params?.limit !== undefined) p.limit = String(params.limit)
    return this.requestList<Archetype>('GET', '/api/archetypes', Object.keys(p).length > 0 ? p : undefined)
  }

  get(id: string): Promise<Result<ArchetypeWithDomains>> {
    return this.request<ArchetypeWithDomains>('GET', `/api/archetypes/${id}`)
  }

  update(id: string, input: UpdateArchetype): Promise<Result<Archetype>> {
    return this.request<Archetype>('PATCH', `/api/archetypes/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/archetypes/${id}`)
  }

  // ── Domain associations ────────────────────────────────────────

  listDomains(archetypeId: string): Promise<Result<Domain[]>> {
    return this.request<Domain[]>('GET', `/api/archetypes/${archetypeId}/domains`)
  }

  addDomain(archetypeId: string, domainId: string): Promise<Result<void>> {
    return this.request<void>('POST', `/api/archetypes/${archetypeId}/domains`, { domain_id: domainId })
  }

  removeDomain(archetypeId: string, domainId: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/archetypes/${archetypeId}/domains/${domainId}`)
  }
}
```

- [ ] Add input types to `packages/sdk/src/types.ts`:

```typescript
// Domain input types
export interface CreateDomain {
  name: string
  description?: string
}

export interface UpdateDomain {
  name?: string
  description?: string | null
}

// Archetype input types
export interface CreateArchetype {
  name: string
  description?: string
}

export interface UpdateArchetype {
  name?: string
  description?: string | null
}

// Rich archetype response
export interface ArchetypeWithDomains extends Archetype {
  domains: Domain[]
}
```

- [ ] Update `packages/sdk/src/client.ts`:

```typescript
// Add imports:
import { DomainsResource } from './resources/domains'
import { ArchetypesResource } from './resources/archetypes'

// Add properties:
/** Domain CRUD. */
public domains: DomainsResource
/** Archetype CRUD + domain associations. */
public archetypes: ArchetypesResource

// Initialize in constructor:
this.domains = new DomainsResource(req, reqList)
this.archetypes = new ArchetypesResource(req, reqList)
```

- [ ] Update `packages/sdk/src/index.ts` to export new types and resources:

```typescript
// Add to input type exports:
export type {
  CreateDomain,
  UpdateDomain,
  CreateArchetype,
  UpdateArchetype,
} from './types'

// Add to rich response type exports:
export type {
  ArchetypeWithDomains,
} from './types'

// Add resource class exports:
export { DomainsResource } from './resources/domains'
export { ArchetypesResource } from './resources/archetypes'
```

**Acceptance Criteria:**
- [ ] `ForgeClient.domains` exposes CRUD methods
- [ ] `ForgeClient.archetypes` exposes CRUD + domain association methods
- [ ] SDK types include `CreateDomain`, `UpdateDomain`, `CreateArchetype`, `UpdateArchetype`, `ArchetypeWithDomains`
- [ ] SDK barrel exports all new types and resource classes
- [ ] `bunx tsc --noEmit` passes in the SDK package

**Testing:**
- Compile: SDK compiles without errors
- Unit: DomainsResource methods call correct HTTP methods and paths (mock request fn)
- Unit: ArchetypesResource methods call correct HTTP methods and paths (mock request fn)

---

### Task 17.7: Domains View (UI)

**Files:**
- Create: `packages/webui/src/routes/domains/+page.svelte`

**Goal:** Create a `/domains` view with CRUD list and inline editor, showing perspective count per domain.

- [ ] Create the Svelte page with:
  - Table/list showing all domains (name, description, perspective count, archetype count)
  - "Add Domain" button opens an inline form or modal
  - Click on a domain row to edit name and description
  - Delete button per row (disabled if perspective_count > 0 or archetype_count > 0)
  - Delete shows confirmation dialog
  - Error display when delete is blocked (shows "referenced by N perspectives" message)
  - Uses the SDK `ForgeClient.domains` methods

- [ ] Navigation: Modify `packages/webui/src/routes/+layout.svelte` to add `{ href: '/domains', label: 'Domains' }` to the `navItems` array. Place it between Organizations and Skills. Check if Phase 14 already added it -- if so, skip.

**Acceptance Criteria:**
- [ ] `/domains` route renders the domains list
- [ ] Domains load from API and display name, description, perspective_count, archetype_count
- [ ] Create domain form validates name format before submission
- [ ] Edit domain updates inline and saves via PATCH
- [ ] Delete is blocked with clear message when domain is referenced
- [ ] Delete succeeds for unreferenced domains
- [ ] Navigation includes Domains link

**Testing:**
- Manual: Navigate to /domains, verify list loads with seeded data (6 domains)
- Manual: Create a new domain, verify it appears in the list
- Manual: Edit a domain's description, verify update persists
- Manual: Attempt to delete a seeded domain (should be blocked by archetype association)
- Manual: Create a new domain, then delete it (should succeed)

---

### Task 17.8: Update Archetypes View (UI)

**Files:**
- Modify or create: `packages/webui/src/routes/archetypes/+page.svelte`

**Goal:** Replace the read-only archetypes page with an editable CRUD view including domain association via tag picker/checkboxes.

- [ ] **Update `packages/webui/src/routes/resumes/+page.svelte`:** Replace the hardcoded `ARCHETYPES` array (used in the archetype dropdown when creating/editing a resume) with a dynamic fetch from `forge.archetypes.list()`. Load archetypes on mount, populate the dropdown dynamically instead of using the constant.

- [ ] Update the archetypes view to:
  - Table showing all archetypes (name, description, domain_count, resume_count, perspective_count)
  - "Add Archetype" button opens inline form
  - Click on archetype to expand/edit
  - Domain association section: checkboxes or tag picker showing all available domains, checked ones are associated
  - Toggling a checkbox calls `POST /api/archetypes/:id/domains` or `DELETE /api/archetypes/:id/domains/:domainId`
  - Delete button per row (disabled if resume_count > 0 or perspective_count > 0)
  - Uses SDK `ForgeClient.archetypes` methods

**Acceptance Criteria:**
- [ ] `/archetypes` route renders editable archetype list
- [ ] Each archetype shows associated domains as tags/badges
- [ ] Domain associations can be added/removed via checkboxes
- [ ] CRUD operations work (create, edit name/description, delete)
- [ ] Delete is blocked with clear message when archetype is referenced
- [ ] Resume and perspective counts display correctly

**Testing:**
- Manual: Navigate to /archetypes, verify 6 seeded archetypes load with domain tags
- Manual: Toggle a domain checkbox, verify association change persists on page reload
- Manual: Create a new archetype, add domain associations
- Manual: Attempt to delete a seeded archetype with perspectives (should be blocked)

---

### Task 17.9: Update constants

**Files:**
- Modify: `packages/core/src/constants/archetypes.ts`

**Goal:** Remove the deprecated constants that are now in the DB. Keep the simple value constants.

- [ ] Remove `ARCHETYPES` constant
- [ ] Remove `DOMAINS` constant
- [ ] Remove `ARCHETYPE_EXPECTED_DOMAINS` constant
- [ ] Keep `THIN_COVERAGE_THRESHOLD = 2`
- [ ] Keep `FRAMINGS` (not DB-backed, simple enum-like constant)
- [ ] Update `RESUME_SECTIONS` to include new section types:

```typescript
export const RESUME_SECTIONS = [
  'summary',
  'experience',
  'work_history',   // backward compat — new resumes should use 'experience'
  'projects',
  'education',
  'skills',
  'certifications',
  'clearance',
  'presentations',
  'awards',
  'custom',
] as const;
```

- [ ] Rename the file to `packages/core/src/constants/archetypes.ts` (keep same name, just trimmed content) or rename to `constants.ts` if preferred. The import paths in `resume-service.ts` and elsewhere must match.

- [ ] Verify no remaining imports of `ARCHETYPES`, `DOMAINS`, or `ARCHETYPE_EXPECTED_DOMAINS`:

```bash
grep -r "ARCHETYPES\|ARCHETYPE_EXPECTED_DOMAINS" packages/core/src/ --include='*.ts' | grep -v '.test.' | grep -v 'node_modules'
grep -r "from.*archetypes.*import.*DOMAINS" packages/core/src/ --include='*.ts' | grep -v '.test.' | grep -v 'node_modules'
```

**Acceptance Criteria:**
- [ ] `ARCHETYPES`, `DOMAINS`, `ARCHETYPE_EXPECTED_DOMAINS` are removed from `archetypes.ts`
- [ ] `THIN_COVERAGE_THRESHOLD`, `FRAMINGS`, `RESUME_SECTIONS` remain
- [ ] `RESUME_SECTIONS` includes `experience`, `certifications`, `clearance`, `presentations`, `awards`, `custom`
- [ ] No source files (excluding tests) import the removed constants
- [ ] `bunx tsc --noEmit` passes

**Testing:**
- Compile: `bunx tsc --noEmit` passes
- Grep: No remaining imports of removed constants in non-test source files

---

### Task 17.10: Tests

**Files:**
- Create: `packages/core/src/db/repositories/__tests__/domain-repository.test.ts`
- Create: `packages/core/src/db/repositories/__tests__/archetype-repository.test.ts`
- Create: `packages/core/src/services/__tests__/domain-service.test.ts`
- Create: `packages/core/src/services/__tests__/archetype-service.test.ts`
- Create: `packages/core/src/routes/__tests__/domain-routes.test.ts`
- Create: `packages/core/src/routes/__tests__/archetype-routes.test.ts`

**Goal:** Comprehensive tests for the new entity stack: repository, service, and route layers.

#### Domain Repository Tests (`domain-repository.test.ts`)

- [ ] Test: `create` inserts domain with UUID and returns it
- [ ] Test: `create` with duplicate name throws
- [ ] Test: `get` returns domain by ID
- [ ] Test: `get` returns null for missing ID
- [ ] Test: `getByName` returns domain by name
- [ ] Test: `getByName` returns null for missing name
- [ ] Test: `list` returns all domains with usage counts
- [ ] Test: `list` respects offset and limit
- [ ] Test: `list` perspective_count reflects actual perspectives with matching domain
- [ ] Test: `update` changes name and description
- [ ] Test: `update` returns null for missing ID
- [ ] Test: `countReferences` returns correct perspective and archetype counts
- [ ] Test: `del` removes domain
- [ ] Test: `del` returns false for missing ID

#### Archetype Repository Tests (`archetype-repository.test.ts`)

- [ ] Test: `create` inserts archetype with UUID
- [ ] Test: `create` with duplicate name throws
- [ ] Test: `get` and `getByName` return correct records
- [ ] Test: `getWithDomains` returns archetype with associated domains
- [ ] Test: `getWithDomains` returns empty domains array when none associated
- [ ] Test: `list` returns counts (resume, perspective, domain)
- [ ] Test: `update` changes fields
- [ ] Test: `countReferences` returns correct resume and perspective counts
- [ ] Test: `addDomain` creates junction row
- [ ] Test: `addDomain` duplicate throws
- [ ] Test: `removeDomain` deletes junction row
- [ ] Test: `removeDomain` returns false for nonexistent
- [ ] Test: `listDomains` returns associated domains
- [ ] Test: `getExpectedDomainNames` returns correct names for seeded data
- [ ] Test: `del` cascades to archetype_domains

#### Domain Service Tests (`domain-service.test.ts`)

- [ ] Test: Create with valid input succeeds
- [ ] Test: Create with empty name fails
- [ ] Test: Create with invalid name format fails
- [ ] Test: Create with duplicate name returns CONFLICT
- [ ] Test: Delete succeeds for unreferenced domain
- [ ] Test: Delete blocked by perspective reference
- [ ] Test: Delete blocked by archetype_domains reference

#### Archetype Service Tests (`archetype-service.test.ts`)

- [ ] Test: Create with valid input succeeds
- [ ] Test: Create with empty name fails
- [ ] Test: Create with invalid name format fails
- [ ] Test: Delete blocked by resume reference
- [ ] Test: Delete blocked by perspective reference
- [ ] Test: Delete succeeds and cascades junction
- [ ] Test: addDomain succeeds
- [ ] Test: addDomain with nonexistent domain returns NOT_FOUND
- [ ] Test: addDomain duplicate returns CONFLICT
- [ ] Test: removeDomain succeeds
- [ ] Test: listDomains returns correct domains

#### Route Contract Tests (`domain-routes.test.ts`, `archetype-routes.test.ts`)

- [ ] Test: POST /api/domains 201
- [ ] Test: POST /api/domains 400 (empty name)
- [ ] Test: GET /api/domains 200 with pagination
- [ ] Test: DELETE /api/domains/:id 409 (referenced)
- [ ] Test: POST /api/archetypes 201
- [ ] Test: GET /api/archetypes/:id 200 with domains
- [ ] Test: POST /api/archetypes/:id/domains 201
- [ ] Test: DELETE /api/archetypes/:id/domains/:domainId 204
- [ ] Test: DELETE /api/archetypes/:id 409 (referenced)

#### Gap Analysis Integration Test

- [ ] Test: `analyzeGaps` produces correct results using DB-backed domains (seed archetype with custom domains, verify gap detection)

#### Derivation Validation Test

- [ ] Test: `derivePerspectivesFromBullet` with unknown archetype returns VALIDATION_ERROR
- [ ] Test: `derivePerspectivesFromBullet` with unknown domain returns VALIDATION_ERROR

**Acceptance Criteria:**
- [ ] All new tests pass
- [ ] All existing tests still pass
- [ ] `bun test` from project root has 0 failures

---

### Task 17.11: Documentation

**Files:**
- Create: `docs/src/adrs/008-editable-taxonomy.md`
- Modify: `docs/src/api/routes.md` (if exists, otherwise create)
- Modify: `docs/src/data/models/entity-types.md` (if exists, otherwise create)
- Modify: `docs/src/lib/services.md` (if exists, otherwise create)

**Goal:** Document the architectural decision to move from hardcoded constants to DB-backed entities, and update API/model documentation.

- [ ] Write ADR `008-editable-taxonomy.md`:

```markdown
# ADR 008: Editable Domain and Archetype Taxonomy

## Status
Accepted

## Context
Forge originally hardcoded 6 archetypes and 6 domains as TypeScript constants.
This made it impossible to add new archetypes or domains without a code change
and server restart. As the system matured, users needed the ability to define
custom archetypes (e.g., "devsecops-lead") and domains (e.g., "data_engineering")
without modifying source code.

## Decision
Move archetypes and domains from hardcoded constants (`ARCHETYPES`, `DOMAINS`,
`ARCHETYPE_EXPECTED_DOMAINS`) to database tables (`archetypes`, `domains`,
`archetype_domains`) with full CRUD APIs. The initial seed data matches the
original constants, ensuring backward compatibility. Gap analysis and derivation
validation query the DB instead of constants.

## Consequences
- Users can add/edit/delete archetypes and domains at runtime
- Delete is blocked when entities are referenced by perspectives or resumes
- `archetype_domains` junction replaces the `ARCHETYPE_EXPECTED_DOMAINS` map
- `perspectives.target_archetype` and `perspectives.domain` remain text columns
  referencing names (not foreign keys to IDs) for simplicity
- `THIN_COVERAGE_THRESHOLD` and `FRAMINGS` remain as code constants (simple values
  that rarely change and don't benefit from DB storage)
```

- [ ] Update route documentation with the 13 new endpoints (domains CRUD + archetypes CRUD + domain associations)

- [ ] Update entity model documentation with Domain, Archetype, ArchetypeDomain definitions

- [ ] Update services documentation with DomainService and ArchetypeService

**Acceptance Criteria:**
- [ ] ADR 008 exists and documents the decision clearly
- [ ] API docs list all domain and archetype endpoints with request/response shapes
- [ ] Entity model docs include Domain, Archetype, ArchetypeDomain
