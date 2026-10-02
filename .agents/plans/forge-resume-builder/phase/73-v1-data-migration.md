# Phase 73: V1 Data Migration

**Status:** Planning
**Date:** 2026-04-04
**Depends on:** All core phases complete (27-68)
**Source:** `/Users/adam/notes/zettelkasten/proj/job-hunting/`

## Goal

Migrate all v1 job hunting data (55 jobs, 49 resumes, 6 archetypes, 135+ skills, 36 gap analyses, 7 cover letters) into Forge's data model. Resumes are imported with `markdown_override` for exact preservation AND decomposed into the Source → Bullet → Perspective → Resume Entry chain to support the primary data model.

## Non-Goals

- Cover letter entity type (stored as Notes for now, named `CoverLetter(<Org>:<Role>)`)
- Re-deriving bullets via AI (existing hand-crafted bullets are preserved as-is)
- Modifying any v1 files (read-only migration)

## Phases

### P73.1: Target Organizations
Import ~25-30 employer organizations from `job.employer` fields in v1 job files. These are target companies (Anthropic, BetterUp, Marriott, etc.) — distinct from existing orgs which are past employers.

**Source:** `jobs/*.md` → extract unique `job.employer` values
**Target:** `forge_create_organization` for each unique employer not already in Forge
**Fields:** name, org_type='company', status='backlog' (or map from job.status)

### P73.2: Job Descriptions
Import 55 jobs from `jobs/*.md` into Forge JD entities.

**Source:** Each `jobs/{UUID}.md` file
**Mapping:**
- `job.title` → title
- `job.employer` → organization_id (FK lookup from P73.1)
- `job.location` → location
- `job.compensation.base.low` → salary_min
- `job.compensation.base.high` → salary_max
- `job.link` → url
- `job.status` → status (map: new→discovered, interested→discovered, applied→applied, rejected→closed, pending→analyzing)
- body markdown → raw_text
- `tags[]` → JD skills (lookup/create skills, then tag)

### P73.3: Skills Reconciliation
Compare v1 `skills-inventory.md` (135+ skills with tier/years/demand across 11 categories) against Forge's existing skills. Create missing skills with proper categories.

**Source:** `skills-inventory.md` markdown tables
**Target:** `forge_create_skill` for each missing skill
**Fields:** name, category (from v1 category group)

### P73.4: Resume Import (Dual Strategy)
For each of the 49 `applications/{employer}/{role}/resume.md` files:

**Step A — Preserve as override:**
- Create Forge resume with name, target_role, target_employer, archetype
- Set `markdown_override` to the full resume body content
- Link to JD via `forge_link_resume_to_jd` (match by employer+role)

**Step B — Decompose into derivation chain:**
- Parse resume markdown into sections (Experience, Education, Skills, etc.)
- For each bullet under an experience section:
  1. Match to existing Forge source (by org name + role title)
  2. Create a Bullet from the bullet text, linked to the matched source
  3. Create a Perspective from the same text (framing=accomplishment, archetype from resume frontmatter)
  4. Auto-approve the bullet and perspective (these are human-vetted)
  5. Create resume section + entry linking the perspective

**Status for chain entities:** All bullets and perspectives created this way get `status: approved` since they were hand-crafted and already used in submitted applications.

### P73.5: Archetypes → Resume Templates
Import 6 archetype files as Forge resume templates.

**Source:** `applications/0-archetypes/{name}.md`
**Target:** Resume template with name, description (from `covers` array), content (reframing directives)

### P73.6: Gap Analyses → Notes
Import 36 analysis files as Forge notes with entity references.

**Source:** `analysis/*.md`
**Target:** `forge_create_note` with:
- title: filename (e.g., "Gap Analysis: Anthropic Cybersec RL")
- content: full markdown body
- references: link to JD entity (match via Job ID UUID or org+role)

### P73.7: Cover Letters → Notes
Import 7 cover letters as Forge notes.

**Source:** `applications/{employer}/{role}/cover-letter.md`
**Target:** `forge_create_note` with:
- title: `CoverLetter(<Org>:<Role>)` (e.g., `CoverLetter(Anthropic:Forward Deployed Engineer)`)
- content: full markdown body (excluding frontmatter)
- references: link to JD + Resume entities

### P73.8: JD-Resume Linkage
Wire up relationships between resumes and JDs based on application directory structure.

**Source:** Directory structure `applications/{employer}/{role}/` implies resume targets that employer's JD
**Target:** `forge_link_resume_to_jd` for each resume→JD pair

## Execution Order

1. P73.1 (orgs) — no deps
2. P73.3 (skills) — no deps
3. P73.2 (JDs) — needs orgs from P73.1
4. P73.5 (templates) — no deps
5. P73.4 (resumes + chain) — needs JDs, skills, sources all present
6. P73.6 (gap analyses) — needs JDs for references
7. P73.7 (cover letters) — needs JDs + resumes for references
8. P73.8 (linkage) — needs JDs + resumes

Steps 1-4 can run in parallel. Steps 5-8 are sequential.

## Implementation Approach

Build a migration script at `packages/cli/src/commands/migrate-v1.ts` that:
1. Reads v1 files from the zettelkasten path
2. Parses YAML frontmatter + markdown body
3. Uses the Forge SDK to create entities via the API
4. Tracks migration state (which files have been imported) to support incremental re-runs
5. Logs a migration report showing what was created, matched, and skipped

Alternatively, use the MCP tools directly from Claude Code for a supervised migration with human review at each step.
