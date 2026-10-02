# Forge MCP Server — Design Spec

**Date:** 2026-04-03 (revised)
**Status:** Draft
**Package:** `@forge/mcp`
**Transport:** STDIO (v1), SSE/Streamable HTTP (future)
**Primary Consumer:** Claude Code → Claude Desktop → other MCP clients
**Interface Layer:** MCP Server → `@forge/sdk` → HTTP API → `@forge/core`

## Design Principles

1. **SDK delegation** — Every MCP tool delegates to `@forge/sdk`. No direct DB or service access.
2. **AI reasoning stays with client** — MCP returns structured data. The calling AI does the thinking.
3. **Programmatic alignment** — Embedding-based similarity for JD↔Resume matching. No LLM needed.
4. **Resources for context, tools for actions** — Static/slow-changing reference data as resources; parameterized queries and mutations as tools.
5. **No delete tools** — Destructive operations are intentionally omitted from the MCP surface. Humans delete via WebUI or CLI. This prevents AI agents from accidentally destroying provenance chains.

---

## MCP Resources

Resources provide ambient context that the AI client loads without explicit tool calls.

| URI Pattern | SDK Method | Returns | Update Frequency |
|---|---|---|---|
| `forge://profile` | `sdk.profile.get()` | UserProfile (name, contact, clearance) | Rarely changes |
| `forge://archetypes` | `sdk.archetypes.list()` | Archetype[] with domain associations | Config-level, rarely |
| `forge://domains` | `sdk.domains.list()` | Domain[] (skill domain taxonomy) | Config-level, rarely |
| `forge://templates` | `sdk.templates.list()` | ResumeTemplate[] (section structures) | Occasionally |
| `forge://resume/{id}` | `sdk.resumes.get(id)` | ResumeWithEntries (sections, entries, snapshots) | Per-session working state |
| `forge://resume/{id}/ir` | `sdk.resumes.ir(id)` | ResumeDocument IR (compiled view for rendering) | Changes as entries are added |
| `forge://job/{id}` | `sdk.jobDescriptions.get(id)` | JobDescriptionWithOrg (title, raw_text, status) | Set once, read many |

### Resource Subscriptions (SSE only — future)

Resource subscriptions (`resources/subscribe`) for `forge://resume/{id}` are deferred to the SSE transport phase. STDIO is request/response and does not support server-initiated push notifications reliably.

**STDIO polling pattern:** After any mutating tool call that affects a resume (add/remove entry, reorder, update header), the AI client should re-read the `forge://resume/{id}` resource to get the updated state.

---

## MCP Tools

### Tier 0: Diagnostics

#### `forge_health`
Check connectivity to the Forge HTTP server.
```
Parameters: none
Returns: { server: 'ok', version: string } or descriptive error
SDK: sdk.request('GET', '/api/health')
Note: Call this at session start to verify the Forge server is running. If it returns
  a connection error, the AI should tell the user to start the Forge server before
  proceeding.
```

---

### Tier 1: Core Resume Workflow

These tools support the primary flow: JD → find matches → build resume → check alignment → export.

#### `forge_search_sources`
Search experience sources by employer, type, date range, status.
```
Parameters:
  source_type?: 'role' | 'project' | 'education' | 'clearance' | 'general'
  status?: 'draft' | 'approved'
  search?: string (full-text on title + description)
  offset?: number (default 0)
  limit?: number (default 20)
Returns: Source[] with extension data (role/project/education/clearance fields)
SDK: sdk.sources.list(filter)
Note: Sources with status 'deriving' are locked by an in-progress AI derivation.
  Do not call forge_derive_bullets on them. Use forge_review_pending to check
  if derivation has completed.
```

#### `forge_get_source`
Get a single source by ID with full extension data and associated bullets.
```
Parameters:
  source_id: string (required)
Returns: SourceWithBullets (source + extension fields + bullets[])
SDK: sdk.sources.get(source_id)
```

#### `forge_search_bullets`
Search bullet inventory by domain, status, source, content.
```
Parameters:
  domain?: string
  status?: 'draft' | 'pending_review' | 'approved' | 'rejected'
  source_id?: string
  search?: string (full-text on content)
  offset?: number (default 0)
  limit?: number (default 20)
Returns: Bullet[] where each bullet has sources: {id, title, is_primary}[] and technologies[]
SDK: sdk.bullets.list(filter)
```

#### `forge_get_bullet`
Get a single bullet by ID with relations.
```
Parameters:
  bullet_id: string (required)
Returns: BulletWithRelations (bullet + sources[] + technologies[])
SDK: sdk.bullets.get(bullet_id)
```

#### `forge_search_perspectives`
Search perspectives by archetype, domain, framing, status.
```
Parameters:
  archetype?: string
  domain?: string
  framing?: 'accomplishment' | 'responsibility' | 'context'
  status?: 'draft' | 'pending_review' | 'approved' | 'rejected'
  search?: string
  offset?: number (default 0)
  limit?: number (default 20)
Returns: Perspective[] with bullet and source chain
SDK: sdk.perspectives.list(filter)
```

#### `forge_get_perspective`
Get a single perspective by ID with full provenance chain.
```
Parameters:
  perspective_id: string (required)
Returns: PerspectiveWithChain {perspective, bullet, source}
SDK: sdk.perspectives.get(perspective_id)
```

#### `forge_derive_bullets`
Trigger AI bullet derivation from a source. Returns generated bullets in pending_review status.
```
Parameters:
  source_id: string (required)
Returns: Bullet[] (newly derived, status=pending_review)
SDK: sdk.sources.deriveBullets(source_id)
Note: This calls Forge's AI module (Claude CLI). The MCP CLIENT is not doing the
  reasoning here — Forge's derivation service handles it. This is acceptable because
  bullet derivation is a structured extraction task, not a reasoning task that benefits
  from conversation context.
  The source must be in 'approved' or 'draft' status. If the source is 'deriving',
  this call will return a CONFLICT error — wait and retry.
```

#### `forge_derive_perspective`
Trigger AI perspective derivation from an approved bullet.
```
Parameters:
  bullet_id: string (required)
  archetype: string (required — target archetype slug)
  domain: string (required — target domain slug)
  framing: 'accomplishment' | 'responsibility' | 'context' (required)
Returns: Perspective (newly derived, status=pending_review)
SDK: sdk.bullets.derivePerspectives(bullet_id, {archetype, domain, framing})
Note: The bullet must be in 'approved' status. Only approved bullets can derive
  perspectives. Same AI module rationale as forge_derive_bullets.
```

#### `forge_approve_bullet`
Approve a bullet (pending_review → approved).
```
Parameters:
  bullet_id: string (required)
Returns: Bullet (updated)
SDK: sdk.bullets.approve(bullet_id)
```

#### `forge_reject_bullet`
Reject a bullet with reason (pending_review → rejected).
```
Parameters:
  bullet_id: string (required)
  rejection_reason: string (required)
Returns: Bullet (updated)
SDK: sdk.bullets.reject(bullet_id, {rejection_reason})
```

#### `forge_approve_perspective`
Approve a perspective (pending_review → approved).
```
Parameters:
  perspective_id: string (required)
Returns: Perspective (updated)
SDK: sdk.perspectives.approve(perspective_id)
```

#### `forge_reject_perspective`
Reject a perspective with reason.
```
Parameters:
  perspective_id: string (required)
  rejection_reason: string (required)
Returns: Perspective (updated)
SDK: sdk.perspectives.reject(perspective_id, {rejection_reason})
```

#### `forge_create_resume`
Create a new resume, optionally from a template.
```
Parameters:
  name: string (required)
  target_role: string (required)
  target_employer: string (required)
  archetype: string (required — archetype slug)
  template_id?: string (creates with predefined sections)
Returns: ResumeWithEntries (includes pre-populated sections if template used)
SDK:
  if template_id provided:
    sdk.templates.createResumeFromTemplate({template_id, name, target_role, target_employer, archetype})
  else:
    sdk.resumes.create({name, target_role, target_employer, archetype})
```

#### `forge_add_resume_entry`
Add an approved perspective to a resume section.
```
Parameters:
  resume_id: string (required)
  section_id: string (required — UUID FK to resume_sections)
  perspective_id: string (required — must be status 'approved')
  position?: number
Returns: ResumeEntry
SDK: sdk.resumes.addEntry(resume_id, {section_id, perspective_id, position})
Note: Only approved perspectives can be added. Attempting to add a non-approved
  perspective returns a VALIDATION_ERROR.
```

#### `forge_create_resume_section`
Create a section in a resume (experience, skills, education, etc).
```
Parameters:
  resume_id: string (required)
  title: string (required — e.g., "Professional Experience", "Technical Skills")
  entry_type: 'experience' | 'skills' | 'education' | 'projects' | 'certifications' | 'clearance' | 'presentations' | 'awards' | 'freeform' (required)
  position?: number
Returns: ResumeSectionEntity
SDK: sdk.resumes.createSection(resume_id, {title, entry_type, position})
Note: 'freeform' sections contain unstructured text content (user-defined sections
  that don't fit the standard categories).
```

#### `forge_gap_analysis`
Get domain coverage gaps for a resume vs. its archetype's expected domains.
```
Parameters:
  resume_id: string (required)
Returns: GapAnalysis {
  covered_domains: string[],
  missing_domains: string[],
  thin_domains: string[],
  entry_count_by_domain: Record<string, number>
}
SDK: sdk.resumes.gaps(resume_id)
Note: A domain is "thin" if it has fewer than 2 approved perspectives covering it
  (THIN_COVERAGE_THRESHOLD = 2 in core constants). Missing means zero perspectives
  for a domain the archetype expects. Covered means >= 2 perspectives.

Example response:
  {
    "covered_domains": ["infrastructure", "security", "ai_ml"],
    "missing_domains": ["data_systems"],
    "thin_domains": ["cloud"],
    "entry_count_by_domain": {
      "infrastructure": 4,
      "security": 3,
      "ai_ml": 2,
      "cloud": 1,
      "data_systems": 0
    }
  }
```

#### `forge_export_resume`
Export resume in specified format.
```
Parameters:
  resume_id: string (required)
  format: 'json' | 'markdown' | 'latex' | 'pdf'
Returns:
  json → ResumeDocument (full IR as JSON object)
  markdown → string (markdown content)
  latex → string (LaTeX source)
  pdf → { file_path: string } (server writes PDF to disk, returns path)
SDK:
  json → sdk.export.resumeAsJson(resume_id)
  markdown/latex → sdk.export.downloadResume(resume_id, format) → extract text
  pdf → sdk.export.downloadResume(resume_id, 'pdf') → write to temp file, return path
Note: For PDF, the MCP server writes the binary to a temp file and returns the path.
  The AI client can then reference this path for the user to open.
```

#### `forge_align_resume` (NEW — requires embedding service)
Programmatic JD↔Resume alignment using embedding similarity.
```
Parameters:
  job_description_id: string (required)
  resume_id: string (required)
  strong_threshold?: number (default 0.75 — similarity above this = 'strong' match)
  adjacent_threshold?: number (default 0.50 — similarity between this and strong = 'adjacent')
Returns: AlignmentReport (see Embedding Service section for full type)
SDK: sdk.alignment.score(job_description_id, resume_id, {strong_threshold?, adjacent_threshold?})
Route: GET /api/alignment/score?jd_id={jd_id}&resume_id={resume_id}&strong_threshold=0.75&adjacent_threshold=0.50
Note: This is a programmatic embedding-similarity computation, not an AI call.
  Requires the embedding service to have vectors for both the JD requirements and
  the resume's perspective entries. If embeddings are missing, returns an error
  indicating which entities need embedding.
```

#### `forge_match_requirements` (NEW — pre-resume discovery)
Match JD requirements against the full bullet or perspective inventory, independent of any resume.
```
Parameters:
  job_description_id: string (required)
  entity_type: 'bullet' | 'perspective' (required)
  threshold?: number (default 0.50)
  limit?: number (default 10 matches per requirement)
Returns: RequirementMatchReport {
  job_description_id: string,
  matches: Array<{
    requirement_text: string,
    candidates: Array<{entity_id: string, content: string, similarity: number}>
  }>
}
SDK: sdk.alignment.matchRequirements(job_description_id, entity_type, {threshold, limit})
Route: GET /api/alignment/match?jd_id={jd_id}&entity_type=perspective&threshold=0.50
Note: Use this BEFORE creating a resume to discover which approved perspectives
  best match a JD. Helps the AI decide what content to pull into a new resume.
```

---

### Tier 2: Data Management

Tools for populating and maintaining the data that feeds resume building.

#### `forge_create_source`
Create a new experience source.
```
Parameters:
  title: string (required)
  description: string (required)
  source_type: 'role' | 'project' | 'education' | 'clearance' | 'general' (required)
  start_date?: string
  end_date?: string
  notes?: string
  # Type-specific extension fields:
  organization_id?: string (for role/project/education)
  is_current?: boolean (for role)
  work_arrangement?: string (for role)
  education_type?: 'degree' | 'certificate' | 'course' | 'self_taught' (for education)
  degree_level?: string (for education)
  field?: string (for education)
  level?: string (for clearance)
  polygraph?: string (for clearance)
Returns: Source with extension data
SDK: sdk.sources.create(input)
```

#### `forge_create_organization`
Create an organization (employer, school, etc).
```
Parameters:
  name: string (required)
  org_type: 'company' | 'nonprofit' | 'government' | 'military' | 'education' | 'volunteer' | 'freelance' | 'other' (required)
  tags?: OrgTag[] (primary classification mechanism — e.g., ['employer', 'defense', 'remote'])
  industry?: string
  website?: string
  location?: string
  status?: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded' | null
  notes?: string
Returns: Organization
SDK: sdk.organizations.create(input)
```

#### `forge_ingest_job_description`
Create/store a job description. Triggers programmatic requirement parsing and embedding.
```
Parameters:
  title: string (required)
  raw_text: string (required — full JD text)
  organization_id?: string
  url?: string
  status?: 'interested' | 'analyzing' | 'applied' | 'interviewing' | 'offered' | 'rejected' | 'withdrawn' | 'closed' (default 'interested')
  salary_range?: string
  location?: string
  notes?: string
Returns: JobDescriptionWithOrg & {
  embedding_status: {
    requirements_parsed: number,
    requirements_embedded: number,
    ready_for_alignment: boolean,
    parse_confidence: number  // 0.0-1.0, low = may need manual review
  }
}
SDK: sdk.jobDescriptions.create(input)
Note: After creation, the embedding service parses requirements from raw_text and
  computes embeddings. The embedding_status in the return tells the AI whether the
  JD is ready for alignment scoring. If parse_confidence is low (< 0.7), the AI
  should suggest the user review the parsed requirements.
```

#### `forge_extract_jd_skills`
Trigger AI-powered skill extraction from a job description.
```
Parameters:
  job_description_id: string (required)
Returns: Array<{skill_name: string, confidence: number, matched_skill_id?: string}>
SDK: sdk.jobDescriptions.extractSkills(job_description_id)
Note: Returns suggested skills with confidence scores. Skills that match existing
  entries in the skills table include matched_skill_id. The AI can then use
  forge_tag_jd_skill to confirm associations.
```

#### `forge_tag_jd_skill`
Associate a skill with a job description (confirm an extracted skill).
```
Parameters:
  job_description_id: string (required)
  skill_id: string (required)
Returns: void
SDK: sdk.jobDescriptions.addSkill(job_description_id, skill_id)
```

#### `forge_untag_jd_skill`
Remove a skill association from a job description.
```
Parameters:
  job_description_id: string (required)
  skill_id: string (required)
Returns: void
SDK: sdk.jobDescriptions.removeSkill(job_description_id, skill_id)
```

#### `forge_link_resume_to_jd`
Link a resume to a job description (tracks which resume targets which job).
```
Parameters:
  job_description_id: string (required)
  resume_id: string (required)
Returns: void
SDK: sdk.jobDescriptions.linkResume(job_description_id, resume_id)
```

#### `forge_unlink_resume_from_jd`
Remove the link between a resume and a job description.
```
Parameters:
  job_description_id: string (required)
  resume_id: string (required)
Returns: void
SDK: sdk.jobDescriptions.unlinkResume(job_description_id, resume_id)
```

#### `forge_create_summary`
Create a resume summary.
```
Parameters:
  title: string (required)
  role?: string
  tagline?: string
  description?: string
  is_template?: boolean
Returns: Summary
SDK: sdk.summaries.create(input)
```

#### `forge_create_skill`
Create a skill entry.
```
Parameters:
  name: string (required)
  category?: string
Returns: Skill
SDK: sdk.skills.create(input)
```

#### `forge_update_profile`
Update the user profile (singleton).
```
Parameters:
  name?: string
  email?: string
  phone?: string
  location?: string
  linkedin?: string
  github?: string
  website?: string
  clearance?: string
Returns: UserProfile
SDK: sdk.profile.update(input)
```

#### `forge_update_resume_header`
Set resume header fields (headline, contact override).
```
Parameters:
  resume_id: string (required)
  header: {headline?, name?, email?, phone?, location?, linkedin?, github?, website?}
Returns: Resume
SDK: sdk.resumes.updateHeader(resume_id, header)
```

#### `forge_set_resume_summary`
Link a summary to a resume.
```
Parameters:
  resume_id: string (required)
  summary_id: string (required)
Returns: Resume
SDK: sdk.resumes.update(resume_id, {summary_id})
```

#### `forge_review_pending`
Get all bullets and perspectives awaiting review.
```
Parameters: none
Returns: ReviewQueue {bullets: Bullet[], perspectives: Perspective[]}
SDK: sdk.review.pending()
```

#### `forge_check_drift`
Detect stale content snapshots in the derivation chain and stale embeddings.
```
Parameters: none
Returns: DriftedEntity[] {entity_type, entity_id, snapshot_value, current_value}
SDK: sdk.integrity.drift()
Note: Includes both snapshot drift (source→bullet→perspective content changes)
  and embedding drift (content changed since vector was computed).
```

#### `forge_search_organizations`
Search organizations by name, type, status, tags.
```
Parameters:
  search?: string
  org_type?: 'company' | 'nonprofit' | 'government' | 'military' | 'education' | 'volunteer' | 'freelance' | 'other'
  status?: 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded'
  tags?: string[] (filter by org tags)
  offset?: number (default 0)
  limit?: number (default 20)
Returns: Organization[]
SDK: sdk.organizations.list(filter)
```

#### `forge_search_job_descriptions`
Search job descriptions by status, organization.
```
Parameters:
  status?: 'interested' | 'analyzing' | 'applied' | 'interviewing' | 'offered' | 'rejected' | 'withdrawn' | 'closed'
  organization_id?: string
  search?: string
  offset?: number (default 0)
  limit?: number (default 20)
Returns: JobDescriptionWithOrg[]
SDK: sdk.jobDescriptions.list(filter)
```

#### `forge_search_summaries`
Search summaries.
```
Parameters:
  is_template?: boolean
  search?: string
  offset?: number (default 0)
  limit?: number (default 20)
Returns: Summary[]
SDK: sdk.summaries.list(filter)
```

---

### Tier 3: Refinement & Assembly

Tools for fine-tuning resume content and structure.

#### `forge_update_bullet`
Edit bullet content.
```
Parameters:
  bullet_id: string (required)
  content?: string
  metrics?: string
  domain?: string
  notes?: string
Returns: Bullet
SDK: sdk.bullets.update(bullet_id, input)
```

#### `forge_update_perspective`
Edit perspective content.
```
Parameters:
  perspective_id: string (required)
  content?: string
  domain?: string
  framing?: 'accomplishment' | 'responsibility' | 'context'
  notes?: string
Returns: Perspective
SDK: sdk.perspectives.update(perspective_id, input)
```

#### `forge_update_resume_entry`
Edit a resume entry's content (copy-on-write: override the perspective content for this specific resume).
```
Parameters:
  resume_id: string (required)
  entry_id: string (required)
  content?: string (set to override perspective content for this resume; set to null to reset to perspective reference)
  notes?: string
Returns: ResumeEntry
SDK: sdk.resumes.updateEntry(resume_id, entry_id, input)
Note: This enables per-resume bullet customization without modifying the shared
  perspective. The entry stores a local content override while preserving the
  perspective_content_snapshot for drift detection.
```

#### `forge_remove_resume_entry`
Remove an entry from a resume section.
```
Parameters:
  resume_id: string (required)
  entry_id: string (required)
Returns: void
SDK: sdk.resumes.removeEntry(resume_id, entry_id)
```

#### `forge_reorder_resume_entries`
Reorder entries within a resume.
```
Parameters:
  resume_id: string (required)
  entries: Array<{id: string, position: number}>
Returns: void
SDK: sdk.resumes.reorderEntries(resume_id, entries)
```

#### `forge_add_resume_skill`
Add a skill to a resume section.
```
Parameters:
  resume_id: string (required)
  section_id: string (required)
  skill_id: string (required)
Returns: ResumeSkill
SDK: sdk.resumes.addSkill(resume_id, section_id, skill_id)
```

#### `forge_remove_resume_skill`
Remove a skill from a resume section.
```
Parameters:
  resume_id: string (required)
  section_id: string (required)
  skill_id: string (required)
Returns: void
SDK: sdk.resumes.removeSkill(resume_id, section_id, skill_id)
```

#### `forge_reorder_resume_skills`
Reorder skills within a section.
```
Parameters:
  resume_id: string (required)
  section_id: string (required)
  skills: Array<{id: string, position: number}>
Returns: void
SDK: sdk.resumes.reorderSkills(resume_id, section_id, skills)
```

#### `forge_trace_chain`
Get full provenance chain for a perspective (perspective → bullet → source).
```
Parameters:
  perspective_id: string (required)
Returns: PerspectiveWithChain {perspective, bullet, source}
SDK: sdk.perspectives.get(perspective_id)
```

#### `forge_save_as_template`
Save a resume's section structure as a reusable template.
```
Parameters:
  resume_id: string (required)
  name: string (required)
  description?: string
Returns: ResumeTemplate
SDK: sdk.resumes.saveAsTemplate(resume_id, {name, description})
```

#### `forge_update_job_description`
Update JD status or content.
```
Parameters:
  job_description_id: string (required)
  status?: 'interested' | 'analyzing' | 'applied' | 'interviewing' | 'offered' | 'rejected' | 'withdrawn' | 'closed'
  raw_text?: string
  notes?: string
Returns: JobDescriptionWithOrg
SDK: sdk.jobDescriptions.update(id, input)
```

#### `forge_update_summary`
Edit summary content.
```
Parameters:
  summary_id: string (required)
  title?: string
  role?: string
  tagline?: string
  description?: string
  is_template?: boolean
Returns: Summary
SDK: sdk.summaries.update(summary_id, input)
```

#### `forge_update_source`
Update a source's content.
```
Parameters:
  source_id: string (required)
  title?: string
  description?: string
  notes?: string
  start_date?: string
  end_date?: string
Returns: Source
SDK: sdk.sources.update(id, input)
```

#### `forge_reopen_bullet`
Reopen a rejected bullet for re-review.
```
Parameters:
  bullet_id: string (required)
Returns: Bullet
SDK: sdk.bullets.reopen(bullet_id)
```

#### `forge_reopen_perspective`
Reopen a rejected perspective for re-review.
```
Parameters:
  perspective_id: string (required)
Returns: Perspective
SDK: sdk.perspectives.reopen(perspective_id)
```

#### `forge_clone_summary`
Duplicate a summary for variation.
```
Parameters:
  summary_id: string (required)
Returns: Summary (new copy)
SDK: sdk.summaries.clone(summary_id)
```

#### `forge_create_note`
Attach a note to any entity.
```
Parameters:
  title?: string
  content: string (required)
  references?: Array<{entity_type: 'source' | 'bullet' | 'perspective' | 'resume_entry' | 'resume' | 'skill' | 'organization', entity_id: string}>
Returns: UserNote (with references attached)
SDK: sdk.notes.create({title, content, references})
Note: References are created atomically with the note in a single API call.
  If any reference entity_id is invalid, the entire creation fails (no orphaned notes).
  This requires the HTTP endpoint to accept references in the create body.
```

#### `forge_search_notes`
Search notes by content.
```
Parameters:
  search?: string (full-text on title + content)
  offset?: number (default 0)
  limit?: number (default 20)
Returns: UserNote[]
SDK: sdk.notes.list({search, offset, limit})
```

---

## New HTTP Routes Required

The following HTTP routes must be added to `@forge/core` to support MCP tools that don't have existing endpoints:

### Alignment Routes (NEW)

```
GET  /api/alignment/score?jd_id=X&resume_id=Y&strong_threshold=0.75&adjacent_threshold=0.50
  → EmbeddingService.alignResume(jdId, resumeId, {strong_threshold, adjacent_threshold})
  → Returns AlignmentReport

GET  /api/alignment/match?jd_id=X&entity_type=perspective&threshold=0.50&limit=10
  → EmbeddingService.matchRequirements(jdId, entityType, {threshold, limit})
  → Returns RequirementMatchReport
```

### SDK Alignment Resource (NEW)

```typescript
// packages/sdk/src/resources/alignment.ts
class AlignmentResource {
  score(jdId: string, resumeId: string, opts?: {strong_threshold?: number, adjacent_threshold?: number}):
    Promise<Result<AlignmentReport>>

  matchRequirements(jdId: string, entityType: 'bullet' | 'perspective', opts?: {threshold?: number, limit?: number}):
    Promise<Result<RequirementMatchReport>>
}
```

### Health Route (NEW)

```
GET  /api/health
  → Returns { server: 'ok', version: string }
```

---

## Embedding Service Design (NEW — `@forge/core`)

### Overview

A new `EmbeddingService` in `@forge/core` that computes and stores vector embeddings for semantic similarity search and JD↔Resume alignment scoring.

### Model

`all-MiniLM-L6-v2` via `@xenova/transformers` (transformers.js)
- 384-dimensional vectors
- ~80MB model (downloaded once, cached locally)
- Runs entirely local in Bun/Node — no API calls
- ~50ms per embedding

### Schema Addition (migration 020+)

> **Note:** Migrations 017-019 are reserved by E-series specs (JD skill extraction, JD-resume
> linkage). The embeddings table must use migration 020 or later. Audit the full migration
> sequence before assigning the final number.

```sql
CREATE TABLE embeddings (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,  -- 'bullet', 'perspective', 'jd_requirement', 'source'
  entity_id TEXT NOT NULL,
  content_hash TEXT NOT NULL,  -- SHA256 of embedded text (for staleness detection)
  vector BLOB NOT NULL,        -- Float32Array serialized as bytes (384 * 4 = 1536 bytes)
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(entity_type, entity_id)
);

CREATE INDEX idx_embeddings_type ON embeddings(entity_type);
```

### Service API

```typescript
class EmbeddingService {
  // Embed and store a single entity
  embed(entityType: string, entityId: string, text: string): Promise<Result<void>>

  // Embed on write — fire-and-forget, does not block the calling service
  onBulletCreated(bullet: Bullet): Promise<void>
  onPerspectiveCreated(perspective: Perspective): Promise<void>
  onJDCreated(jd: JobDescription, requirements: string[]): Promise<void>

  // Similarity search
  findSimilar(queryText: string, entityType: string, threshold?: number, limit?: number):
    Promise<Result<Array<{entity_id: string, similarity: number}>>>

  // JD ↔ Resume alignment (the main API)
  alignResume(jdId: string, resumeId: string, opts?: {strong_threshold?: number, adjacent_threshold?: number}):
    Promise<Result<AlignmentReport>>

  // JD ↔ Bullet/Perspective matching (pre-resume discovery)
  matchRequirements(jdId: string, entityType: 'bullet' | 'perspective', opts?: {threshold?: number, limit?: number}):
    Promise<Result<RequirementMatchReport>>

  // Staleness check (content changed since embedding)
  checkStale(): Promise<Result<StaleEmbedding[]>>

  // Re-embed stale entries
  refreshStale(): Promise<Result<number>>  // returns count refreshed
}
```

### Embedding Lifecycle

Embedding computation is **fire-and-forget** — it does not block the calling service's transaction:

```
BulletService.createBullet()
  → bullet saved to DB (transaction committed)
  → embeddingService.onBulletCreated(bullet)  // async, non-blocking
  → if embedding fails, bullet still exists; checkStale() will detect it later
```

This means:
- Bullet/perspective creation is never slowed by embedding computation
- If the embedding model isn't loaded or OOM occurs, the entity is still usable
- `checkStale()` and `refreshStale()` provide the recovery path
- `forge_align_resume` returns a clear error if required embeddings are missing

### AlignmentReport Type

```typescript
interface AlignmentReport {
  job_description_id: string
  resume_id: string
  overall_score: number  // 0.0 - 1.0 weighted average
  requirement_matches: RequirementMatch[]
  unmatched_entries: UnmatchedEntry[]  // resume entries with no requirement match (noise)
  summary: {
    strong: number    // similarity > strong_threshold (default 0.75)
    adjacent: number  // between adjacent_threshold and strong_threshold
    gaps: number      // below adjacent_threshold or no match
    total_requirements: number
    total_entries: number
  }
  computed_at: string  // ISO timestamp
}

interface RequirementMatch {
  requirement_text: string
  requirement_index: number
  best_match: {
    entry_id: string
    perspective_id: string
    perspective_content: string
    similarity: number
  } | null
  verdict: 'strong' | 'adjacent' | 'gap'
}

interface UnmatchedEntry {
  entry_id: string
  perspective_content: string
  best_requirement_similarity: number  // highest similarity to any requirement
}

interface RequirementMatchReport {
  job_description_id: string
  matches: Array<{
    requirement_text: string
    candidates: Array<{entity_id: string, content: string, similarity: number}>
  }>
}
```

### JD Requirement Parsing

For `onJDCreated`, the service needs to split raw JD text into individual requirements. For v1:

```typescript
function parseRequirements(rawText: string): ParsedRequirements {
  // Split on bullet points, numbered lists, or line breaks in requirements/qualifications sections
  // Return individual requirement strings with confidence
}

interface ParsedRequirements {
  requirements: Array<{text: string, confidence: number}>
  overall_confidence: number  // average confidence
  // Low confidence (< 0.7) = prose paragraphs, ambiguous structure
  // High confidence (> 0.7) = structured lists, clear bullet points
}
```

This is the component that gets the confidence-gated LLM fallback later:
- High confidence (structured list, clear bullet points) → parse programmatically
- Low confidence (prose paragraphs, ambiguous) → flag for LLM or manual review

---

## JD Ingestion Pipeline Design

### v1: Structured Text Input

```
User provides raw JD text
  → forge_ingest_job_description(title, raw_text, org_id?, url?)
  → JD stored in job_descriptions table
  → Programmatic requirement parser extracts requirement strings (with confidence)
  → Each requirement embedded and stored in embeddings table (fire-and-forget)
  → Returns JD + embedding_status (parsed count, embedded count, confidence, ready flag)
  → If ready_for_alignment = true, forge_align_resume and forge_match_requirements are available
```

### v2 (future): Crawl + Extract

```
User provides URL
  → Template-based scraper (Greenhouse, Lever, LinkedIn, Workday templates)
  → Extracts: title, description, requirements, qualifications, salary, location
  → Confidence score on extraction quality
  → If confidence < threshold → flag for LLM extraction or manual review
  → Same downstream pipeline as v1
```

### Scraper Templates (future)

```typescript
interface ScraperTemplate {
  name: string           // 'greenhouse', 'lever', 'linkedin', etc.
  urlPattern: RegExp     // Match URL to template
  selectors: {
    title: string        // CSS/XPath selector
    description: string
    requirements: string
    qualifications: string
    salary?: string
    location?: string
  }
  parser: (html: string) => ParsedJD
}
```

---

## Tool Count Summary

| Tier | Count | Purpose |
|---|---|---|
| Tier 0: Diagnostics | 1 | Health check |
| Tier 1: Core Workflow | 20 | Search, get, list, derive, approve, assemble, align, match, export |
| Tier 2: Data Management | 18 | Create entities, JD skills, JD-resume linkage, profile, review, drift |
| Tier 3: Refinement | 18 | Update, reorder, trace, clone, notes, search notes |
| **Total** | **57** | |
| Resources | **7** | Ambient context (STDIO: poll after mutations; SSE: subscribe) |

---

## Implementation Notes

### STDIO Transport

```typescript
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { ForgeClient } from '@forge/sdk'

const sdk = new ForgeClient({ baseUrl: 'http://localhost:3000' })
const server = new McpServer({ name: 'forge', version: '1.0.0' })

// Register tools with proper error handling
server.tool('forge_search_sources', schema, async (params) => {
  const result = await sdk.sources.list(params)
  if (!result.ok) {
    return { content: [{ type: 'text', text: result.error.message }], isError: true }
  }
  return { content: [{ type: 'text', text: JSON.stringify(result.data) }] }
})

// Register resources
server.resource('forge://profile', async () => {
  const result = await sdk.profile.get()
  if (!result.ok) {
    return { contents: [{ uri: 'forge://profile', text: `Error: ${result.error.message}` }] }
  }
  return { contents: [{ uri: 'forge://profile', text: JSON.stringify(result.data) }] }
})

const transport = new StdioServerTransport()
await server.connect(transport)
```

### Prerequisite

The Forge HTTP server (`@forge/core`) must be running for the MCP server to function, since the MCP server delegates through the SDK which calls the HTTP API. Use `forge_health` at session start to verify connectivity.

### Error Handling

All SDK methods return `Result<T>`. The MCP server maps errors to MCP-level errors using `isError: true`:

| SDK Error Code | MCP Behavior |
|---|---|
| `NOT_FOUND` | `isError: true` with "Entity not found: {type} {id}" |
| `VALIDATION_ERROR` | `isError: true` with field-level details |
| `CONFLICT` | `isError: true` with constraint explanation (e.g., "Source is locked for derivation") |
| `AI_ERROR` | `isError: true` with "AI derivation failed — retry or check server logs" |
| `GATEWAY_TIMEOUT` | `isError: true` with "AI call timed out — retry" |
| `NETWORK_ERROR` | `isError: true` with "Cannot reach Forge server — is it running?" |

### Dependencies

This spec requires the following to exist before MCP implementation begins:

1. **SDK alignment resource** — `AlignmentResource` class with `score()` and `matchRequirements()` methods
2. **HTTP alignment routes** — `GET /api/alignment/score` and `GET /api/alignment/match`
3. **HTTP health route** — `GET /api/health`
4. **Embedding service** — `EmbeddingService` in `@forge/core` with migration 020+
5. **SDK entry methods** — Confirm `addEntry`/`updateEntry`/`removeEntry`/`reorderEntries` exist (Phase 12 rename from `addPerspective`/etc.)
6. **Atomic note creation** — HTTP endpoint accepts `references[]` in the create body
7. **JD skill extraction endpoint** — `POST /api/job-descriptions/:id/extract-skills` (E-series spec)
8. **JD-resume linkage endpoint** — `POST /api/job-descriptions/:id/resumes` (E-series spec)
