# Forge Resume Builder — Design Spec

**Date:** 2026-03-28
**Status:** Draft (v2 — post-review)
**Author:** Adam + Claude

## Overview

Forge is an AI-backed resume builder that breaks resume creation into auditable phases with controlled intermediates. The core problem it solves: when AI derives resume bullets from experience descriptions, it drifts from truth to plausible-sounding fabrication. Forge enforces a strict derivation chain (Source → Bullet → Perspective) where every artifact traces back to human-authored ground truth.

### Relationship to Existing System

Forge is a new system informed by the existing job-hunting toolkit (SQLite schema, Phase 1-6 alignment workflow, archetypes, AI signal detection). It does not wrap or replace the existing system. The existing schema and workflows serve as design input. Data migration from the existing `resume.sqlite.db` is post-MVP.

**Key lessons carried forward:**
- The 6 battle-tested archetypes: `agentic-ai`, `infrastructure`, `security-engineer`, `solutions-architect`, `public-sector`, `hft`
- Domain framing taxonomy: `systems_engineering`, `software_engineering`, `security`, `devops`, `ai_ml`, `leadership`
- The distinction between narrative framing (accomplishment/responsibility/context) and domain framing (what field the work is in) — these are orthogonal axes
- AI signal detection principles (no fabrication, no buzzword inflation, every claim must be defensible)
- The export pipeline produces Markdown as its interchange format; downstream LaTeX/PDF generation uses the existing `just export-resume` toolchain

## Goals

1. **Graphical UI** for managing resume data with better UX than CLI-only workflows
2. **Controlled intermediates** with deterministic, auditable sourcing to prevent hallucination
3. **CoreLib + SDK + Consumer pattern** enabling rapid addition of new interfaces (CLI, web, MCP, future Tauri)

---

## Architecture Decisions

### ADR-001: API-First

All consumers talk to core through the SDK, which wraps an HTTP API. No direct imports of core from consumers.

```
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│   CLI    │  │  WebUI   │  │   MCP    │  │  Future  │
│ consumer │  │ consumer │  │ (stub)   │  │ (Tauri)  │
└────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘
     │ uses        │ uses        │ uses        │ uses
┌────┴─────────────┴─────────────┴─────────────┴──────┐
│                  @forge/sdk                          │
│            typed fetch-based API client              │
└──────────────────────┬──────────────────────────────┘
                       │ HTTP
┌──────────────────────┴──────────────────────────────┐
│                  @forge/core                         │
│  ┌────────────┐  ┌──────────┐  ┌────────────────┐  │
│  │ Data Layer │  │    AI    │  │  HTTP Routes   │  │
│  │  (SQLite)  │  │ (Claude  │  │ (thin layer)   │  │
│  │            │  │   Code)  │  │                │  │
│  └────────────┘  └──────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Why:** Clean SDK boundary means all consumers use the same interface. Rust migration replaces what's behind the API — SDK, CLI, webui, MCP don't change. MCP server is just another SDK consumer.

**Trade-off accepted:** Slightly more boilerplate upfront (API routes + SDK client) and two local processes during development. Mitigated by `just dev` starting both.

### ADR-002: Bun-Native, No Tauri for MVP

**Why:** Tauri's backend is Rust, not TypeScript. TS is for development speed now; Rust rewrite comes later. Adding Tauri during the TS phase means either a Bun sidecar (complexity) or Rust business logic (premature). A local Bun server + browser gives the same GUI experience with zero wrapper overhead.

**Migration path:** When the Rust rewrite happens, Tauri adoption and CoreLib rewrite happen together. The SDK boundary stays the same.

### ADR-003: Svelte 5 + Vite for WebUI

**Why:** Compiler-based, minimal runtime, less boilerplate than React for CRUD views. Fastest path to a working data management UI. Runes (Svelte 5) provide fine-grained reactivity for the derivation review workflow.

### ADR-004: Claude Code CLI for AI Integration

**Why:** Wrapping Claude Code CLI (not the Claude API directly) for MVP. No API key management, no token billing logic. The AI module invokes `claude` in print mode with structured prompts and parses JSON output.

**Invocation pattern:**
```bash
claude -p "prompt text here" --output-format json
```

The `-p` flag runs Claude Code in non-interactive print mode (single prompt, single response, then exits). The `--output-format json` flag returns structured JSON output. This must be verified as stable during the first implementation task before building the full AI module.

**Timeout:** 60-second timeout per invocation. If Claude Code does not respond within 60s, the process is killed and the derivation request returns an error. The caller retries manually.

**Future:** Direct Claude API integration is a post-MVP enhancement that can replace the CLI wrapper behind the same service interface.

### ADR-005: Synchronous Derivation for MVP

**Why:** Async job queues add significant complexity (job table, polling, cleanup). For a single-user local tool, a synchronous HTTP request with a 60-second timeout is sufficient. The WebUI shows a loading spinner during derivation.

**Constraint:** Only one derivation can run at a time per source/bullet (enforced by `deriving` status — see Data Model). Concurrent derivation requests return 409 Conflict.

**Future:** Post-MVP adds async derivation with SSE/WebSocket progress streaming.

### ADR-006: Numbered SQL Migrations

**Why:** The schema will evolve through TS development and the eventual Rust rewrite. Manual schema changes are risky. Numbered SQL files provide reproducible, ordered migrations.

**Approach:** `packages/core/src/db/migrations/` contains files named `001_initial.sql`, `002_add_projects.sql`, etc. A `migrations` table tracks which have been applied. The core server runs pending migrations on startup.

### ADR-007: UUIDs as Primary Keys

**Why:** Better for API consumers, MCP integration, and distributed scenarios. TEXT type in SQLite with CHECK constraint for format validation.

**Implementation:** `crypto.randomUUID()` (available in Bun). All tables use `STRICT` mode to prevent type coercion. UUID columns use `CHECK(typeof(id) = 'text' AND length(id) = 36)`.

---

## Data Model

### Derivation Chain

```
Source ──1:N──► Bullet ──1:N──► Perspective
                                     │
                              used in │
                                     ▼
                               ResumeBullet (join: perspective + resume + section + position)
```

**Chain-of-custody rule:** Every Perspective must link to a Bullet. Every Bullet must link to a Source. The UI and API enforce this structurally — there is no way to create an orphaned Perspective. FK constraints use `ON DELETE RESTRICT` — a Source cannot be deleted while it has Bullets, a Bullet cannot be deleted while it has Perspectives.

**Content snapshots:** When a Bullet is derived from a Source, the Source's `description` at that moment is stored in `Bullet.source_content_snapshot`. When a Perspective is derived from a Bullet, the Bullet's `content` is stored in `Perspective.bullet_content_snapshot`. This preserves auditability even if the upstream entity is later edited.

### Entities

**Employers**
- `id` (uuid), `name`, `created_at`

**Projects**
- `id` (uuid), `name`, `employer_id?` (FK to employers), `description?`, `created_at`

**Sources** — human-authored ground truth descriptions of work performed
- `id` (uuid), `title`, `description` (NLP narrative)
- `employer_id?` (FK to employers), `project_id?` (FK to projects)
- `start_date?` (TEXT, ISO 8601), `end_date?` (TEXT, ISO 8601)
- `status`: `draft` | `approved` | `deriving` (locked while AI derivation is in progress)
- `created_at`, `updated_at`, `updated_by` (`human` | `ai`)
- `last_derived_at?` — timestamp of most recent bullet derivation
- Mutable by AI only when working on the source directly (e.g., human requests help refining)
- MVP scope: work-focused only. Post-MVP adds entity types for education, certifications, clearances, awards, publications, research.

**Bullets** — factual decompositions, AI-derived from a Source
- `id` (uuid), `source_id` (FK, required, ON DELETE RESTRICT)
- `content` (TEXT, required)
- `source_content_snapshot` (TEXT, required) — Source.description at derivation time
- `status`: `draft` | `pending_review` | `approved` | `rejected`
- `rejection_reason?` (TEXT) — why this was rejected, for audit trail
- `generated_prompt?` (TEXT) — the full prompt that produced this
- `created_at`, `approved_at?`, `approved_by?` (`human` — always human for MVP, field exists for future multi-user)
- AI-generated with explicit, required human review before approval
- A rejected bullet can be re-opened to `pending_review` status (e.g., rejected by mistake)

**Bullet Technologies** — junction table for queryable technology tags
- `bullet_id` (FK), `technology` (TEXT)
- Indexed on `technology` for efficient "all bullets mentioning Kubernetes" queries
- Replaces the JSON array approach for better SQLite queryability

**Perspectives** — role-targeted reframings, AI-derived from a Bullet
- `id` (uuid), `bullet_id` (FK, required, ON DELETE RESTRICT)
- `content` (TEXT, required)
- `bullet_content_snapshot` (TEXT, required) — Bullet.content at derivation time
- `target_archetype?` — one of: `agentic-ai`, `infrastructure`, `security-engineer`, `solutions-architect`, `public-sector`, `hft` (seeded from existing system; extensible)
- `domain?` — one of: `systems_engineering`, `software_engineering`, `security`, `devops`, `ai_ml`, `leadership` (carried from existing system's framing taxonomy)
- `framing`: `accomplishment` | `responsibility` | `context` (narrative structure)
- `status`: `draft` | `pending_review` | `approved` | `rejected`
- `rejection_reason?` (TEXT)
- `generated_prompt?` (TEXT)
- `created_at`, `approved_at?`, `approved_by?`
- A rejected perspective can be re-opened to `pending_review` status

**Skills**
- `id` (uuid), `name` (TEXT, unique), `category?` (TEXT)

**Bullet Skills** — junction table
- `bullet_id` (FK), `skill_id` (FK)

**Perspective Skills** — junction table
- `perspective_id` (FK), `skill_id` (FK)

**Resumes** — assembled from approved Perspectives
- `id` (uuid), `name` (TEXT), `target_role`, `target_employer`, `archetype`, `status` (`draft` | `final`)
- `created_at`, `updated_at`

**Resume Perspectives** — join table (renamed from ResumeBullets to avoid collision with existing schema's `resume_bullets`)
- `resume_id` (FK), `perspective_id` (FK)
- `section`: `summary` | `work_history` | `projects` | `education` | `skills` | `awards` (which resume section this appears in)
- `position` (INTEGER) — ordering within section
- Unique constraint on `(resume_id, perspective_id)` — a perspective appears once per resume

**Prompt Log** — normalized prompt storage
- `id` (uuid), `entity_type` (`bullet` | `perspective`), `entity_id` (uuid)
- `prompt_template` (TEXT) — the template name/version used
- `prompt_input` (TEXT) — the full rendered prompt sent to Claude
- `raw_response` (TEXT) — Claude's raw response
- `created_at`

This replaces the inline `generated_prompt` field on Bullets and Perspectives. The entities retain a `prompt_log_id?` FK for quick lookups, but the full prompt content lives in this table to avoid bloating the main entity tables.

### SQLite Configuration

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA strict = ON;
```

All tables use `STRICT` mode. UUIDs are TEXT with length checks. Enum fields use CHECK constraints (e.g., `CHECK(status IN ('draft', 'pending_review', 'approved', 'rejected'))`).

### Resume Generation Flow

1. User creates a resume with `target_role`, `target_employer`, and `archetype`
2. AI selects from pool of approved Perspectives matching the archetype
3. If no Perspective meets the requirement for the target role, AI either:
   a. Recommends generating a new Perspective from a relevant approved Bullet (specifying which Bullet and why)
   b. Flags the gap to the human for further review
4. The chain is always auditable: Perspective → Bullet → Source (including content snapshots at each derivation point)

### Gap Analysis Algorithm

Gap analysis answers: "What's missing from this resume relative to its target archetype?"

**Inputs:** Resume ID (which provides archetype, target_role, and the set of included perspectives)
**Process:**
1. Query all approved perspectives with `target_archetype` matching the resume's archetype
2. Query all skills associated with included perspectives (via perspective_skills)
3. Compare against the archetype's expected skill coverage (seeded from existing skills-inventory.md)
4. Identify:
   - **Missing archetypes:** The resume has zero perspectives for a domain relevant to the target role
   - **Thin coverage:** Fewer than N approved perspectives for a key domain
   - **Unused bullets:** Approved bullets that have no perspective for this archetype (derivation opportunity)

**Output shape:**
```json
{
  "resume_id": "uuid",
  "archetype": "agentic-ai",
  "gaps": [
    {
      "type": "missing_domain_coverage",
      "domain": "ai_ml",
      "available_bullets": ["uuid1", "uuid2"],
      "recommendation": "Derive perspectives with domain 'ai_ml' from these bullets"
    },
    {
      "type": "thin_coverage",
      "domain": "leadership",
      "current_count": 1,
      "recommendation": "Consider adding more leadership-framed perspectives"
    },
    {
      "type": "unused_bullet",
      "bullet_id": "uuid3",
      "source_title": "Cloud Forensics Platform Migration",
      "recommendation": "This bullet has no perspective for archetype 'agentic-ai'"
    }
  ],
  "coverage_summary": {
    "perspectives_included": 12,
    "skills_covered": ["python", "aws", "kubernetes"],
    "domains_represented": ["software_engineering", "devops"]
  }
}
```

Post-MVP: Add a `JobDescription` entity with `required_skills` to make gap analysis targeted against a specific JD rather than just archetype defaults.

---

## Core Internals

```
HTTP Routes (thin)  ──►  Services (business logic)  ──►  Repositories (data access)
                              │
                              ▼
                         AI Module (Claude Code CLI)
```

### Repositories — pure data access, one per entity

- `SourceRepository` — CRUD, search by employer/project/date, filter by status
- `BulletRepository` — CRUD, query by source, by status, by technology (via bullet_technologies join)
- `PerspectiveRepository` — CRUD, query by bullet, by archetype, by domain, by framing
- `ResumeRepository` — CRUD, assemble/reorder perspectives within sections
- `SkillRepository` — CRUD, query by category
- `PromptLogRepository` — append-only log, query by entity

All repositories use `bun:sqlite` (Bun's native SQLite driver). This dependency is scoped to `packages/core` only — no other package imports `bun:sqlite`.

### Services — business logic

- `SourceService` — create/update sources, validate status transitions, enforce `deriving` lock
- `DerivationService` — orchestrates Source→Bullet and Bullet→Perspective generation:
  1. Sets source/bullet status to `deriving` (lock)
  2. Calls AI module with structured prompt
  3. Validates AI output against expected schema
  4. Creates entities in `pending_review` status with content snapshots
  5. Stores prompt in PromptLog
  6. Resets source/bullet status from `deriving` back to previous status
  7. On AI failure: resets status, returns typed error, no partial records written
- `ResumeService` — assembles resumes from approved perspectives, runs gap analysis, recommends new perspective generation when gaps found
- `AuditService` — traces any perspective back through bullet→source (including content snapshots), validates chain integrity, generates audit reports showing what changed between snapshots and current content

### AI Module

**Implementation:** Spawns `claude -p "<prompt>" --output-format json` as a child process via Bun's `Bun.spawn()`. Reads stdout as JSON. Kills after 60-second timeout.

**Validation layer:** Before persisting any AI output:
1. Parse response as JSON — if malformed, return error
2. Validate against expected schema (e.g., bullets must have `content` string, optional `technologies` array)
3. Verify no content in the output that isn't traceable to the input (heuristic: flag if output contains proper nouns, company names, or technologies not present in the source/bullet)
4. If validation fails, return typed error with the raw response for debugging — never persist invalid output

**Prompt templates:**

Source → Bullet derivation:
```
You are a resume content assistant. Given a source description of work performed,
decompose it into factual bullet points. Each bullet must:
- State only facts present in the source description
- Include specific technologies, tools, or methods mentioned
- Include quantitative metrics if present in the source
- NOT infer, embellish, or add context not explicitly stated

Source description:
---
{source.description}
---

Respond with a JSON object:
{
  "bullets": [
    {
      "content": "factual bullet text",
      "technologies": ["tech1", "tech2"],
      "metrics": "quantitative metric if present, null otherwise"
    }
  ]
}
```

Bullet → Perspective derivation:
```
You are a resume content assistant. Given a factual bullet point, reframe it
for a target role archetype. The reframing must:
- Only use facts present in the original bullet
- Emphasize aspects relevant to the target archetype
- NOT add claims, technologies, outcomes, or context not in the bullet
- Use active voice, concise phrasing

Original bullet:
---
{bullet.content}
Technologies: {bullet.technologies}
Metrics: {bullet.metrics}
---

Target archetype: {archetype}
Target domain: {domain}
Framing style: {framing} (accomplishment | responsibility | context)

Respond with a JSON object:
{
  "content": "reframed bullet text",
  "reasoning": "brief explanation of what was emphasized and why"
}
```

These are starting templates. They will be refined through use. The `reasoning` field in perspective derivation is logged but not shown to the user by default — it exists for debugging prompt quality.

### HTTP API Surface

All responses follow a consistent envelope:

```typescript
// Success
{ "data": T }

// Error
{ "error": { "code": string, "message": string, "details"?: unknown } }

// List (paginated)
{ "data": T[], "pagination": { "total": number, "offset": number, "limit": number } }
```

**Status codes:**
- `200` — success (GET, PATCH)
- `201` — created (POST)
- `204` — deleted (DELETE)
- `400` — validation error (bad input, invalid status transition)
- `404` — entity not found
- `409` — conflict (e.g., derivation already in progress, duplicate)
- `501` — not implemented (stubbed endpoints)
- `504` — gateway timeout (AI derivation timed out)

**Pagination:** All list endpoints accept `?offset=0&limit=50` (default limit 50, max 200).

**Sources:**
- `POST /sources` — create → 201
- `GET /sources` — list (filterable by employer_id, project_id, status) → 200
- `GET /sources/:id` — get with linked bullet count and last_derived_at → 200
- `PATCH /sources/:id` — update → 200
- `DELETE /sources/:id` — delete (fails with 409 if has bullets) → 204
- `POST /sources/:id/derive-bullets` — trigger derivation (409 if already deriving) → 201 with created bullets

**Bullets:**
- `GET /bullets` — list (filterable by source_id, status, technology) → 200
- `GET /bullets/:id` — get with linked source and perspective count → 200
- `PATCH /bullets/:id` — update content → 200
- `DELETE /bullets/:id` — delete (fails with 409 if has perspectives) → 204
- `PATCH /bullets/:id/approve` — approve (400 if not pending_review) → 200
- `PATCH /bullets/:id/reject` — reject with required `rejection_reason` body (400 if not pending_review) → 200
- `PATCH /bullets/:id/reopen` — move from rejected back to pending_review → 200
- `POST /bullets/:id/derive-perspectives` — trigger derivation with `{ archetype, domain, framing }` body → 201

**Perspectives:**
- `GET /perspectives` — list (filterable by bullet_id, archetype, domain, framing, status) → 200
- `GET /perspectives/:id` — get with full chain (bullet + source + content snapshots) → 200
- `PATCH /perspectives/:id` — update content → 200
- `DELETE /perspectives/:id` — delete (fails with 409 if used in a resume) → 204
- `PATCH /perspectives/:id/approve` → 200
- `PATCH /perspectives/:id/reject` — with required `rejection_reason` → 200
- `PATCH /perspectives/:id/reopen` → 200

**Resumes:**
- `POST /resumes` — create → 201
- `GET /resumes` — list → 200
- `GET /resumes/:id` — get with assembled perspectives grouped by section → 200
- `PATCH /resumes/:id` — update metadata → 200
- `DELETE /resumes/:id` — delete (also deletes resume_perspectives join rows) → 204
- `POST /resumes/:id/perspectives` — add perspective to resume with `{ perspective_id, section, position }` → 201
- `DELETE /resumes/:id/perspectives/:perspective_id` — remove perspective from resume → 204
- `PATCH /resumes/:id/reorder` — reorder perspectives with `{ perspectives: [{ perspective_id, section, position }] }` → 200
- `GET /resumes/:id/gaps` — gap analysis → 200
- `POST /resumes/:id/export` — **501 Not Implemented** for MVP. Returns `{ error: { code: "NOT_IMPLEMENTED", message: "Export is not yet available. Use 'just export-resume' with the existing pipeline." } }`

**Review queue:**
- `GET /review/pending` — returns counts and items across all entity types in `pending_review` status → 200
  ```json
  {
    "data": {
      "bullets": { "count": 5, "items": [...] },
      "perspectives": { "count": 3, "items": [...] }
    }
  }
  ```

---

## SDK

`@forge/sdk` — typed fetch-based API client.

Package name: `@forge/sdk`. Consumed via Bun workspace protocol (`"@forge/sdk": "workspace:*"` in consumer package.json). Not published to npm for MVP.

### Error Contract

```typescript
type Result<T> =
  | { ok: true; data: T }
  | { ok: false; error: ForgeError }

interface ForgeError {
  code: string        // machine-readable: "NOT_FOUND", "CONFLICT", "VALIDATION_ERROR", etc.
  message: string     // human-readable description
  details?: unknown   // optional structured details (e.g., validation field errors)
}
```

All SDK methods return `Promise<Result<T>>`. Never throws. Network errors are caught and returned as `{ ok: false, error: { code: "NETWORK_ERROR", message: "..." } }`.

### Usage Examples

```typescript
const forge = new ForgeClient({ baseUrl: 'http://localhost:3000' })

// Create a source
const result = await forge.sources.create({ title: '...', description: '...' })
if (!result.ok) {
  console.error(result.error.code, result.error.message)
  return
}
const source = result.data

// Derive bullets — may take up to 60s
const bullets = await forge.sources.deriveBullets(source.id)
if (!bullets.ok && bullets.error.code === 'CONFLICT') {
  console.log('Derivation already in progress')
  return
}
if (!bullets.ok && bullets.error.code === 'GATEWAY_TIMEOUT') {
  console.log('AI timed out — try again')
  return
}

// Approve with typed result
const approved = await forge.bullets.approve(id)
if (!approved.ok && approved.error.code === 'VALIDATION_ERROR') {
  console.log('Cannot approve:', approved.error.message) // e.g., "Bullet is not in pending_review status"
}

// Reject with reason
const rejected = await forge.bullets.reject(id, { rejection_reason: 'Overstates impact' })

// List with filters and pagination
const pending = await forge.bullets.list({ status: 'pending_review', limit: 10, offset: 0 })

// Review queue
const queue = await forge.review.pending()

// Gap analysis
const gaps = await forge.resumes.gaps(resumeId)
```

---

## CLI

`packages/cli` — SDK consumer.

```
forge source add|list|edit|show|delete
forge source derive-bullets <source-id>

forge bullet list [--source <id>] [--status pending_review] [--technology <name>]
forge bullet show <id>
forge bullet approve <id>
forge bullet reject <id> --reason "..."
forge bullet reopen <id>
forge bullet derive-perspectives <bullet-id> --archetype <name> --domain <name> --framing <type>
forge bullet delete <id>

forge perspective list [--bullet <id>] [--archetype <name>] [--domain <name>]
forge perspective show <id>
forge perspective approve <id>
forge perspective reject <id> --reason "..."
forge perspective reopen <id>
forge perspective delete <id>

forge resume create|list|show|delete
forge resume add-perspective <resume-id> <perspective-id> --section <name> --position <n>
forge resume remove-perspective <resume-id> <perspective-id>
forge resume reorder <resume-id>
forge resume gaps <resume-id>
forge resume export <resume-id>          # 501 for MVP — prints message to use just export-resume

forge review                             # interactive review of pending items
```

**Naming convention:** CLI commands use entity-then-action, matching the API resource structure. `forge source derive-bullets` (not `forge bullet derive`) because derivation is an action on the source that produces bullets.

**`forge review` UX:**

```
── Pending Review (5 bullets, 3 perspectives) ──────────────

[Bullet 1/5] from source: "Cloud Forensics Platform Migration"

  "Migrated cloud forensics platform from on-prem ELK to AWS
   OpenSearch, reducing mean incident response time by 40%"

  Technologies: ELK, AWS OpenSearch
  Source snapshot matches current: ✓

  [a]pprove  [r]eject  [s]kip  [q]uit
  > _
```

**JSON output:** `--json` flag on all list/show commands for piping. `--json` on `forge review` disables interactive mode and outputs the pending queue as JSON.

**Exit codes:** 0 success, 1 general error, 2 validation error (e.g., bad arguments).

**Connection error handling:** If core server is not running, all commands print `Error: Cannot connect to Forge server at http://localhost:3000. Start it with 'just dev'.` and exit with code 1.

---

## WebUI

Svelte 5 SPA, imports `@forge/sdk`.

### Serving

- **Development:** Vite dev server on port 5173, proxies API calls to core on port 3000. `just dev` starts both.
- **Production:** `vite build` outputs static assets to `packages/webui/dist/`. Core server serves these at `/` and handles API routes at `/api/*`. Single process in production.

### MVP Views

1. **Sources list/editor** — create and edit source descriptions, see linked bullet count and `last_derived_at`. Filter by employer, project, status. Inline "Derive Bullets" button (shows spinner during derivation, disabled if status is `deriving`).

2. **Derivation view** — select a source, trigger bullet generation, review/approve/reject inline with rejection reason prompt. Select a bullet, trigger perspective generation with archetype/domain/framing selector, review/approve/reject inline. Shows content snapshots alongside current content to highlight drift.

3. **Chain view** — tree visualization of Source→Bullet→Perspective provenance for any entity. Click any node to see full content, content snapshot, and edit. Highlights chain integrity (snapshot matches current = green, diverged = yellow warning).

4. **Resume builder** — drag/arrange approved perspectives grouped by section, see gap analysis results inline, "Export" button shows 501 message with instructions to use `just export-resume`.

5. **Review queue** — dashboard landing page showing pending review counts and items. Click to navigate to the relevant derivation view for each item.

### State Management

- SDK calls wrapped in Svelte 5 runes for reactivity
- Loading states: skeleton placeholders during data fetches, spinner overlay during derivation (with "This may take up to 60 seconds" message)
- Error states: toast notifications for transient errors, inline error messages for validation errors
- Optimistic updates for approve/reject actions (revert on error)

No auth for MVP (local tool, single user). `approved_by` fields are set to `"human"` (constant).

---

## MCP Server (Post-MVP, Stubbed)

`packages/mcp` — SDK consumer, exposes tools for Claude Desktop / Cursor / Windsurf:
- `forge_search_sources`, `forge_derive_bullets`, `forge_review_pending`, etc.

Stub contains: package.json, empty src/index.ts with a comment describing the intended MCP tool surface.

---

## Monorepo Structure

```
forge/
├── packages/
│   ├── core/                  # Bun HTTP server + business logic
│   │   ├── src/
│   │   │   ├── db/
│   │   │   │   ├── migrations/      # 001_initial.sql, 002_xxx.sql, ...
│   │   │   │   ├── migrate.ts       # Migration runner
│   │   │   │   └── repositories/    # One file per entity
│   │   │   ├── services/            # Business logic, derivation chain
│   │   │   ├── ai/                  # Claude Code CLI wrapper + validation
│   │   │   ├── routes/              # HTTP route handlers
│   │   │   └── types/               # Shared type definitions
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── sdk/                   # Typed API client
│   │   ├── src/
│   │   │   ├── client.ts            # ForgeClient class
│   │   │   ├── types.ts             # Result<T>, ForgeError, entity types
│   │   │   └── index.ts             # Public API
│   │   └── package.json
│   ├── cli/                   # CLI consumer
│   │   ├── src/
│   │   └── package.json
│   ├── webui/                 # Svelte 5 SPA
│   │   ├── src/
│   │   ├── vite.config.ts
│   │   └── package.json
│   └── mcp/                   # Stubbed for post-MVP
│       ├── src/
│       │   └── index.ts             # Stub with tool surface comments
│       └── package.json
├── crates/
│   └── core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── db/                  # mirrors packages/core/src/db
│           ├── services/            # mirrors packages/core/src/services
│           ├── ai/                  # mirrors packages/core/src/ai
│           └── routes/              # mirrors packages/core/src/routes
├── docs/
│   └── src/
│       ├── adrs/
│       ├── data/models/
│       ├── architecture/
│       ├── mvp/
│       ├── api/
│       ├── sdk/
│       ├── lib/
│       ├── cli/
│       ├── webui/
│       └── mcp/
├── package.json               # Bun workspace root
├── bunfig.toml
├── Cargo.toml                 # Rust workspace root (members = ["crates/core"])
├── justfile                   # Dev commands
└── .env.example               # Environment variable template
```

### Workspace Configuration

**`package.json` (root):**
```json
{
  "name": "forge",
  "private": true,
  "workspaces": ["packages/*"]
}
```

**Package names:** `@forge/core`, `@forge/sdk`, `@forge/cli`, `@forge/webui`, `@forge/mcp`

**`Cargo.toml` (root):**
```toml
[workspace]
members = ["crates/core"]
```

### Environment Variables

Defined in `.env.example`, loaded by core on startup:

```bash
FORGE_PORT=3000              # Core HTTP server port
FORGE_DB_PATH=./data/forge.db  # SQLite database file path
FORGE_CLAUDE_PATH=claude     # Path to claude binary (default: found via $PATH)
FORGE_CLAUDE_TIMEOUT=60000   # AI derivation timeout in ms
FORGE_LOG_LEVEL=info         # debug | info | warn | error
```

### Justfile

```justfile
# Start core server + webui dev server
dev:
    bun run --filter '@forge/core' dev & \
    bun run --filter '@forge/webui' dev

# Run all tests
test:
    bun test --filter '@forge/core'
    bun test --filter '@forge/sdk'
    bun test --filter '@forge/cli'

# Check Rust stubs compile
check-rust:
    cargo check --workspace

# Run database migrations
migrate:
    bun run --filter '@forge/core' migrate

# Dump Forge database for backup
dump:
    sqlite3 $FORGE_DB_PATH .dump > data/forge-dump-$(date +%Y%m%d).sql

# Build all packages
build:
    bun run --filter '*' build
    cargo check --workspace
```

---

## MVP Scope

### In
- `packages/core` — SQLite data layer (with migrations), HTTP API, AI module (Claude Code CLI), derivation chain enforcement, content snapshots, concurrency locking
- `packages/sdk` — typed fetch client with `Result<T>` error contract
- `packages/cli` — CRUD commands, `forge review` interactive mode, `--json` output
- `packages/webui` — Svelte 5 SPA: source editor, derivation workflow, chain view, review queue, resume builder with gap analysis
- `crates/core/src/` — Rust module stubs with doc comments mapping to TS counterparts
- Documentation in `docs/src/`
- Single-user, local-only, no auth
- Export endpoint stubbed as 501

### Out (Post-MVP)
- MCP server (`packages/mcp` — stubbed only)
- Structured event extraction layer (between Source and Bullet)
- Direct Claude API integration (replace CLI wrapper)
- PDF export pipeline within Forge (use existing `just export-resume` externally)
- Tauri desktop wrapper
- Rust CoreLib implementation
- Auth, multi-user, remote deployment
- Data migration from existing SQLite schema (mapping: existing `bullets` → Forge Sources, existing `bullet_roles` + `bullet_skills` → metadata on Sources)
- Source entity types beyond work (education, certifications, clearances, awards, publications, research)
- JobDescription entity for targeted gap analysis
- Async derivation with SSE/WebSocket progress streaming
- Content versioning / edit history beyond snapshots

---

## Rust Stubs

Each Rust module in `crates/core/src/` mirrors its TypeScript counterpart with:
- Module file with doc comments describing the TS implementation
- Type stubs matching the TS types
- Function signatures with `todo!()` bodies
- Notes on design decisions and edge cases discovered during TS development

**Example stub** (`crates/core/src/db/source_repository.rs`):
```rust
//! Source repository — data access for Source entities.
//!
//! TS implementation: packages/core/src/db/repositories/source-repository.ts
//! Storage: SQLite via bun:sqlite (TS) / rusqlite (Rust)
//!
//! Design notes:
//! - Sources use UUID primary keys (TEXT in SQLite)
//! - `status` is an enum: draft | approved | deriving
//! - `deriving` status acts as a lock during AI derivation
//! - FK to employers and projects (both optional)

use crate::types::{Source, SourceFilter, CreateSource, UpdateSource};

/// Create a new source. Generates UUID, sets status to 'draft'.
pub fn create(input: CreateSource) -> Result<Source, RepoError> {
    todo!()
}

/// Get source by ID. Returns None if not found.
pub fn get(id: &str) -> Result<Option<Source>, RepoError> {
    todo!()
}

/// List sources with optional filters and pagination.
pub fn list(filter: SourceFilter, offset: u32, limit: u32) -> Result<Vec<Source>, RepoError> {
    todo!()
}

/// Update source fields. Returns error if source not found.
/// Note: If source has derived bullets and description changes,
/// existing bullets' source_content_snapshot will diverge from
/// current content — this is intentional and the chain view
/// highlights the divergence.
pub fn update(id: &str, input: UpdateSource) -> Result<Source, RepoError> {
    todo!()
}

/// Delete source. Fails if source has any bullets (ON DELETE RESTRICT).
pub fn delete(id: &str) -> Result<(), RepoError> {
    todo!()
}
```

---

## Examples

### End-to-End Derivation Chain

**Source (human-authored):**
> "Led a team of 4 engineers to migrate Raytheon's cloud forensics platform from on-prem ELK to AWS OpenSearch. The migration took 6 months and reduced mean incident response time by 40%. Used Terraform for infrastructure, GitLab CI/CD for deployment automation, and built custom Python log parsers for format translation."

**Derived Bullets (AI-generated, human-approved):**
1. "Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch over 6 months" (technologies: ELK, AWS OpenSearch; metrics: 4 engineers, 6 months)
2. "Reduced mean incident response time by 40% through platform migration" (metrics: 40% reduction)
3. "Built infrastructure automation with Terraform and GitLab CI/CD for deployment pipeline" (technologies: Terraform, GitLab CI/CD)
4. "Developed custom Python log parsers for format translation between ELK and OpenSearch" (technologies: Python)

**Derived Perspectives (AI-generated, human-approved):**

From Bullet #1, archetype `agentic-ai`, domain `ai_ml`, framing `accomplishment`:
> "Led cloud platform migration enabling ML-based log analysis pipeline on AWS OpenSearch"

From Bullet #1, archetype `infrastructure`, domain `devops`, framing `accomplishment`:
> "Led 4-engineer team delivering 6-month cloud forensics platform migration from ELK to AWS OpenSearch"

From Bullet #2, archetype `security-engineer`, domain `security`, framing `accomplishment`:
> "Reduced incident response time 40% through forensics platform modernization"

### Chain Provenance

For any Perspective, the audit trail shows:
```
Perspective: "Led cloud platform migration enabling ML-based log analysis pipeline on AWS OpenSearch"
  ├── bullet_content_snapshot: "Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch over 6 months"
  ├── Bullet (current): "Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch over 6 months"
  │   ├── source_content_snapshot: "Led a team of 4 engineers to migrate Raytheon's cloud forensics..."
  │   └── Source (current): "Led a team of 4 engineers to migrate Raytheon's cloud forensics..."
  └── Prompt log: template=bullet-to-perspective, archetype=agentic-ai, domain=ai_ml
```

If the source was later edited, the chain view shows the divergence between snapshot and current.

---

## Acceptance Criteria

### `packages/core`
- [ ] All HTTP route groups return correct status codes with valid response bodies
- [ ] Derivation chain FK enforcement tested: cannot create Bullet without valid `source_id`, cannot create Perspective without valid `bullet_id`
- [ ] `ON DELETE RESTRICT` prevents deleting entities with dependents
- [ ] Status transitions enforced: `draft → pending_review → approved/rejected`, no skipping. `rejected → pending_review` (reopen) allowed.
- [ ] `deriving` lock prevents concurrent derivation on the same source/bullet
- [ ] Content snapshots captured at derivation time
- [ ] AI module validates output schema before persisting — malformed responses never create records
- [ ] Prompt log captures full prompt input and raw response
- [ ] `bun test` passes for all repositories and services
- [ ] Server starts cleanly from `just dev` with automatic migration
- [ ] SQLite PRAGMA foreign_keys = ON verified on every connection
- [ ] Pagination works on all list endpoints

### `packages/sdk`
- [ ] Every API endpoint has a corresponding typed SDK method
- [ ] All methods return `Result<T>` — never throws
- [ ] Network errors return `{ ok: false, error: { code: "NETWORK_ERROR" } }`
- [ ] SDK works in browser (WebUI) and Bun (CLI) runtimes — uses only `fetch`, no Node/Bun builtins
- [ ] TypeScript compiles with `strict: true`, zero errors

### `packages/cli`
- [ ] All commands listed in CLI section are implemented
- [ ] `--json` flag works on all list/show commands
- [ ] `forge review` walks through pending items with approve/reject/skip controls
- [ ] `forge review` displays content snapshot match status
- [ ] Exit codes: 0 success, 1 error, 2 validation error
- [ ] Connection failure shows clear message with `just dev` hint

### `packages/webui`
- [ ] All 5 MVP views navigable
- [ ] End-to-end workflow: create source → derive → review → approve bullets → derive perspectives → approve → assemble resume → view gaps
- [ ] Chain view renders provenance with snapshot divergence highlighting
- [ ] Resume builder drag/reorder persists via API
- [ ] Loading spinner during derivation with timeout message
- [ ] Error toasts for transient errors, inline messages for validation
- [ ] No hardcoded localhost — base URL from environment/config

### Data model
- [ ] Schema migration runs idempotently (no errors on fresh or existing database)
- [ ] `PRAGMA foreign_keys = ON` enforced
- [ ] UUID generation consistent (`crypto.randomUUID()`)
- [ ] All enum fields have CHECK constraints
- [ ] `STRICT` mode on all tables

---

## Test Cases

### Unit — Repositories
- `SourceRepository.create` rejects null `description` → validation error
- `BulletRepository.create` rejects nonexistent `source_id` → FK violation
- `PerspectiveRepository.create` rejects nonexistent `bullet_id` → FK violation
- `BulletRepository.listByTechnology("kubernetes")` returns correct results via junction table
- `ResumeRepository` reorder updates positions correctly within sections

### Unit — Services
- `DerivationService.deriveBulletsFrom(sourceId)` creates bullets in `pending_review` status
- `DerivationService` sets source status to `deriving` during derivation, resets after
- `DerivationService` stores prompt in PromptLog on success
- `DerivationService` returns error and creates no records when AI output is malformed
- `DerivationService` returns 409 when source is already in `deriving` status
- `SourceService.update` on a source after bullets exist — allowed, but chain view shows divergence
- Status transition: `approved → pending_review` blocked, `rejected → pending_review` (reopen) allowed
- `AuditService.traceChain(perspectiveId)` returns full chain with snapshots
- `ResumeService.gaps(resumeId)` identifies missing domain coverage

### Integration — API
- `POST /sources` + `POST /sources/:id/derive-bullets` → bullets linked to source with snapshots
- `PATCH /bullets/:id/approve` on `pending_review` → 200; on `draft` → 400
- `PATCH /bullets/:id/reject` without `rejection_reason` → 400
- `DELETE /sources/:id` with existing bullets → 409
- `DELETE /sources/:id` with no bullets → 204
- `GET /perspectives/:id` returns nested bullet + source + snapshots
- `POST /resumes/:id/export` → 501 with helpful message
- `GET /review/pending` returns correct counts across entity types
- Concurrent `POST /sources/:id/derive-bullets` → first succeeds, second gets 409

### SDK
- `forge.bullets.approve(nonexistentId)` returns `{ ok: false, error: { code: "NOT_FOUND" } }`
- SDK handles network errors (server down) → `NETWORK_ERROR` result, not thrown exception
- Trailing slash in `baseUrl` handled correctly
- Pagination parameters pass through correctly

### End-to-End
- Full derivation chain: create source → derive bullets → approve one → derive perspectives → approve → create resume → add perspective → verify chain view
- Rejection flow: derive bullets → reject all → verify no perspectives can be derived from rejected bullets (they can — rejection is on the bullet, derivation reads content regardless of status. But the UI should warn.)
- Gap analysis: create resume targeting `agentic-ai` → verify gaps for domains with no perspectives
- AI failure: mock claude to return garbage → verify no records created, typed error returned
- Content drift: create source → derive bullets → edit source description → verify chain view shows divergence between snapshot and current

---

## Parallelization & Dependencies

### Critical Path

```
Shared Types ──► SQLite Schema + Migrations ──► Repositories ──► Services ──► HTTP Routes ──► SDK ──► (CLI | WebUI)
```

### Parallel Tracks

Once shared types and API surface are locked:

| Track | Work | Requires | Can Start |
|-------|------|----------|-----------|
| **A** | Core: schema, repos, services, routes | — | Immediately |
| **B** | SDK: typed client, error types | API surface spec (this doc) | Immediately (build against spec) |
| **C** | CLI: all commands | SDK types | After SDK types defined (can mock SDK) |
| **D** | WebUI: all views | SDK types | After SDK types defined (can mock SDK) |
| **E** | Rust stubs | TS types finalized | After Track A completes |
| **F** | Docs: ADRs, API docs, model docs | — | Immediately |
| **G** | AI module verification | `claude -p` flag stability | Immediately (independent spike) |

**Track G is a risk gate:** If `claude -p --output-format json` is not stable or does not behave as expected, the AI module design must be revised before Track A's service layer is built. Run this verification as the first task.

**Tracks C and D** can build against mock SDK responses in parallel with Track A. Integration testing happens when A + B converge.

**Rust stubs (Track E)** are off the critical path entirely and should not gate any MVP milestone.
