# Phase 35: Config Export (Spec 7)

**Status:** Planning
**Date:** 2026-03-31
**Spec:** [2026-03-30-config-export.md](../refs/specs/2026-03-30-config-export.md)
**Depends on:** Phase 29 (Config Profile — for profile data export)
**Soft deps:** Phase 30 (Summaries), Phase 31 (JD) — for exporting those entity types
**Blocks:** None
**Parallelizable with:** Phase 33, Phase 34 (if they don't touch export routes or services)

## Goal

Add a unified export system to Forge that lets users download resumes in four formats (PDF, Markdown, LaTeX, JSON IR), export entity data as JSON bundles, and dump the full database as SQL. This phase creates the `ExportService` in core, new API routes under `/api/export/*`, an `ExportResource` in the SDK, a config UI page at `/config/export`, and a download dropdown on the resume builder page. It also removes the legacy `POST /api/resumes/:id/export` stub and the dead `ResumesResource.export()` SDK method.

## Non-Goals

- Import from JSON (future spec)
- Export to DOCX or Google Docs format
- Scheduled/automated backups
- Cloud storage integration
- Authentication on export endpoints (local-only assumption)

## Context

Resumes can already be compiled to LaTeX and rendered to PDF via tectonic (`POST /api/resumes/:id/pdf`), but there is no UI for triggering downloads and no way to export resumes as Markdown or JSON. There is no way to export the underlying data entities for backup or portability beyond the `just dump` SQLite dump command. The existing `POST /api/resumes/:id/export` route returns 501, and the SDK's `ResumesResource.export()` method calls a dead endpoint. Both must be removed and replaced by the new export system.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Resume Export Formats (PDF, Markdown, LaTeX, JSON) | Yes |
| 2. Data Export (entity JSON bundles) | Yes |
| 3. API Endpoints | Yes |
| 4. SDK `ExportResource` | Yes |
| 5. UI: `/config/export` | Yes |
| 6. Resume Builder Download Button | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/services/export-service.ts` | ExportService with resume format export, data export, and database dump |
| `packages/core/src/routes/export.ts` | Route handler for `/api/export/*` endpoints |
| `packages/core/src/lib/slugify.ts` | `slugify(name: string): string` helper |
| `packages/sdk/src/resources/export.ts` | `ExportResource` class for SDK |
| `packages/webui/src/routes/config/export/+page.svelte` | Export config page |
| `packages/core/src/services/__tests__/export-service.test.ts` | Unit tests for ExportService |
| `packages/core/src/routes/__tests__/export.test.ts` | Integration tests for export routes |
| `packages/core/src/lib/__tests__/slugify.test.ts` | Unit tests for slugify helper |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add `DataExportBundle` type |
| `packages/sdk/src/types.ts` | Mirror `DataExportBundle` type |
| `packages/core/src/services/index.ts` | Change `createServices(db)` to `createServices(db, dbPath)`, add `export: new ExportService(db, dbPath)` |
| `packages/core/src/index.ts` | Pass `dbPath` to `createServices(db, dbPath)` |
| `packages/core/src/routes/server.ts` | Import and mount `exportRoutes` |
| `packages/core/src/routes/resumes.ts` | Remove `POST /resumes/:id/export` stub |
| `packages/sdk/src/client.ts` | Add `public export: ExportResource`, instantiate in constructor |
| `packages/sdk/src/index.ts` | Export `ExportResource` class and `DataExportBundle` type |
| `packages/sdk/src/resources/resumes.ts` | Remove dead `export()` method |
| `packages/sdk/src/__tests__/resources.test.ts` | Remove or replace the `client.resumes.export('r1')` test (~line 616); add `ExportResource` mock tests |
| `packages/core/src/db/repositories/source-repository.ts` | Add `listAll(db)` function |
| `packages/core/src/db/repositories/bullet-repository.ts` | Add `listAll(db)` method to `BulletRepository` object |
| `packages/core/src/db/repositories/perspective-repository.ts` | Add `listAll(db)` method to `PerspectiveRepository` object |
| `packages/core/src/db/repositories/organization-repository.ts` | Add `listAll(db)` function |
| `packages/webui/src/routes/resumes/+page.svelte` | Add download dropdown to resume list (until `[id]` page exists) |

## Fallback Strategies

| Scenario | Fallback |
|----------|----------|
| Summaries spec (Phase 30) not yet implemented | Guard `summaries` case in `exportData()` with try/catch; skip silently if table doesn't exist |
| JD spec (Phase 31) not yet implemented | Guard `job_descriptions` case in `exportData()` with try/catch; skip silently if table doesn't exist |
| `sqlite3` CLI not on PATH | `dumpDatabase()` returns `{ ok: false, error: { code: 'DUMP_FAILED', message: 'sqlite3 not found on PATH' } }` |
| `tectonic` not installed | PDF export falls through to existing `generatePDF()` error handling (`TECTONIC_NOT_AVAILABLE`) |
| Resume builder `[id]` page not yet created | Add download buttons to the resume list page instead; revisit when `[id]` route is created |

---

## Tasks

### T35.1: Add `slugify` Helper

**File:** `packages/core/src/lib/slugify.ts`

```typescript
/**
 * Slugify a string for use in filenames.
 *
 * Converts to lowercase, replaces non-alphanumeric runs with hyphens,
 * and strips leading/trailing hyphens.
 */
export function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
}
```

**Acceptance criteria:**
- `slugify('Agentic AI Engineer')` returns `'agentic-ai-engineer'`
- `slugify('---foo---')` returns `'foo'`
- `slugify('hello world!')` returns `'hello-world'`
- `slugify('')` returns `''`

**Failure criteria:**
- Returns uppercase characters
- Returns leading or trailing hyphens
- Returns non-alphanumeric characters other than hyphens

---

### T35.2: Add `DataExportBundle` Type

#### `packages/core/src/types/index.ts`

Add at the end of the file, after the existing type definitions:

```typescript
// ── Export Types ────────────────────────────────────────────────────────

export interface DataExportBundle {
  forge_export: {
    version: string
    exported_at: string
    entities: string[]
  }
  sources?: Source[]
  bullets?: Bullet[]
  perspectives?: Perspective[]
  skills?: Skill[]
  organizations?: Organization[]
  summaries?: unknown[]       // typed as unknown[] until Spec 2 lands
  job_descriptions?: unknown[] // typed as unknown[] until Spec 4 lands
}
```

#### `packages/sdk/src/types.ts`

Mirror the same type:

```typescript
// ── Export Types ────────────────────────────────────────────────────────

export interface DataExportBundle {
  forge_export: {
    version: string
    exported_at: string
    entities: string[]
  }
  sources?: Source[]
  bullets?: Bullet[]
  perspectives?: Perspective[]
  skills?: Skill[]
  organizations?: Organization[]
  summaries?: unknown[]
  job_descriptions?: unknown[]
}
```

**Acceptance criteria:**
- `DataExportBundle` is importable from both `@forge/core` types and `@forge/sdk` types
- The `forge_export.entities` field is `string[]` (reflects only resolved entities)

**Failure criteria:**
- Type uses `Record<string, unknown>` instead of the structured interface
- Missing `forge_export` metadata envelope

---

### T35.3: Add `listAll` Methods to Repositories

Repositories that use paginated `list()` functions need a convenience `listAll()` method for the data export. Repositories using object-literal style add a method; function-style repositories add a standalone export.

#### `packages/core/src/db/repositories/source-repository.ts`

Add after the existing `list()` function:

```typescript
/** List all sources without pagination (for data export). */
export function listAll(db: Database): Source[] {
  return db
    .query('SELECT * FROM sources ORDER BY created_at DESC')
    .all() as Source[]
}
```

#### `packages/core/src/db/repositories/bullet-repository.ts`

Add inside the `BulletRepository` object:

```typescript
  /** List all bullets without pagination (for data export). */
  listAll(db: Database): Bullet[] {
    return db
      .query('SELECT * FROM bullets ORDER BY created_at DESC')
      .all() as Bullet[]
  },
```

#### `packages/core/src/db/repositories/perspective-repository.ts`

Add inside the `PerspectiveRepository` object:

```typescript
  /** List all perspectives without pagination (for data export). */
  listAll(db: Database): Perspective[] {
    return (
      db
        .query('SELECT * FROM perspectives ORDER BY created_at DESC')
        .all() as PerspectiveRow[]
    ).map(toPerspective)
  },
```

Note: `PerspectiveRepository` maps rows through `toPerspective()`. Follow the same pattern as existing `list()` methods in that file.

#### `packages/core/src/db/repositories/organization-repository.ts`

Add after the existing `list()` function:

```typescript
/** List all organizations without pagination (for data export). */
export function listAll(db: Database): Organization[] {
  return db
    .query('SELECT * FROM organizations ORDER BY name ASC')
    .all() as Organization[]
}
```

**Note on `SkillRepository`:** The existing `SkillRepository.list(db)` already returns all skills when called with no filter. No `listAll()` wrapper is needed — the `ExportService` calls `SkillRepo.list(db)` directly.

**Acceptance criteria:**
- Each `listAll()` returns all rows from the table, ordered consistently
- No filter parameters, no pagination
- Returns the same entity type as the paginated `list()`

**Failure criteria:**
- `listAll()` applies filters or pagination limits
- Returns raw rows without mapping (for repos that use row mappers)

---

### T35.4: Create `ExportService`

**File:** `packages/core/src/services/export-service.ts`

```typescript
/**
 * ExportService — resume format export, data bundle export, and database dump.
 *
 * Resume export methods delegate to ResumeService/resume-compiler for IR
 * compilation, then use the existing compileToMarkdown/compileToLatex functions.
 *
 * Data export collects entity data from repositories into a DataExportBundle.
 *
 * Database dump shells out to `sqlite3 <dbPath> .dump`.
 */

import type { Database } from 'bun:sqlite'
import type {
  DataExportBundle,
  ResumeDocument,
  Result,
  Source,
  Bullet,
  Perspective,
  Skill,
  Organization,
} from '../types'
import { ResumeRepository } from '../db/repositories/resume-repository'
import { BulletRepository } from '../db/repositories/bullet-repository'
import { PerspectiveRepository } from '../db/repositories/perspective-repository'
import * as SourceRepo from '../db/repositories/source-repository'
import * as SkillRepo from '../db/repositories/skill-repository'
import * as OrgRepo from '../db/repositories/organization-repository'
import { compileResumeIR } from './resume-compiler'
import { compileToLatex } from '../lib/latex-compiler'
import { compileToMarkdown } from '../lib/markdown-compiler'
import { sb2nov } from '../templates/sb2nov'

export class ExportService {
  constructor(private db: Database, private dbPath: string) {}

  // ── Resume Export Methods ─────────────────────────────────────────

  getJSON(resumeId: string): Result<ResumeDocument> {
    const ir = compileResumeIR(this.db, resumeId)
    if (!ir) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
    }
    return { ok: true, data: ir }
  }

  getMarkdown(resumeId: string): Result<string> {
    const resume = ResumeRepository.get(this.db, resumeId)
    if (!resume) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
    }

    if (resume.markdown_override) {
      return { ok: true, data: resume.markdown_override }
    }

    const ir = compileResumeIR(this.db, resumeId)
    if (!ir) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
    }

    return { ok: true, data: compileToMarkdown(ir) }
  }

  getLatex(resumeId: string): Result<string> {
    const resume = ResumeRepository.get(this.db, resumeId)
    if (!resume) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
    }

    if (resume.latex_override) {
      return { ok: true, data: resume.latex_override }
    }

    const ir = compileResumeIR(this.db, resumeId)
    if (!ir) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Resume ${resumeId} not found` } }
    }

    return { ok: true, data: compileToLatex(ir, sb2nov) }
  }

  // ── Data Export ───────────────────────────────────────────────────

  exportData(entities: string[]): Result<DataExportBundle> {
    const resolved: string[] = []
    const bundle: DataExportBundle = {
      forge_export: {
        version: '1.0',
        exported_at: new Date().toISOString(),
        entities: resolved, // populated below; reflects only entities actually included
      },
    }

    for (const entity of entities) {
      switch (entity) {
        case 'sources':
          bundle.sources = SourceRepo.listAll(this.db)
          resolved.push(entity)
          break
        case 'bullets':
          bundle.bullets = BulletRepository.listAll(this.db)
          resolved.push(entity)
          break
        case 'perspectives':
          bundle.perspectives = PerspectiveRepository.listAll(this.db)
          resolved.push(entity)
          break
        case 'skills':
          bundle.skills = SkillRepo.list(this.db)
          resolved.push(entity)
          break
        case 'organizations':
          bundle.organizations = OrgRepo.listAll(this.db)
          resolved.push(entity)
          break
        case 'summaries':
          try {
            const rows = this.db.query('SELECT * FROM summaries ORDER BY created_at DESC').all()
            bundle.summaries = rows
            resolved.push(entity)
          } catch {
            // Table does not exist yet (Spec 2 / Phase 30 not implemented)
          }
          break
        case 'job_descriptions':
          try {
            const rows = this.db.query('SELECT * FROM job_descriptions ORDER BY created_at DESC').all()
            bundle.job_descriptions = rows
            resolved.push(entity)
          } catch {
            // Table does not exist yet (Spec 4 / Phase 31 not implemented)
          }
          break
        // Unknown entity names are silently ignored (no error, just excluded)
      }
    }

    return { ok: true, data: bundle }
  }

  // ── Database Dump ─────────────────────────────────────────────────

  async dumpDatabase(): Promise<Result<string>> {
    try {
      const proc = Bun.spawn(['sqlite3', this.dbPath, '.dump'], {
        stdout: 'pipe',
        stderr: 'pipe',
      })

      const stdout = await new Response(proc.stdout).text()
      const exitCode = await proc.exited

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text()
        return {
          ok: false,
          error: { code: 'DUMP_FAILED', message: stderr.trim() || 'sqlite3 dump failed' },
        }
      }

      return { ok: true, data: stdout }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
        return {
          ok: false,
          error: { code: 'DUMP_FAILED', message: 'sqlite3 not found on PATH' },
        }
      }
      throw err
    }
  }
}
```

**Key design decisions:**
- `ExportService` receives `dbPath` as a separate constructor parameter because `Database` does not expose its file path.
- `summaries` and `job_descriptions` cases use raw SQL with try/catch to guard against missing tables (Phases 30/31 may not be implemented yet).
- `dumpDatabase()` shells out to `sqlite3` CLI (same as `just dump`), catches `ENOENT` when `sqlite3` is not on PATH.
- `getMarkdown()` and `getLatex()` check for override fields before compiling from IR.

**Acceptance criteria:**
- `getJSON()` returns the same IR as `ResumeService.getIR()`
- `getMarkdown()` returns `markdown_override` when present, compiled markdown otherwise
- `getLatex()` returns `latex_override` when present, compiled LaTeX otherwise
- `exportData(['sources', 'bogus'])` returns bundle with `forge_export.entities: ['sources']` (bogus silently skipped)
- `exportData(['summaries'])` does not throw when `summaries` table is missing
- `dumpDatabase()` returns `{ ok: false }` with `DUMP_FAILED` when `sqlite3` is missing

**Failure criteria:**
- `getMarkdown()` ignores `markdown_override`
- `exportData()` throws on unknown entity names
- `dumpDatabase()` throws unhandled `ENOENT`
- `forge_export.entities` contains entity names that were not successfully populated

---

### T35.5: Wire `ExportService` into `createServices`

#### `packages/core/src/services/index.ts`

```typescript
import { ExportService } from './export-service'

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
  domains: DomainService
  archetypes: ArchetypeService
  export: ExportService               // ADD
}

export function createServices(db: Database, dbPath: string): Services {  // ADD dbPath
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
    domains: new DomainService(db),
    archetypes: new ArchetypeService(db),
    export: new ExportService(db, dbPath),  // ADD
  }
}

// Re-export
export { ExportService } from './export-service'
```

#### `packages/core/src/index.ts`

Change:
```typescript
// Before:
const services = createServices(db)

// After:
const services = createServices(db, dbPath)
```

The `dbPath` variable already exists at line 41 (`const dbPath = resolve(DB_PATH)`), so no new variable is needed.

**Note:** `dbPath` is consumed only at `ExportService` construction time inside `createServices()`. It does NOT flow through `createApp()` or to the route layer. The route accesses `ExportService` methods via `services.export`, which already has `dbPath` internally. Do not attempt to thread `dbPath` through `createApp` — it is not needed there.

**Acceptance criteria:**
- `createServices(db, dbPath)` compiles without errors
- `services.export` is an instance of `ExportService`
- `services.export.dumpDatabase()` has access to the correct `dbPath`

**Failure criteria:**
- `createServices` still only accepts `(db)` — callers break
- `dbPath` is not threaded through from the entrypoint

---

### T35.6: Create Export Route Handler

**File:** `packages/core/src/routes/export.ts`

```typescript
/**
 * Export routes — resume format export, data bundle, and database dump.
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './server'
import { slugify } from '../lib/slugify'
import { ResumeRepository } from '../db/repositories/resume-repository'
import type { Database } from 'bun:sqlite'

export function exportRoutes(services: Services, db: Database) {
  const app = new Hono()

  // ── Resume Export ──────────────────────────────────────────────────

  app.get('/export/resume/:id', async (c) => {
    const id = c.req.param('id')
    const format = c.req.query('format')

    if (!format || !['pdf', 'markdown', 'latex', 'json'].includes(format)) {
      return c.json(
        { error: { code: 'VALIDATION_ERROR', message: 'format query parameter is required. Valid values: pdf, markdown, latex, json' } },
        400,
      )
    }

    // Look up the resume to get the name for the filename
    const resume = ResumeRepository.get(db, id)
    if (!resume) {
      return c.json(
        { error: { code: 'NOT_FOUND', message: `Resume ${id} not found` } },
        404,
      )
    }

    const slug = slugify(resume.name)
    const date = new Date().toISOString().slice(0, 10) // YYYY-MM-DD

    switch (format) {
      case 'json': {
        const result = services.export.getJSON(id)
        if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
        return c.json({ data: result.data }, 200, {
          'Content-Disposition': `attachment; filename="${slug}-${date}.json"`,
        })
      }

      case 'markdown': {
        const result = services.export.getMarkdown(id)
        if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
        return new Response(result.data, {
          status: 200,
          headers: {
            'Content-Type': 'text/markdown; charset=utf-8',
            'Content-Disposition': `attachment; filename="${slug}-${date}.md"`,
          },
        })
      }

      case 'latex': {
        const result = services.export.getLatex(id)
        if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
        return new Response(result.data, {
          status: 200,
          headers: {
            'Content-Type': 'application/x-latex; charset=utf-8',
            'Content-Disposition': `attachment; filename="${slug}-${date}.tex"`,
          },
        })
      }

      case 'pdf': {
        // Delegate to existing ResumeService.generatePDF()
        const result = await services.resumes.generatePDF(id)
        if (!result.ok) {
          const code = result.error.code
          const status = code === 'TECTONIC_NOT_AVAILABLE' ? 501
                       : code === 'TECTONIC_TIMEOUT' ? 504
                       : code === 'LATEX_COMPILE_ERROR' ? 422
                       : mapStatusCode(code)
          return c.json({ error: result.error }, status as any)
        }
        return new Response(result.data, {
          status: 200,
          headers: {
            'Content-Type': 'application/pdf',
            'Content-Disposition': `attachment; filename="${slug}-${date}.pdf"`,
          },
        })
      }
    }
  })

  // ── Data Export ────────────────────────────────────────────────────

  app.get('/export/data', (c) => {
    const entitiesParam = c.req.query('entities')
    if (!entitiesParam) {
      return c.json(
        { error: { code: 'VALIDATION_ERROR', message: 'entities query parameter is required (comma-separated)' } },
        400,
      )
    }

    const entities = entitiesParam.split(',').map(e => e.trim()).filter(Boolean)
    if (entities.length === 0) {
      return c.json(
        { error: { code: 'VALIDATION_ERROR', message: 'At least one entity type is required' } },
        400,
      )
    }

    const result = services.export.exportData(entities)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))

    const date = new Date().toISOString().slice(0, 10)
    return c.json({ data: result.data }, 200, {
      'Content-Disposition': `attachment; filename="forge-export-${date}.json"`,
    })
  })

  // ── Database Dump ─────────────────────────────────────────────────

  app.get('/export/dump', async (c) => {
    const result = await services.export.dumpDatabase()
    if (!result.ok) {
      const status = result.error.code === 'DUMP_FAILED' ? 502 : 500
      return c.json({ error: result.error }, status as any)
    }

    const date = new Date().toISOString().slice(0, 10)
    return new Response(result.data, {
      status: 200,
      headers: {
        'Content-Type': 'application/sql; charset=utf-8',
        'Content-Disposition': `attachment; filename="forge-dump-${date}.sql"`,
      },
    })
  })

  return app
}
```

**Key design decisions:**
- PDF export uses `Content-Disposition: attachment` (triggers download), unlike the existing `POST /api/resumes/:id/pdf` which uses `inline` (for preview).
- Data export wraps the bundle in `{ data: bundle }` envelope (standard pattern). The data export bundle is returned inside the standard `{ data: ... }` envelope, matching all other API endpoints. The spec's bare JSON example is a shorthand — the actual HTTP response body is `{ data: DataExportBundle }`.
- Database dump returns raw SQL text (not JSON envelope).
- `format` query param is validated upfront.
- Resume name is fetched separately for the slug filename, even though `getJSON()` etc. also look up the resume. This is fine for export (not a hot path).

**Note:** `exportRoutes(services, db)` passes `db` to the route factory for `ResumeRepository.get()` lookup (resume name for slug). This follows the `supportingRoutes(services, db)` pattern. An alternative would be adding `getResumeName(id)` to ExportService, but the current approach is simpler and consistent with existing code.

**Acceptance criteria:**
- `GET /api/export/resume/:id?format=json` returns `{ data: ResumeDocument }` with `Content-Disposition: attachment`
- `GET /api/export/resume/:id?format=pdf` returns binary PDF with `Content-Disposition: attachment`
- `GET /api/export/resume/:id?format=markdown` returns text with `Content-Type: text/markdown`
- `GET /api/export/resume/:id?format=latex` returns text with `Content-Type: application/x-latex`
- `GET /api/export/data?entities=sources,skills` returns `{ data: DataExportBundle }`
- `GET /api/export/dump` returns SQL text with `Content-Type: application/sql`
- Missing `format` param returns 400
- Invalid `format` value returns 400
- Missing `entities` param returns 400
- Filename matches `{slug}-{YYYY-MM-DD}.{ext}` pattern

**Failure criteria:**
- JSON export omits the `{ data: ... }` envelope
- PDF export uses `Content-Disposition: inline` instead of `attachment`
- Dump endpoint returns JSON envelope instead of raw SQL
- Routes not mounted under `/api` basePath

---

### T35.7: Mount Export Routes in Server

#### `packages/core/src/routes/server.ts`

Add import and route registration:

```typescript
import { exportRoutes } from './export'

// In createApp(), after existing routes:
app.route('/', exportRoutes(services, db))
```

**Acceptance criteria:**
- `GET /api/export/resume/:id?format=json` is reachable
- `GET /api/export/data?entities=sources` is reachable
- `GET /api/export/dump` is reachable

**Failure criteria:**
- Export routes return 404 (not mounted)

---

### T35.8: Remove Legacy Export Stub

#### `packages/core/src/routes/resumes.ts`

Remove the legacy export endpoint (lines 198-205):

```typescript
// REMOVE THIS BLOCK:
  // ── Legacy export endpoint (replaced by PDF) ──────────────────────

  app.post('/resumes/:id/export', (c) => {
    return c.json(
      { error: { code: 'NOT_IMPLEMENTED', message: 'Resume export is not yet implemented. Use POST /resumes/:id/pdf instead.' } },
      501,
    )
  })
```

#### `packages/sdk/src/resources/resumes.ts`

Remove the dead `export()` method (lines 194-196):

```typescript
// REMOVE THIS BLOCK:
  export(id: string): Promise<Result<never>> {
    return this.request<never>('GET', `/api/resumes/${id}/export`)
  }
```

Also update `packages/sdk/src/__tests__/resources.test.ts` — remove or replace the test at line ~616 that tests `client.resumes.export('r1')`. This test will fail to compile after the method is removed.

**Acceptance criteria:**
- `POST /api/resumes/:id/export` returns 404 (route gone)
- `ResumesResource` no longer has an `export()` method
- Existing `POST /api/resumes/:id/pdf` endpoint still works
- The SDK test for `resumes.export()` is removed or replaced

**Failure criteria:**
- `POST /api/resumes/:id/pdf` is accidentally removed
- SDK `export()` method still exists and confuses callers
- SDK tests fail to compile due to removed `export()` method

---

### T35.9: Create SDK `ExportResource`

**File:** `packages/sdk/src/resources/export.ts`

```typescript
import type {
  DataExportBundle,
  ForgeError,
  RequestFn,
  ResumeDocument,
  Result,
} from '../types'

export class ExportResource {
  constructor(
    private request: RequestFn,
    private baseUrl: string,
  ) {}

  /** Export a resume as JSON (returns the full IR document). */
  async resumeAsJson(id: string): Promise<Result<ResumeDocument>> {
    return this.request<ResumeDocument>(
      'GET',
      `/api/export/resume/${id}?format=json`,
    )
  }

  /**
   * Download a resume as a binary blob (PDF, Markdown, or LaTeX).
   *
   * Uses raw `fetch()` instead of `this.request()` because the response
   * is not a JSON envelope — it is the raw file content.
   */
  async downloadResume(
    id: string,
    format: 'pdf' | 'markdown' | 'latex',
  ): Promise<Result<Blob>> {
    try {
      const response = await fetch(
        `${this.baseUrl}/api/export/resume/${id}?format=${format}`,
      )

      if (!response.ok) {
        // Try to parse JSON error envelope
        try {
          const json = await response.json() as { error?: ForgeError }
          const error = json.error ?? {
            code: 'EXPORT_FAILED',
            message: `HTTP ${response.status}`,
          }
          return { ok: false, error }
        } catch {
          return {
            ok: false,
            error: {
              code: 'EXPORT_FAILED',
              message: `HTTP ${response.status}: ${response.statusText}`,
            },
          }
        }
      }

      const blob = await response.blob()
      return { ok: true, data: blob }
    } catch (err) {
      return {
        ok: false,
        error: { code: 'NETWORK_ERROR', message: String(err) },
      }
    }
  }

  /** Export entity data as a JSON bundle. */
  async exportData(entities: string[]): Promise<Result<DataExportBundle>> {
    return this.request<DataExportBundle>(
      'GET',
      `/api/export/data?entities=${entities.join(',')}`,
    )
  }

  /**
   * Download a full database dump as SQL text.
   *
   * Uses raw `fetch()` because the response is SQL text, not JSON.
   */
  async dumpDatabase(): Promise<Result<Blob>> {
    try {
      const response = await fetch(`${this.baseUrl}/api/export/dump`)

      if (!response.ok) {
        try {
          const json = await response.json() as { error?: ForgeError }
          const error = json.error ?? {
            code: 'DUMP_FAILED',
            message: `HTTP ${response.status}`,
          }
          return { ok: false, error }
        } catch {
          return {
            ok: false,
            error: {
              code: 'DUMP_FAILED',
              message: `HTTP ${response.status}: ${response.statusText}`,
            },
          }
        }
      }

      const blob = await response.blob()
      return { ok: true, data: blob }
    } catch (err) {
      return {
        ok: false,
        error: { code: 'NETWORK_ERROR', message: String(err) },
      }
    }
  }
}
```

**Key design decisions:**
- `resumeAsJson()` uses `this.request<ResumeDocument>()` which unwraps the `{ data }` envelope automatically.
- `downloadResume()` and `dumpDatabase()` use raw `fetch()` because they return non-JSON content (Blob).
- Error responses from non-JSON endpoints try to parse JSON first (the server returns JSON error envelopes even for binary endpoints), then fall back to a generic error.
- Constructor takes `(request, baseUrl)` — matching the spec's `ExportResource(req, baseUrl)` pattern.

**Acceptance criteria:**
- `resumeAsJson(id)` returns `Result<ResumeDocument>` (envelope unwrapped)
- `downloadResume(id, 'pdf')` returns `Result<Blob>`
- `exportData(['sources'])` returns `Result<DataExportBundle>` (envelope unwrapped)
- `dumpDatabase()` returns `Result<Blob>`
- Network errors are caught and returned as `{ ok: false, error: ... }`

**Failure criteria:**
- `resumeAsJson()` returns the raw `{ data: ... }` envelope without unwrapping
- `downloadResume()` tries to parse the response as JSON
- Unhandled exceptions escape the SDK methods

---

### T35.10: Wire `ExportResource` into `ForgeClient`

#### `packages/sdk/src/client.ts`

Add import:

```typescript
import { ExportResource } from './resources/export'
```

Add property declaration (after `public skills: SkillsResource`):

```typescript
  /** Export: resume downloads, data bundles, database dump. */
  public export: ExportResource
```

Add instantiation in constructor (after `this.skills = ...`):

```typescript
    this.export = new ExportResource(req, this.baseUrl)
```

#### `packages/sdk/src/index.ts`

Add resource class export:

```typescript
export { ExportResource } from './resources/export'
```

Add type export in the appropriate section:

```typescript
// Export types
export type { DataExportBundle } from './types'
```

**Acceptance criteria:**
- `new ForgeClient({ baseUrl }).export` is an `ExportResource` instance
- `ExportResource` is importable from `@forge/sdk`
- `DataExportBundle` is importable from `@forge/sdk`

**Failure criteria:**
- `client.export` is undefined
- `ExportResource` not re-exported from barrel

---

### T35.11: Create Export Config UI Page

**File:** `packages/webui/src/routes/config/export/+page.svelte`

```svelte
<script lang="ts">
  import { getContext } from 'svelte'
  import type { ForgeClient } from '@forge/sdk'

  const client = getContext<ForgeClient>('client')

  // ── Resume export state ────────────────────────────────────────────
  let resumes: Array<{ id: string; name: string }> = $state([])
  let selectedResumeId: string = $state('')
  let format: 'pdf' | 'markdown' | 'latex' | 'json' = $state('pdf')
  let exportingResume = $state(false)
  let resumeError: string | null = $state(null)

  // ── Data export state ──────────────────────────────────────────────
  const entityTypes = [
    { key: 'sources', label: 'Sources' },
    { key: 'bullets', label: 'Bullets' },
    { key: 'perspectives', label: 'Perspectives' },
    { key: 'skills', label: 'Skills' },
    { key: 'organizations', label: 'Organizations' },
    { key: 'summaries', label: 'Summaries' },
    { key: 'job_descriptions', label: 'Job Descriptions' },
  ]
  let selectedEntities: Record<string, boolean> = $state(
    Object.fromEntries(entityTypes.map(e => [e.key, true]))
  )
  let exportingData = $state(false)
  let dataError: string | null = $state(null)

  // ── Dump state ─────────────────────────────────────────────────────
  let exportingDump = $state(false)
  let dumpError: string | null = $state(null)

  // ── Load resumes on mount ──────────────────────────────────────────
  $effect(() => {
    loadResumes()
  })

  async function loadResumes() {
    const result = await client.resumes.list({ limit: 200, offset: 0 })
    if (result.ok) {
      resumes = result.data
      if (resumes.length > 0 && !selectedResumeId) {
        selectedResumeId = resumes[0].id
      }
    }
  }

  // ── Download helpers ───────────────────────────────────────────────

  function triggerDownload(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  async function downloadResume() {
    if (!selectedResumeId) return
    exportingResume = true
    resumeError = null

    try {
      if (format === 'json') {
        const result = await client.export.resumeAsJson(selectedResumeId)
        if (!result.ok) {
          resumeError = result.error.message
          return
        }
        const blob = new Blob([JSON.stringify(result.data, null, 2)], {
          type: 'application/json',
        })
        triggerDownload(blob, `resume-${new Date().toISOString().slice(0, 10)}.json`)
      } else {
        const result = await client.export.downloadResume(selectedResumeId, format)
        if (!result.ok) {
          resumeError = result.error.message
          return
        }
        const ext = format === 'pdf' ? 'pdf' : format === 'markdown' ? 'md' : 'tex'
        triggerDownload(result.data, `resume-${new Date().toISOString().slice(0, 10)}.${ext}`)
      }
    } catch (err) {
      resumeError = String(err)
    } finally {
      exportingResume = false
    }
  }

  async function downloadData() {
    const entities = Object.entries(selectedEntities)
      .filter(([, selected]) => selected)
      .map(([key]) => key)

    if (entities.length === 0) {
      dataError = 'Select at least one entity type'
      return
    }

    exportingData = true
    dataError = null

    try {
      const result = await client.export.exportData(entities)
      if (!result.ok) {
        dataError = result.error.message
        return
      }
      const blob = new Blob([JSON.stringify(result.data, null, 2)], {
        type: 'application/json',
      })
      triggerDownload(blob, `forge-export-${new Date().toISOString().slice(0, 10)}.json`)
    } catch (err) {
      dataError = String(err)
    } finally {
      exportingData = false
    }
  }

  async function downloadDump() {
    exportingDump = true
    dumpError = null

    try {
      const result = await client.export.dumpDatabase()
      if (!result.ok) {
        dumpError = result.error.message
        return
      }
      triggerDownload(result.data, `forge-dump-${new Date().toISOString().slice(0, 10)}.sql`)
    } catch (err) {
      dumpError = String(err)
    } finally {
      exportingDump = false
    }
  }

  function selectAll() {
    for (const e of entityTypes) {
      selectedEntities[e.key] = true
    }
  }

  function deselectAll() {
    for (const e of entityTypes) {
      selectedEntities[e.key] = false
    }
  }
</script>

<svelte:head>
  <title>Export | Forge</title>
</svelte:head>

<div class="page">
  <h1>Export</h1>

  <!-- Resume Export -->
  <section class="export-section">
    <h2>Export Resume</h2>
    <p>Download a resume in your preferred format.</p>

    <div class="export-form">
      <label>
        Resume
        <select bind:value={selectedResumeId}>
          {#each resumes as resume}
            <option value={resume.id}>{resume.name}</option>
          {/each}
        </select>
      </label>

      <label>
        Format
        <div class="format-selector">
          {#each (['pdf', 'markdown', 'latex', 'json'] as const) as fmt}
            <button
              class="format-btn"
              class:active={format === fmt}
              onclick={() => format = fmt}
            >
              {fmt.toUpperCase()}
            </button>
          {/each}
        </div>
      </label>

      <button
        class="btn btn-primary"
        onclick={downloadResume}
        disabled={exportingResume || !selectedResumeId}
      >
        {exportingResume ? 'Generating...' : 'Download'}
      </button>

      {#if resumeError}
        <p class="error">{resumeError}</p>
      {/if}
    </div>
  </section>

  <!-- Data Export -->
  <section class="export-section">
    <h2>Export Data</h2>
    <p>Download entity data as a JSON bundle for backup or portability.</p>

    <div class="entity-checkboxes">
      {#each entityTypes as entity}
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={selectedEntities[entity.key]} />
          {entity.label}
        </label>
      {/each}
    </div>

    <div class="entity-actions">
      <button class="btn btn-sm" onclick={selectAll}>Select All</button>
      <button class="btn btn-sm" onclick={deselectAll}>Deselect All</button>
    </div>

    <button
      class="btn btn-primary"
      onclick={downloadData}
      disabled={exportingData}
    >
      {exportingData ? 'Exporting...' : 'Download Data'}
    </button>

    {#if dataError}
      <p class="error">{dataError}</p>
    {/if}
  </section>

  <!-- Database Backup -->
  <section class="export-section">
    <h2>Database Backup</h2>
    <p>
      Download a full SQL dump of the database. This includes all tables,
      data, and schema definitions. The dump can be re-imported with
      <code>sqlite3 forge.db &lt; dump.sql</code>.
    </p>

    <button
      class="btn btn-primary"
      onclick={downloadDump}
      disabled={exportingDump}
    >
      {exportingDump ? 'Dumping...' : 'Download SQL Dump'}
    </button>

    {#if dumpError}
      <p class="error">{dumpError}</p>
    {/if}
  </section>
</div>

<style>
  .page {
    max-width: 48rem;
    margin: 0 auto;
    padding: 2rem 1rem;
  }

  h1 {
    margin-bottom: 2rem;
  }

  .export-section {
    margin-bottom: 3rem;
    padding: 1.5rem;
    border: 1px solid var(--border, #e2e8f0);
    border-radius: 0.5rem;
  }

  .export-section h2 {
    margin-top: 0;
    margin-bottom: 0.5rem;
  }

  .export-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-top: 1rem;
  }

  .format-selector {
    display: flex;
    gap: 0.25rem;
    margin-top: 0.25rem;
  }

  .format-btn {
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--border, #e2e8f0);
    background: transparent;
    cursor: pointer;
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }

  .format-btn.active {
    background: var(--primary, #3b82f6);
    color: white;
    border-color: var(--primary, #3b82f6);
  }

  .entity-checkboxes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
    gap: 0.5rem;
    margin: 1rem 0;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .entity-actions {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border, #e2e8f0);
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
    background: transparent;
  }

  .btn-primary {
    background: var(--primary, #3b82f6);
    color: white;
    border-color: var(--primary, #3b82f6);
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-sm {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
  }

  .error {
    color: var(--error, #ef4444);
    font-size: 0.875rem;
    margin-top: 0.5rem;
  }

  code {
    background: var(--code-bg, #f1f5f9);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }
</style>
```

**Acceptance criteria:**
- Page renders at `/config/export` with three sections
- Resume dropdown loads from `GET /api/resumes`
- Format selector highlights the active format
- Download triggers file download in the browser
- Entity checkboxes support Select All / Deselect All
- Error messages display when export fails
- Loading spinners display during export

**Note:** When using blob URLs for downloads, the `Content-Disposition` header from the server is lost. The `a.download = filename` attribute sets the filename client-side. Apply the same slugify logic client-side to match the server's filename format, or import the `slugify` function from a shared utility.

**Failure criteria:**
- Page crashes when no resumes exist
- Download uses `window.open()` instead of blob URL (unreliable for binary)
- Entity selection state doesn't persist across re-renders

---

### T35.12: Add Download Dropdown to Resume List Page

**File:** `packages/webui/src/routes/resumes/+page.svelte`

Since the resume builder `[id]` page does not exist yet, add download buttons to the resume list page. This will be moved to the `[id]` page when it is created.

Add to each resume's row/card in the list a download dropdown:

```svelte
<!-- Add to each resume item in the list -->
<div class="dropdown">
  <button class="btn btn-sm" onclick={() => toggleDropdown(resume.id)}>
    Download
  </button>
  {#if openDropdown === resume.id}
    <div class="dropdown-menu">
      <button onclick={() => downloadResumeAs(resume.id, 'pdf')}>PDF</button>
      <button onclick={() => downloadResumeAs(resume.id, 'markdown')}>Markdown</button>
      <button onclick={() => downloadResumeAs(resume.id, 'latex')}>LaTeX</button>
      <button onclick={() => downloadResumeAs(resume.id, 'json')}>JSON (IR)</button>
    </div>
  {/if}
</div>
```

Script additions:

```svelte
<script>
  // Add to existing script block:
  let openDropdown: string | null = $state(null)

  function toggleDropdown(id: string) {
    openDropdown = openDropdown === id ? null : id
  }

  function triggerDownload(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  async function downloadResumeAs(id: string, format: 'pdf' | 'markdown' | 'latex' | 'json') {
    openDropdown = null

    if (format === 'json') {
      const result = await client.export.resumeAsJson(id)
      if (!result.ok) return // TODO: show error toast
      const blob = new Blob([JSON.stringify(result.data, null, 2)], { type: 'application/json' })
      triggerDownload(blob, `resume-${new Date().toISOString().slice(0, 10)}.json`)
    } else {
      const result = await client.export.downloadResume(id, format)
      if (!result.ok) return // TODO: show error toast
      const ext = format === 'pdf' ? 'pdf' : format === 'markdown' ? 'md' : 'tex'
      triggerDownload(result.data, `resume-${new Date().toISOString().slice(0, 10)}.${ext}`)
    }
  }
</script>
```

**Acceptance criteria:**
- Each resume in the list has a "Download" button
- Clicking "Download" shows a dropdown with four format options
- Selecting a format triggers download and closes the dropdown

**Note:** When using blob URLs for downloads, the `Content-Disposition` header from the server is lost. The `a.download = filename` attribute sets the filename client-side. Apply the same slugify logic client-side to match the server's filename format, or import the `slugify` function from a shared utility.

**Failure criteria:**
- Multiple dropdowns can be open simultaneously
- Download fails silently with no user feedback path

---

## Testing Support

### Fixtures

**Test helper: `createTestExportService`**

```typescript
// In export-service test files:
import { getDatabase } from '../../db/connection'
import { runMigrations } from '../../db/migrate'
import { resolve } from 'path'
import { ExportService } from '../export-service'

function createTestExportService() {
  const db = getDatabase(':memory:')
  const migrationsDir = resolve(import.meta.dir, '../../db/migrations')
  runMigrations(db, migrationsDir)
  // dbPath is ':memory:' — dumpDatabase() will fail with sqlite3 on :memory:
  // but that's fine for unit tests (we test dump separately)
  return { db, service: new ExportService(db, ':memory:') }
}
```

**Important:** `dumpDatabase()` cannot be meaningfully tested against `:memory:` databases — the `sqlite3` CLI opens a fresh in-memory DB, not the Bun process's in-process connection. Tests for `dumpDatabase()` require a file-backed database (use `mkdtemp()` to create a temp directory with a real SQLite file).

### Test Cases

#### Unit Tests: `packages/core/src/lib/__tests__/slugify.test.ts`

| Test | Input | Expected | Kind |
|------|-------|----------|------|
| Slugifies regular string | `'Agentic AI Engineer'` | `'agentic-ai-engineer'` | unit |
| Strips leading/trailing hyphens | `'---foo---'` | `'foo'` | unit |
| Handles special characters | `'hello world! @#$'` | `'hello-world'` | unit |
| Empty string | `''` | `''` | unit |
| Single word | `'Python'` | `'python'` | unit |
| Multiple consecutive spaces | `'a   b'` | `'a-b'` | unit |

#### Unit Tests: `packages/core/src/services/__tests__/export-service.test.ts`

| Test | Description | Kind |
|------|-------------|------|
| `getJSON` returns IR for existing resume | Seed resume + entries, verify IR shape | unit |
| `getJSON` returns NOT_FOUND for missing resume | Call with bad ID | unit |
| `getMarkdown` returns markdown_override when set | Seed resume with override | unit |
| `getMarkdown` compiles from IR when no override | Seed resume without override | unit |
| `getMarkdown` returns NOT_FOUND for missing resume | Call with bad ID | unit |
| `getLatex` returns latex_override when set | Seed resume with override | unit |
| `getLatex` compiles from IR when no override | Seed resume without override | unit |
| `getLatex` returns NOT_FOUND for missing resume | Call with bad ID | unit |
| `exportData` returns sources | Seed sources, export `['sources']` | unit |
| `exportData` returns bullets | Seed bullets, export `['bullets']` | unit |
| `exportData` returns perspectives | Seed perspectives, export `['perspectives']` | unit |
| `exportData` returns skills | Seed skills, export `['skills']` | unit |
| `exportData` returns organizations | Seed orgs, export `['organizations']` | unit |
| `exportData` ignores unknown entity names | Export `['bogus']`, verify empty resolved list | unit |
| `exportData` resolved entities reflects actual | Export `['sources', 'bogus']`, entities = `['sources']` | unit |
| `exportData` handles missing summaries table | Export `['summaries']`, no crash | unit |
| `exportData` handles missing job_descriptions table | Export `['job_descriptions']`, no crash | unit |
| `exportData` metadata envelope has version and timestamp | Verify `forge_export` structure | unit |
| `dumpDatabase` returns DUMP_FAILED when sqlite3 not found | Requires PATH manipulation or mock to simulate missing sqlite3 | unit |

#### Integration Tests: `packages/core/src/routes/__tests__/export.test.ts`

| Test | Description | Kind |
|------|-------------|------|
| `GET /api/export/resume/:id?format=json` returns IR envelope | Verify `{ data: ResumeDocument }` | integration |
| `GET /api/export/resume/:id?format=json` has Content-Disposition attachment | Check header | integration |
| `GET /api/export/resume/:id?format=markdown` returns text/markdown | Check Content-Type | integration |
| `GET /api/export/resume/:id?format=latex` returns application/x-latex | Check Content-Type | integration |
| `GET /api/export/resume/:id?format=json` with missing resume returns 404 | Check error | integration |
| `GET /api/export/resume/:id` without format returns 400 | Check error | integration |
| `GET /api/export/resume/:id?format=invalid` returns 400 | Check error | integration |
| `GET /api/export/data?entities=sources,skills` returns bundle | Verify structure | integration |
| `GET /api/export/data` without entities returns 400 | Check error | integration |
| `GET /api/export/data?entities=` (empty) returns 400 | Check error | integration |
| Content-Disposition filename matches slug-date pattern | Regex check on header | integration |
| `GET /api/export/dump` returns 200 with Content-Type: application/sql and Content-Disposition: attachment | Requires file-backed test DB (use `mkdtemp()`) | integration |
| `GET /api/export/dump` returns 500/502 when sqlite3 unavailable | Simulate missing sqlite3 binary | integration |
| Existing `POST /api/resumes/:id/pdf` still returns `Content-Disposition: inline` | Verify `POST /api/resumes/:id/pdf` returns `Content-Disposition: inline` while `GET /api/export/resume/:id?format=pdf` returns `Content-Disposition: attachment`. Assert exact header values. | contract |
| New `GET /api/export/resume/:id?format=pdf` returns `Content-Disposition: attachment` | Verify new endpoint returns attachment disposition with slug-date filename | contract |
| Legacy `POST /api/resumes/:id/export` returns 404 | Verify stub removed | smoke |

#### SDK Tests: `packages/sdk/src/__tests__/resources.test.ts`

Add SDK-level mock tests for `ExportResource` — these replace the removed `resumes.export()` test.

| Test | Description | Kind |
|------|-------------|------|
| `client.export.resumeAsJson(id)` returns Result<ResumeDocument> | Mock request fn, verify URL and envelope unwrapping | unit |
| `client.export.downloadResume(id, 'pdf')` returns Result<Blob> | Mock fetch, verify raw blob handling | unit |
| `client.export.exportData(['sources', 'skills'])` returns Result<DataExportBundle> | Mock request fn, verify query param encoding | unit |
| `client.export.dumpDatabase()` returns Result<Blob> | Mock fetch, verify raw blob handling | unit |

#### Smoke Tests

| Test | Description | Kind |
|------|-------------|------|
| Export page renders at `/config/export` | Navigate to URL, page loads | smoke/e2e |
| Resume dropdown populates | At least one resume appears | smoke/e2e |
| Download button initiates download | Click triggers blob download | smoke/e2e |

### Test Kinds Summary

| Kind | Count | Location |
|------|-------|----------|
| Unit | ~25 | `slugify.test.ts`, `export-service.test.ts`, `resources.test.ts` (SDK ExportResource) |
| Integration | ~14 | `export.test.ts` (includes dump endpoint tests) |
| Contract | 2 | `export.test.ts` (inline/attachment header validation) |
| Smoke | 3 | `e2e.test.ts` or manual |

---

## Documentation Requirements

- No new documentation files are created by this phase.
- The spec file (`2026-03-30-config-export.md`) serves as the primary reference.
- API endpoints are self-documenting via the error messages and Content-Type headers.
- If a shared API docs file exists, add entries for:
  - `GET /api/export/resume/:id?format={pdf|markdown|latex|json}`
  - `GET /api/export/data?entities={comma-separated}`
  - `GET /api/export/dump`

---

## Parallelization Notes

**Within this phase:**
- T35.1 (slugify) can be done in parallel with T35.2 (types) and T35.3 (listAll methods).
- T35.4 (ExportService) depends on T35.2 and T35.3.
- T35.5 (wire services) depends on T35.4.
- T35.6 (routes) depends on T35.1, T35.4, T35.5.
- T35.7 (mount routes) depends on T35.6.
- T35.8 (remove legacy) is independent and can be done in parallel with T35.4-T35.7.
- T35.9 (SDK resource) is independent from T35.4-T35.7 (depends only on T35.2 for types).
- T35.10 (wire SDK client) depends on T35.9.
- T35.11 (config UI) depends on T35.10.
- T35.12 (resume list download) depends on T35.10.

**Critical path:** T35.2 -> T35.4 -> T35.5 -> T35.6 -> T35.7

**Across phases:**
- This phase can run in parallel with any phase that does not modify `packages/core/src/services/index.ts`, `packages/core/src/routes/server.ts`, or `packages/sdk/src/client.ts`.
- Phase 30 (Summaries) and Phase 31 (JD) are soft dependencies. If they land first, the `summaries` and `job_descriptions` export cases work. If not, the try/catch guards prevent runtime errors.
- Phase 29 (Config Profile) must complete before this phase starts, as the `/config` route namespace must be established.
