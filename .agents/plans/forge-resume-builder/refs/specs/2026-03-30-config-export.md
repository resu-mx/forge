# Config -- Export

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Resume IR Compiler, Resume Sections (Phases 27-28), LaTeX/PDF pipeline (Phase 20)

## Purpose

Provide export functionality for resumes and data. Resumes can already be compiled to LaTeX and rendered to PDF via tectonic, but there is no UI for triggering downloads and no way to export resumes as Markdown or JSON. There is also no way to export the underlying data entities (sources, bullets, perspectives, skills) for backup or portability beyond the `just dump` SQLite dump command.

This spec adds:
- Resume export in four formats (PDF, Markdown, LaTeX, JSON)
- Data export as JSON bundles
- Database backup as SQL dump
- A unified export UI at `/config/export`
- Download buttons on the resume builder page

## Goals

1. Export a resume as PDF, Markdown, LaTeX, or JSON (the full IR)
2. Export entities (sources, bullets, perspectives, skills, organizations) as JSON
3. Export the full database as a SQL dump
4. API endpoints for all export operations
5. Export UI at `/config/export`
6. Download button on the resume builder page

## Non-Goals

- Import from JSON (future spec)
- Export to DOCX or Google Docs format
- Scheduled/automated backups
- Cloud storage integration
- Authentication on export endpoints. All export endpoints (including `/api/export/dump`) have no auth guard and are expected to be local-only. If the webui is ever exposed beyond localhost, these endpoints will need authentication.

---

## 1. Resume Export Formats

### 1.1 PDF (existing)

Already implemented via `ResumeService.generatePDF()` which compiles IR to LaTeX via `compileToLatex()` and renders with tectonic. The export API wraps this existing capability.

### 1.2 LaTeX

Return the compiled LaTeX source. If the resume has a `latex_override`, return that. Otherwise compile IR to LaTeX.

```typescript
getLatex(resumeId: string): Result<string> {
  const resume = ResumeRepository.get(this.db, resumeId)
  if (!resume) return notFound(resumeId)

  if (resume.latex_override) return { ok: true, data: resume.latex_override }

  const ir = compileResumeIR(this.db, resumeId)
  if (!ir) return notFound(resumeId)
  return { ok: true, data: compileToLatex(ir, sb2nov) }
}
```

### 1.3 Markdown

Compile the IR to Markdown using the existing `compileToMarkdown()` function.

> **Note:** Both `compileToMarkdown()` (from `packages/core/src/lib/markdown-compiler.ts`) and `compileToLatex()` (from `packages/core/src/lib/latex-compiler.ts`) already exist. The `ExportService` imports them directly. No new compiler files are needed.

If the resume has a `markdown_override`, return that instead.

```typescript
getMarkdown(resumeId: string): Result<string> {
  const resume = ResumeRepository.get(this.db, resumeId)
  if (!resume) return notFound(resumeId)

  if (resume.markdown_override) return { ok: true, data: resume.markdown_override }

  const ir = compileResumeIR(this.db, resumeId)
  if (!ir) return notFound(resumeId)
  return { ok: true, data: compileToMarkdown(ir) }
}
```

### 1.4 JSON (IR)

Return the full `ResumeDocument` IR as JSON. This is the most complete export -- it captures the exact structure the UI renders, including provenance chain data.

```typescript
getJSON(resumeId: string): Result<ResumeDocument> {
  const ir = compileResumeIR(this.db, resumeId)
  if (!ir) return notFound(resumeId)
  return { ok: true, data: ir }
}
```

## 2. Data Export

Export entities as a JSON bundle. The user selects which entity types to include.

### 2.1 Exportable Entities

| Entity | What's included | JSON shape |
|--------|----------------|------------|
| `sources` | Source + all extension tables (roles, education, projects, clearances) | `{ sources: Source[] }` |
| `bullets` | Bullet + sources linkage + skills | `{ bullets: Bullet[] }` |
| `perspectives` | Perspective + skills | `{ perspectives: Perspective[] }` |
| `skills` | All skills with categories | `{ skills: Skill[] }` |
| `organizations` | Organizations + metadata | `{ organizations: Organization[] }` |
| `summaries` | Summaries (Spec 2) | `{ summaries: Summary[] }` |
| `job_descriptions` | Job descriptions (Spec 4) | `{ job_descriptions: JobDescription[] }` |

### 2.2 Bundle Format

The export returns a single JSON object with a metadata envelope:

```json
{
  "forge_export": {
    "version": "1.0",
    "exported_at": "2026-03-30T14:00:00Z",
    "entities": ["sources", "bullets", "skills"]
  },
  "sources": [ ... ],
  "bullets": [ ... ],
  "skills": [ ... ]
}
```

**Type definition** (add to `packages/core/src/types/index.ts`):

```typescript
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
  summaries?: Summary[]
  job_descriptions?: JobDescription[]
}
```

### 2.3 Implementation

**File:** `packages/core/src/services/export-service.ts`

```typescript
export class ExportService {
  constructor(private db: Database, private dbPath: string) {}

  // Note: The `Database` instance doesn't expose its file path. The `ExportService`
  // constructor receives `dbPath: string` as a separate parameter.
  //
  // Required wiring changes:
  // 1. `packages/core/src/services/index.ts`: Change `createServices(db: Database)` to
  //    `createServices(db: Database, dbPath: string)` and add `export: new ExportService(db, dbPath)`
  //    to the returned services object.
  // 2. `packages/core/src/index.ts`: Pass `dbPath` when calling `createServices(db, dbPath)`.

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
          bundle.sources = SourceRepository.listAll(this.db)
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
          bundle.skills = SkillRepository.listAll(this.db)
          resolved.push(entity)
          break
        case 'organizations':
          bundle.organizations = OrganizationRepository.listAll(this.db)
          resolved.push(entity)
          break
        case 'summaries':
          bundle.summaries = SummaryRepository.listAll(this.db)
          resolved.push(entity)
          break
        case 'job_descriptions':
          bundle.job_descriptions = JobDescriptionRepository.listAll(this.db)
          resolved.push(entity)
          break
        // Unknown entity names are silently ignored (no error, just excluded)
      }
    }

    return { ok: true, data: bundle }
  }

  async dumpDatabase(): Promise<Result<string>> {
    // Shell out to sqlite3 .dump (same as `just dump`)
    try {
      const proc = Bun.spawn(['sqlite3', this.dbPath, '.dump'], { stdout: 'pipe', stderr: 'pipe' })
      const stdout = await new Response(proc.stdout).text()
      const exitCode = await proc.exited
      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text()
        return { ok: false, error: { code: 'DUMP_FAILED', message: stderr } }
      }
      return { ok: true, data: stdout }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
        return { ok: false, error: { code: 'DUMP_FAILED', message: 'sqlite3 not found on PATH' } }
      }
      throw err
    }
  }
}
```

## 3. API Endpoints

### 3.1 Resume Export

**`GET /api/export/resume/:id`**

Query parameters:
- `format` (required): `pdf` | `markdown` | `latex` | `json`

Response varies by format:
- `pdf`: Binary response, `Content-Type: application/pdf`, `Content-Disposition: attachment; filename="resume.pdf"`
- `markdown`: Text response, `Content-Type: text/markdown`, `Content-Disposition: attachment; filename="resume.md"`
- `latex`: Text response, `Content-Type: application/x-latex`, `Content-Disposition: attachment; filename="resume.tex"`
- `json`: JSON response, `Content-Type: application/json`, `Content-Disposition: attachment; filename="resume.json"`. Returns `{ data: ResumeDocument }` (standard envelope, same as all other JSON API responses).

All responses include the `Content-Disposition: attachment` header to trigger a browser download.

> **Envelope contract for JSON export:** The JSON format returns the standard `{ data: ir }` envelope. The SDK's `resumeAsJson()` uses `this.request<ResumeDocument>()` which unwraps the envelope, so callers receive a `ResumeDocument` directly. For binary formats (PDF, Markdown, LaTeX), the SDK's `downloadResume()` uses raw `fetch()` and returns a `Blob`.

**Filename:** `{resume_name}-{date}.{ext}` (e.g., `agentic-ai-engineer-2026-03-30.pdf`). The resume name is slugified using: `name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')`. A `slugify(name: string): string` helper should be added to `packages/core/src/lib/utils.ts` (or colocated in the export route handler if the project has no shared utils module).

> **PDF Route Coexistence:** The existing `POST /api/resumes/:id/pdf` endpoint remains active for direct PDF generation from the resume builder. The new `GET /api/export/resume/:id?format=pdf` endpoint provides the same functionality via the export interface. Both call the same underlying tectonic pipeline. The old route is NOT deprecated -- it serves a different UX context (inline preview vs. download).

### 3.2 Data Export

**`GET /api/export/data`**

Query parameters:
- `entities` (required): comma-separated list of entity types. Valid values: `sources`, `bullets`, `perspectives`, `skills`, `organizations`, `summaries`, `job_descriptions`

Response: JSON bundle with `Content-Disposition: attachment; filename="forge-export-{date}.json"`

### 3.3 Database Dump

**`GET /api/export/dump`**

Response: SQL text, `Content-Type: application/sql`, `Content-Disposition: attachment; filename="forge-dump-{date}.sql"`

Requires `sqlite3` to be available on PATH (same dependency as `just dump`). Returns error if not found.

## 4. SDK

**File:** `packages/sdk/src/resources/export.ts`

```typescript
export class ExportResource {
  constructor(private request: RequestFn, private baseUrl: string) {}

  async resumeAsJson(id: string): Promise<Result<ResumeDocument>> {
    return this.request<ResumeDocument>('GET', `/api/export/resume/${id}?format=json`)
  }

  // For binary downloads (PDF, markdown, latex), use raw fetch
  async downloadResume(id: string, format: 'pdf' | 'markdown' | 'latex'): Promise<Result<Blob>> {
    // Similar to the existing pdf() method pattern in ResumesResource
    const response = await fetch(`${this.baseUrl}/api/export/resume/${id}?format=${format}`)
    if (!response.ok) {
      return { ok: false, error: { code: 'EXPORT_FAILED', message: response.statusText } }
    }
    const blob = await response.blob()
    return { ok: true, data: blob }
  }

  async exportData(entities: string[]): Promise<Result<DataExportBundle>> {
    return this.request<DataExportBundle>('GET', `/api/export/data?entities=${entities.join(',')}`)
  }
}
```

> **Note:** The `downloadResume` method for binary formats uses raw `fetch()` (same pattern as `ResumesResource.pdf()`). Browser-only download via `document.createElement('a')` is handled in the Svelte component, not the SDK.

## 5. UI: `/config/export`

**File:** `packages/webui/src/routes/config/export/+page.svelte`

Three sections:

### 5.1 Export Resume

- Dropdown to select a resume (loaded from `GET /api/resumes`)
- Format selector: radio buttons or segmented control (PDF / Markdown / LaTeX / JSON)
- Download button
- Status indicator: "Generating PDF..." spinner for PDF (which takes a few seconds via tectonic)

```svelte
<div class="export-section">
  <h2>Export Resume</h2>
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
        {#each ['pdf', 'markdown', 'latex', 'json'] as fmt}
          <button class:active={format === fmt} onclick={() => format = fmt}>
            {fmt.toUpperCase()}
          </button>
        {/each}
      </div>
    </label>
    <button class="btn btn-primary" onclick={downloadResume} disabled={exporting}>
      {exporting ? 'Generating...' : 'Download'}
    </button>
  </div>
</div>
```

### 5.2 Export Data

- Checkboxes for each entity type (Sources, Bullets, Perspectives, Skills, Organizations, Summaries, Job Descriptions)
- "Select All" / "Deselect All" buttons
- Download button

### 5.3 Database Backup

- Single button: "Download SQL Dump"
- Description text explaining what the dump includes

## 6. Resume Builder Download Button

**File:** `packages/webui/src/routes/resumes/[id]/+page.svelte` (or wherever the resume builder toolbar lives)

Add a download dropdown to the resume builder toolbar:

```svelte
<div class="toolbar-actions">
  <!-- existing buttons -->
  <div class="dropdown">
    <button class="btn btn-sm">Download</button>
    <div class="dropdown-menu">
      <button onclick={() => downloadResume('pdf')}>PDF</button>
      <button onclick={() => downloadResume('markdown')}>Markdown</button>
      <button onclick={() => downloadResume('latex')}>LaTeX</button>
      <button onclick={() => downloadResume('json')}>JSON (IR)</button>
    </div>
  </div>
</div>
```

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/services/export-service.ts` | Export service with data/dump methods |
| `packages/core/src/routes/export.ts` | Route handler for `/api/export/*` endpoints |
| `packages/sdk/src/resources/export.ts` | `ExportResource` class for SDK |
| `packages/webui/src/routes/config/export/+page.svelte` | Export config page |

> **Note:** `packages/core/src/lib/markdown-compiler.ts` and `packages/core/src/lib/latex-compiler.ts` already exist. They are imported by the `ExportService`, not created by this spec.

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/services/resume-service.ts` | Add `getMarkdown()`, `getLatex()`, `getJSON()` methods; add `import { compileToMarkdown } from '../lib/markdown-compiler'` |
| `packages/core/src/services/index.ts` | Change `createServices(db)` to `createServices(db, dbPath)`, add `export: new ExportService(db, dbPath)` to services object, re-export `ExportService` |
| `packages/core/src/index.ts` | Pass `dbPath` to `createServices(db, dbPath)` |
| `packages/core/src/types/index.ts` | Add export-related types (`DataExportBundle`, etc.) |
| `packages/sdk/src/client.ts` | Add `public export: ExportResource` property, instantiate as `this.export = new ExportResource(req, this.baseUrl)` in constructor |
| `packages/sdk/src/index.ts` | Export `ExportResource` class alongside other resource exports |
| `packages/webui/src/routes/resumes/[id]/+page.svelte` | Add download dropdown to toolbar |
| `packages/core/src/routes/resumes.ts` | Remove the `POST /api/resumes/:id/export` stub (currently returns 501) |
| `packages/sdk/src/resources/resumes.ts` | Remove or redirect the `export()` method that currently calls the old `GET /api/resumes/:id/export` route (now dead code, replaced by `ExportResource`) |
| `packages/core/src/db/repositories/source-repository.ts` | Add `listAll(db)` convenience method (or use existing `list(db, {}, 0, 999999)` as a pragmatic alternative) |
| `packages/core/src/db/repositories/bullet-repository.ts` | Add `listAll(db)` convenience method |
| `packages/core/src/db/repositories/perspective-repository.ts` | Add `listAll(db)` convenience method |
| `packages/core/src/db/repositories/skill-repository.ts` | Add `listAll(db)` convenience method |
| `packages/core/src/db/repositories/organization-repository.ts` | Add `listAll(db)` convenience method |
| `packages/core/src/server.ts` | Register export router from `packages/core/src/routes/export.ts` |

**Note on `listAll()`:** If adding dedicated `listAll()` methods is too much churn, a pragmatic alternative is to use existing `list(db, {}, 0, 999999)` with no filter and a high limit. The `listAll()` wrapper is preferred for clarity but not strictly required.

## Dependencies

Requires Spec 2 (Summaries) and Spec 4 (Job Descriptions) to be implemented first if `summaries` and `job_descriptions` are included in the exportable entity list. If those specs are not yet implemented, exclude those entities from the export.

> **Compile-time guard:** If Spec 2 or Spec 4 are not yet implemented, the `summaries` and `job_descriptions` cases in `exportData()` should be guarded with a `try/catch` around the repository call (to handle missing tables) or conditionally included based on migration state. This prevents runtime errors when the corresponding tables do not yet exist.

## Acceptance Criteria

1. Resume export works in all four formats (PDF, Markdown, LaTeX, JSON)
2. Data export includes all 7 entity types (sources, bullets, perspectives, skills, organizations, summaries, job_descriptions)
3. Database dump produces valid importable SQL
4. Export UI at `/config/export` renders with all three sections
5. Download button on resume builder page triggers export
6. Existing `POST /api/resumes/:id/export` stub removed (replaced by `GET /api/export/resume/:id`)

## Testing

- Verify PDF export generates a valid PDF via tectonic
- Verify Markdown export produces well-formatted Markdown for each section type
- Verify LaTeX export returns compilable LaTeX (use `latex_override` if present, otherwise compile from IR)
- Verify JSON export returns the full IR with provenance chain data
- Verify data export includes the correct entities based on query params
- Verify database dump produces valid SQL that can be re-imported
- Verify `Content-Disposition` headers trigger browser downloads
- Verify the download button on the resume builder page works
- Verify error handling when tectonic is not available (PDF only)
- Verify error handling when sqlite3 is not available (dump only)
- Verify `getMarkdown()` uses `markdown_override` when present
- Verify `getLatex()` uses `latex_override` when present
- Verify `dumpDatabase()` fails gracefully when `sqlite3` not on PATH
- Verify export data with unknown entity string is ignored (no error, just excluded)
- Verify export JSON IR matches `getIR()` response
- Verify `Content-Disposition` filename matches `{slug}-{YYYY-MM-DD}.{ext}` format (e.g., `agentic-ai-engineer-2026-03-30.pdf`)
- Verify existing `POST /api/resumes/:id/pdf` returns `Content-Disposition: inline` while new `GET /api/export/resume/:id?format=pdf` returns `Content-Disposition: attachment`
- Verify `forge_export.entities` in the data export bundle reflects only the entities actually populated (resolved list), not the raw requested list (e.g., requesting `["sources", "bogus"]` yields `entities: ["sources"]`)
