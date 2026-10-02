# Phase 72: MCP Server Completion -- Tier 2-3 Tools

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-mcp-server-design.md](../refs/specs/2026-04-03-mcp-server-design.md)
**Depends on:** Phase 71 (MCP server foundation, transport, error mapping)
**Soft deps:** Phase 60 (JD-Resume Linkage -- `linkResume`/`unlinkResume` SDK methods), Phase 62 (JD Skill Extraction -- `extractSkills`/`addSkill`/`removeSkill` SDK methods)
**Blocks:** None
**Parallelizable with:** None -- this phase owns the MCP tool registry exclusively
**Duration:** Medium

## Goal

Register all Tier 2 (18 data management tools) and Tier 3 (18 refinement tools) in the `@forge/mcp` server, bringing the total registered tools to 57 (including the 21 Tier 0+1 tools from Phase 71). Feature-flag tools that depend on Phase 60/62 SDK methods so the server starts cleanly regardless of which phases have landed. Add end-to-end workflow tests and a complete README.

## Non-Goals

- Implementing SSE/Streamable HTTP transport (future phase)
- Adding new SDK methods -- this phase only wraps existing SDK methods as MCP tools
- Modifying `@forge/core` or `@forge/sdk` -- all work is in `@forge/mcp`
- Implementing the embedding service or alignment routes (separate spec)
- Adding MCP resource subscriptions (deferred to SSE transport phase)
- Tool-level authorization or rate limiting
- WebUI or CLI changes

## Context

Phase 71 establishes the MCP server skeleton: `McpServer` instantiation, STDIO transport, SDK client initialization, error mapping helper (`sdkResultToMcpResponse`), and the 21 Tier 0+1 tools (health, search/get, derive, approve/reject, create/add/export/align/match/gap). This phase adds the remaining 36 tools across Tiers 2 and 3.

Six tools depend on SDK methods introduced by Phase 60 (JD-resume linkage) and Phase 62 (JD skill extraction). These phases may not have landed yet, so the MCP server must detect whether the SDK methods exist and skip registration for unavailable tools, logging which tools are feature-flagged off at startup.

### SDK Method Inventory (Tier 2-3)

| Tool | SDK Call | Resource File |
|------|----------|---------------|
| `forge_create_source` | `sdk.sources.create(input)` | `sources.ts` |
| `forge_create_organization` | `sdk.organizations.create(input)` | `organizations.ts` |
| `forge_create_summary` | `sdk.summaries.create(input)` | `summaries.ts` |
| `forge_create_skill` | `sdk.skills.create(input)` | `skills.ts` |
| `forge_update_profile` | `sdk.profile.update(input)` | `profile.ts` |
| `forge_update_resume_header` | `sdk.resumes.updateHeader(id, header)` | `resumes.ts` |
| `forge_set_resume_summary` | `sdk.resumes.update(id, {summary_id})` | `resumes.ts` |
| `forge_ingest_job_description` | `sdk.jobDescriptions.create(input)` | `job-descriptions.ts` |
| `forge_extract_jd_skills` | `sdk.jobDescriptions.extractSkills(id)` | `job-descriptions.ts` -- Phase 62 |
| `forge_tag_jd_skill` | `sdk.jobDescriptions.addSkill(jdId, skillId)` | `job-descriptions.ts` -- Phase 62 |
| `forge_untag_jd_skill` | `sdk.jobDescriptions.removeSkill(jdId, skillId)` | `job-descriptions.ts` -- Phase 62 |
| `forge_link_resume_to_jd` | `sdk.jobDescriptions.linkResume(jdId, resumeId)` | `job-descriptions.ts` -- Phase 60 |
| `forge_unlink_resume_from_jd` | `sdk.jobDescriptions.unlinkResume(jdId, resumeId)` | `job-descriptions.ts` -- Phase 60 |
| `forge_review_pending` | `sdk.review.pending()` | `review.ts` |
| `forge_check_drift` | `sdk.integrity.drift()` | `integrity.ts` |
| `forge_search_organizations` | `sdk.organizations.list(filter)` | `organizations.ts` |
| `forge_search_job_descriptions` | `sdk.jobDescriptions.list(filter)` | `job-descriptions.ts` |
| `forge_search_summaries` | `sdk.summaries.list(filter)` | `summaries.ts` |
| `forge_update_bullet` | `sdk.bullets.update(id, input)` | `bullets.ts` |
| `forge_update_perspective` | `sdk.perspectives.update(id, input)` | `perspectives.ts` |
| `forge_update_resume_entry` | `sdk.resumes.updateEntry(resumeId, entryId, input)` | `resumes.ts` |
| `forge_update_source` | `sdk.sources.update(id, input)` | `sources.ts` |
| `forge_update_summary` | `sdk.summaries.update(id, input)` | `summaries.ts` |
| `forge_remove_resume_entry` | `sdk.resumes.removeEntry(resumeId, entryId)` | `resumes.ts` |
| `forge_reorder_resume_entries` | Not yet in SDK -- see Fallback | `resumes.ts` |
| `forge_add_resume_skill` | `sdk.resumes.addSkill(resumeId, sectionId, skillId)` | `resumes.ts` |
| `forge_remove_resume_skill` | `sdk.resumes.removeSkill(resumeId, sectionId, skillId)` | `resumes.ts` |
| `forge_reorder_resume_skills` | `sdk.resumes.reorderSkills(resumeId, sectionId, skills)` | `resumes.ts` |
| `forge_reopen_bullet` | `sdk.bullets.reopen(id)` | `bullets.ts` |
| `forge_reopen_perspective` | `sdk.perspectives.reopen(id)` | `perspectives.ts` |
| `forge_clone_summary` | `sdk.summaries.clone(id)` | `summaries.ts` |
| `forge_trace_chain` | `sdk.perspectives.get(id)` | `perspectives.ts` |
| `forge_save_as_template` | `sdk.resumes.saveAsTemplate(resumeId, {name, description})` | `resumes.ts` |
| `forge_update_job_description` | `sdk.jobDescriptions.update(id, input)` | `job-descriptions.ts` |
| `forge_create_note` | `sdk.notes.create(input)` + `sdk.notes.addReference(...)` | `notes.ts` |
| `forge_search_notes` | `sdk.notes.list(filter)` | `notes.ts` |

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| Tier 2: Entity creation tools (5) | Yes |
| Tier 2: Resume metadata tools (3 -- includes forge_update_profile) | Yes |
| Tier 2: JD tools (4) | Yes |
| Tier 2: JD-Resume linkage tools (2) | Yes |
| Tier 2: Monitoring tools (2) | Yes |
| Tier 2: Search tools (3) | Yes |
| Tier 2 total: 19 tools (18 always + 1 more when Phase 60 lands) | — |
| Tier 3: Update tools (5) | Yes |
| Tier 3: Resume assembly tools (5) | Yes |
| Tier 3: Workflow tools (5) | Yes |
| Tier 3: Note tools (2) | Yes |
| Tier 3: JD update tool (1) | Yes |
| Tier 3 total: 18 tools | — |
| Feature flag infrastructure | Yes |
| End-to-end workflow test | Yes |
| Package README and documentation | Yes |

> **Tool count summary:** Phase 71 = 21 tools. Phase 72 Tier 2 = 18 tools (+ up to 5 feature-flagged). Phase 72 Tier 3 = 18 tools (including 1 feature-flagged). Grand total when all flags enabled = 57.

## Files to Create

| File | Description |
|------|-------------|
| `packages/mcp/src/tools/tier2-entity-creation.ts` | Registers 5 entity creation tools |
| `packages/mcp/src/tools/tier2-resume-metadata.ts` | Registers 3 resume metadata tools (includes forge_update_profile) |
| `packages/mcp/src/tools/tier2-jd.ts` | Registers 4 JD tools (2 feature-flagged) |
| `packages/mcp/src/tools/tier2-jd-linkage.ts` | Registers 2 JD-resume linkage tools (feature-flagged) |
| `packages/mcp/src/tools/tier2-monitoring.ts` | Registers 2 monitoring tools |
| `packages/mcp/src/tools/tier2-search.ts` | Registers 3 search tools |
| `packages/mcp/src/tools/tier3-update.ts` | Registers 5 update tools. <!-- Cross-ref: see also tier3-assembly.ts for resume-level mutations --> |
| `packages/mcp/src/tools/tier3-assembly.ts` | Registers 5 resume assembly tools. <!-- Cross-ref: see also tier3-update.ts for entry-level content mutations --> |
| `packages/mcp/src/tools/tier3-workflow.ts` | Registers 5 workflow tools |
| `packages/mcp/src/tools/tier3-notes.ts` | Registers 2 note tools |
| `packages/mcp/src/tools/tier3-jd-update.ts` | Registers 1 JD update tool |
| `packages/mcp/src/feature-flags.ts` | Feature detection utility for Phase 60/62 SDK methods |
| `packages/mcp/src/truncation.ts` | Response payload truncation utility |
| `packages/mcp/tests/e2e/full-resume-workflow.test.ts` | End-to-end workflow test |
| `packages/mcp/tests/unit/feature-flags.test.ts` | Feature flag detection tests |
| `packages/mcp/tests/unit/truncation.test.ts` | Truncation utility tests |
| `packages/mcp/tests/fixtures/mock-sdk.ts` | Shared mock SDK factory for unit tests |
| `packages/mcp/README.md` | Package documentation with tool reference, configuration guides |

## Files to Modify

| File | Change |
|------|--------|
| `packages/mcp/src/index.ts` | Import and call all tier2/tier3 registration modules, call feature flag detection at startup |
| `packages/mcp/package.json` | No changes expected (deps already in Phase 71) |

## Fallback Strategies

- **Phase 60 not implemented:** `sdk.jobDescriptions.linkResume` and `unlinkResume` do not exist. Feature flag detection skips `forge_link_resume_to_jd` and `forge_unlink_resume_from_jd`. The server logs: `[forge:mcp] Skipping forge_link_resume_to_jd — sdk.jobDescriptions.linkResume not available (Phase 60)`. All other tools register normally.
- **Phase 62 not implemented:** `sdk.jobDescriptions.extractSkills`, `addSkill`, `removeSkill` do not exist. Feature flag detection skips `forge_extract_jd_skills`, `forge_tag_jd_skill`, `forge_untag_jd_skill`. Same logging pattern.
- **`sdk.resumes.reorderEntries` not in SDK:** The spec calls for `forge_reorder_resume_entries` but `ResumesResource` does not currently expose `reorderEntries()`. This tool is feature-flagged off via `flags.reorderEntries`. A follow-up task should add the SDK method. See T72.8 and T72.12 for implementation details.
- **`sdk.review` / `sdk.integrity` not in SDK:** If `sdk.review` or `sdk.integrity` are undefined (e.g., not yet implemented), the monitoring tools (`forge_review_pending`, `forge_check_drift`) should be feature-flagged off. Add `reviewAvailable` and `integrityAvailable` to `FeatureFlags`. See T72.12.
- **`sdk.notes` not in SDK:** If `sdk.notes` is undefined (not yet implemented), the note tools (`forge_create_note`, `forge_search_notes`) should be feature-flagged off. Add `notesAvailable` to `FeatureFlags`. See T72.12.
- **SDK method signature changes:** Each tool handler wraps the SDK call in a try-catch. If the SDK throws a TypeError (wrong arguments), the catch block returns `isError: true` with "Internal error: SDK method signature mismatch — check @forge/sdk version".
- **Large response payloads:** The truncation utility caps MCP text content at 50KB. If `JSON.stringify(result.data)` exceeds this, the response is truncated with a `_truncated: true` flag and a message telling the AI to use more specific filters or paginate.
- **Atomic note creation not supported:** The `CreateNote` SDK type does not include `references[]`. The `forge_create_note` handler calls `sdk.notes.create({title, content})` then calls `sdk.notes.addReference()` for each reference in a loop. If any reference fails, the note still exists but partial references are logged as warnings in the response. **Known deviation from spec:** The spec implies atomic note+reference creation, but the SDK does not support it. This is the expected implementation pattern until the SDK adds batch reference support. Follow-up: file SDK issue to support `references[]` in `CreateNote`.

---

## Tasks

### T72.1: Register Tier 2 Entity Creation Tools (4 tools)

**File:** `packages/mcp/src/tools/tier2-entity-creation.ts`

[CRITICAL] `forge_create_source` is polymorphic -- the Zod schema must accept extension fields conditionally based on `source_type`. The SDK `CreateSource` type uses nested objects (`role`, `project`, `education`, `clearance`) rather than flat extension fields. The MCP tool must accept the flat spec parameters and restructure them into the nested SDK format before calling `sdk.sources.create()`.

[IMPORTANT] `forge_create_organization` must validate `org_type` against the enum. The SDK `CreateOrganization` type accepts `org_type` as an optional string, but the MCP schema should constrain it to the valid enum values.

> **Note:** `forge_update_profile` has been moved to `tier2-resume-metadata.ts` (T72.2) since it is logically a metadata/singleton operation, not entity creation.

```typescript
import { z } from 'zod'
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { mapResult } from '../response'  // Phase 71 helper (aliased from sdkResultToMcpResponse)

export function registerTier2EntityCreationTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
): void {
  // forge_create_source
  server.tool(
    'forge_create_source',
    'Create a new experience source (role, project, education, clearance, or general).',
    {
      title: z.string().describe('Source title'),
      description: z.string().describe('Source description'),
      source_type: z.enum(['role', 'project', 'education', 'clearance', 'general'])
        .describe('Type of experience source'),
      start_date: z.string().optional().describe('Start date (ISO 8601)'),
      end_date: z.string().optional().describe('End date (ISO 8601)'),
      notes: z.string().optional().describe('Free-text notes'),
      // Role extension fields
      organization_id: z.string().optional()
        .describe('Organization ID (for role/project/education)'),
      is_current: z.boolean().optional().describe('Currently active (for role)'),
      work_arrangement: z.string().optional()
        .describe('Work arrangement (for role)'),
      // Education extension fields
      education_type: z.enum(['degree', 'certificate', 'course', 'self_taught']).optional()
        .describe('Education type (for education)'),
      degree_level: z.string().optional().describe('Degree level (for education)'),
      field: z.string().optional().describe('Field of study (for education)'),
      // Clearance extension fields
      level: z.string().optional().describe('Clearance level (for clearance)'),
      polygraph: z.string().optional().describe('Polygraph type (for clearance)'),
    },
    async (params) => {
      // Restructure flat params into SDK nested format
      // TODO: Remove `as any` once SDK types are updated (see Phase 62 type alignment)
      const input: any = {
        title: params.title,
        description: params.description,
        source_type: params.source_type,
        start_date: params.start_date,
        end_date: params.end_date,
        notes: params.notes,
      }

      if (params.source_type === 'role') {
        input.role = {
          organization_id: params.organization_id,
          is_current: params.is_current,
          work_arrangement: params.work_arrangement,
        }
      } else if (params.source_type === 'project') {
        input.project = { organization_id: params.organization_id }
      } else if (params.source_type === 'education') {
        input.education = {
          organization_id: params.organization_id,
          education_type: params.education_type,
          degree_level: params.degree_level,
          field: params.field,
        }
      } else if (params.source_type === 'clearance') {
        input.clearance = {
          level: params.level,
          polygraph: params.polygraph,
        }
      }
      // Note: source_type 'general' silently ignores extension fields.
      // Extension fields like organization_id, is_current, etc. are accepted by Zod
      // but not forwarded to the SDK when source_type is 'general'. This is intentional
      // -- the SDK has no extension object for general sources.

      const result = await sdk.sources.create(input)
      return respond(result)
    },
  )

  // forge_create_organization
  server.tool(
    'forge_create_organization',
    'Create an organization (employer, school, etc).',
    {
      name: z.string().describe('Organization name'),
      org_type: z.enum([
        'company', 'nonprofit', 'government', 'military',
        'education', 'volunteer', 'freelance', 'other',
      ]).describe('Organization type'),
      tag: z.string().optional()
        .describe('Classification tag (e.g., "employer", "defense", "remote")'),
      industry: z.string().optional().describe('Industry sector'),
      website: z.string().optional().describe('Website URL'),
      location: z.string().optional().describe('Primary location'),
      status: z.enum([
        'backlog', 'researching', 'exciting',
        'interested', 'acceptable', 'excluded',
      ]).optional().describe('Pipeline status'),
      notes: z.string().optional().describe('Free-text notes'),
    },
    async (params) => {
      const result = await sdk.organizations.create(params)
      return respond(result)
    },
  )

  // forge_create_summary
  server.tool(
    'forge_create_summary',
    'Create a resume summary paragraph.',
    {
      title: z.string().describe('Summary title/label'),
      role: z.string().optional().describe('Target role for this summary'),
      tagline: z.string().optional().describe('One-line tagline'),
      description: z.string().optional().describe('Full summary paragraph'),
      is_template: z.boolean().optional()
        .describe('Mark as a reusable template'),
    },
    async (params) => {
      const result = await sdk.summaries.create(params)
      return respond(result)
    },
  )

  // forge_create_skill
  server.tool(
    'forge_create_skill',
    'Create a skill entry in the skills inventory.',
    {
      name: z.string().describe('Skill name (e.g., "Kubernetes", "Python")'),
      category: z.string().optional()
        .describe('Skill category (e.g., "Infrastructure", "Languages")'),
    },
    async (params) => {
      const result = await sdk.skills.create(params)
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- All 4 tools register without error when `registerTier2EntityCreationTools()` is called.
- `forge_create_source` with `source_type: 'role'` restructures `organization_id`, `is_current`, `work_arrangement` into `{ role: { ... } }` before calling SDK.
- `forge_create_source` with `source_type: 'education'` restructures `education_type`, `degree_level`, `field` into `{ education: { ... } }`.
- `forge_create_source` with `source_type: 'clearance'` restructures `level`, `polygraph` into `{ clearance: { ... } }`.
- `forge_create_source` with `source_type: 'general'` passes no extension objects (extension fields silently ignored).
- `forge_create_source` with `source_type: 'project'` restructures `organization_id` into `{ project: { ... } }`.
- `forge_create_organization` constrains `org_type` and `status` to valid enum values via Zod.
- `forge_create_organization` uses `tag` (singular string), not `tags` (array), matching the SDK filter type.
- All tools return MCP-formatted responses via the shared `respond` helper (aliased from `mapResult`).

**Failure criteria:**
- Extension fields are passed flat to `sdk.sources.create()` instead of nested -- SDK will ignore them.
- `org_type` accepts arbitrary strings -- schema validation should reject invalid types.
- Missing `.describe()` on Zod fields -- MCP clients use these as parameter documentation.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_create_source` restructures role extension fields | Unit | Mock SDK returning `{ ok: true, data: { id: '...', source_type: 'role' } }` |
| `forge_create_source` restructures education extension fields | Unit | Mock SDK |
| `forge_create_source` with `source_type: 'project'` restructures organization_id | Unit | Mock SDK |
| `forge_create_source` with `source_type: 'general'` omits extension objects | Unit | Mock SDK |
| `forge_create_organization` rejects invalid `org_type` | Unit | Zod validation error |

---

### T72.2: Register Tier 2 Resume Metadata Tools (3 tools)

**File:** `packages/mcp/src/tools/tier2-resume-metadata.ts`

[IMPORTANT] `forge_set_resume_summary` is a convenience wrapper that calls `sdk.resumes.update(resume_id, { summary_id })`. The spec defines it as a dedicated tool rather than exposing the full `update` interface to keep the AI's decision space narrow.

> **Note:** `forge_update_profile` is registered here (moved from tier2-entity-creation.ts) since it is a singleton metadata update, not entity creation.

```typescript
import { mapResult } from '../response'  // Phase 71 helper

export function registerTier2ResumeMetadataTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
): void {
  // forge_update_resume_header
  server.tool(
    'forge_update_resume_header',
    'Set resume header fields (headline, contact override). Overrides profile defaults for this resume.',
    {
      resume_id: z.string().describe('Resume ID'),
      header: z.object({
        headline: z.string().optional().describe('Resume headline / professional title'),
        name: z.string().optional().describe('Name override'),
        email: z.string().optional().describe('Email override'),
        phone: z.string().optional().describe('Phone override'),
        location: z.string().optional().describe('Location override'),
        linkedin: z.string().optional().describe('LinkedIn URL override'),
        github: z.string().optional().describe('GitHub URL override'),
        website: z.string().optional().describe('Website URL override'),
      }).describe('Header fields to set'),
    },
    async (params) => {
      const result = await sdk.resumes.updateHeader(params.resume_id, params.header)
      return respond(result)
    },
  )

  // forge_set_resume_summary
  server.tool(
    'forge_set_resume_summary',
    'Link a summary to a resume. The summary appears at the top of the resume.',
    {
      resume_id: z.string().describe('Resume ID'),
      summary_id: z.string().describe('Summary ID to link'),
    },
    async (params) => {
      const result = await sdk.resumes.update(params.resume_id, {
        summary_id: params.summary_id,
      })
      return respond(result)
    },
  )

  // forge_update_profile
  server.tool(
    'forge_update_profile',
    'Update the user profile (singleton). Only provided fields are modified.',
    {
      name: z.string().optional().describe('Full name'),
      email: z.string().optional().describe('Email address'),
      phone: z.string().optional().describe('Phone number'),
      location: z.string().optional().describe('Location / city'),
      linkedin: z.string().optional().describe('LinkedIn URL'),
      github: z.string().optional().describe('GitHub URL'),
      website: z.string().optional().describe('Personal website URL'),
      clearance: z.string().optional().describe('Security clearance'),
    },
    async (params) => {
      const result = await sdk.profile.update(params)
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_update_resume_header` passes the `header` object directly to `sdk.resumes.updateHeader()`.
- `forge_set_resume_summary` calls `sdk.resumes.update()` with only `{ summary_id }`.
- `forge_update_profile` passes partial update correctly to `sdk.profile.update()`.
- All 3 tools use the shared `respond` helper (aliased from `mapResult`).

**Failure criteria:**
- `forge_update_resume_header` flattens the header fields instead of passing the nested object.
- `forge_set_resume_summary` passes extra fields to `update()`.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_update_resume_header` passes nested header to SDK | Unit | Mock SDK |
| `forge_set_resume_summary` calls `update` with only `summary_id` | Unit | Mock SDK, verify call args |
| `forge_update_profile` passes partial update correctly | Unit | Mock SDK |

---

### T72.3: Register Tier 2 JD Tools (4 tools)

**File:** `packages/mcp/src/tools/tier2-jd.ts`

[CRITICAL] `forge_extract_jd_skills`, `forge_tag_jd_skill`, and `forge_untag_jd_skill` depend on Phase 62 SDK methods. These must be conditionally registered using the feature flag utility from T72.12. `forge_ingest_job_description` is always available.

[IMPORTANT] `forge_ingest_job_description` wraps `sdk.jobDescriptions.create()`. The spec mentions an `embedding_status` return field, but this requires the embedding service (separate spec). The handler explicitly includes `embedding_status` in the response: if present in the SDK result, it is passed through; if absent, it is set to `null` so the caller always sees a consistent shape.

```typescript
import { detectFeatures, type FeatureFlags } from '../feature-flags'
import { mapResult } from '../response'  // Phase 71 helper
import { truncateResponse } from '../truncation'

export function registerTier2JDTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
  flags: FeatureFlags,
): void {
  // forge_ingest_job_description -- always available
  server.tool(
    'forge_ingest_job_description',
    'Store a job description. Triggers requirement parsing and embedding if available.',
    {
      title: z.string().describe('Job title'),
      raw_text: z.string().describe('Full job description text'),
      organization_id: z.string().optional().describe('Organization ID'),
      url: z.string().optional().describe('Original posting URL'),
      status: z.enum([
        'interested', 'analyzing', 'applied', 'interviewing',
        'offered', 'rejected', 'withdrawn', 'closed',
      ]).optional().describe('Application pipeline status (default: interested)'),
      salary_range: z.string().optional().describe('Salary range'),
      location: z.string().optional().describe('Job location'),
      notes: z.string().optional().describe('Free-text notes'),
    },
    async (params) => {
      const result = await sdk.jobDescriptions.create(params)
      if (!result.ok) {
        return respond(result)
      }
      // Ensure embedding_status is always present in response (null if absent)
      const data = {
        ...result.data,
        embedding_status: (result.data as any).embedding_status ?? null,
      }
      return respond({ ok: true, data })
    },
  )

  // Feature-flagged: Phase 62
  if (flags.jdSkillExtraction) {
    server.tool(
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

    server.tool(
      'forge_tag_jd_skill',
      'Associate a skill with a job description (confirm an extracted skill).',
      {
        job_description_id: z.string().describe('Job description ID'),
        skill_id: z.string().describe('Skill ID to associate'),
      },
      async (params) => {
        // TODO: Remove `as any` once Phase 62 SDK types are available (see Phase 62)
        const result = await (sdk.jobDescriptions as any).addSkill(
          params.job_description_id,
          params.skill_id,
        )
        return respond(result)
      },
    )

    server.tool(
      'forge_untag_jd_skill',
      'Remove a skill association from a job description.',
      {
        job_description_id: z.string().describe('Job description ID'),
        skill_id: z.string().describe('Skill ID to remove'),
      },
      async (params) => {
        // TODO: Remove `as any` once Phase 62 SDK types are available (see Phase 62)
        const result = await (sdk.jobDescriptions as any).removeSkill(
          params.job_description_id,
          params.skill_id,
        )
        return respond(result)
      },
    )
  }
}
```

**Acceptance criteria:**
- `forge_ingest_job_description` always registers regardless of feature flags.
- `forge_ingest_job_description` response always includes `embedding_status` (value or `null`).
- When `flags.jdSkillExtraction` is `false`, the 3 Phase 62 tools are not registered.
- When `flags.jdSkillExtraction` is `true`, all 4 tools register and delegate to the correct SDK methods.
- `forge_ingest_job_description` constrains `status` to the valid JD status enum.
- All `as any` casts have TODO comments referencing Phase 62.

**Failure criteria:**
- Phase 62 tools registered without checking feature flags -- server crashes if SDK methods are missing.
- `forge_ingest_job_description` accepts `status` values outside the enum.
- Feature-flagged tools use `sdk.jobDescriptions.extractSkills` without `as any` cast -- TypeScript compile error if Phase 62 types aren't in the SDK yet.
- `embedding_status` absent from response when embedding service is not running.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_ingest_job_description` passes params to `sdk.jobDescriptions.create()` | Unit | Mock SDK |
| `forge_ingest_job_description` returns `embedding_status: null` when absent from SDK result | Unit | Mock SDK returning result without `embedding_status` |
| `forge_ingest_job_description` passes through `embedding_status` when present | Unit | Mock SDK returning result with `embedding_status: 'pending'` |
| Phase 62 tools register when flags.jdSkillExtraction is true | Unit | Mock SDK with extractSkills/addSkill/removeSkill |
| Phase 62 tools skipped when flags.jdSkillExtraction is false | Unit | Mock SDK without Phase 62 methods |

---

### T72.4: Register Tier 2 JD-Resume Linkage Tools (2 tools)

**File:** `packages/mcp/src/tools/tier2-jd-linkage.ts`

[CRITICAL] Both tools depend on Phase 60 SDK methods (`linkResume`, `unlinkResume`). Entire file is conditionally registered via feature flags.

```typescript
import { mapResult } from '../response'  // Phase 71 helper

export function registerTier2JDLinkageTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
  flags: FeatureFlags,
): void {
  if (!flags.jdResumeLinkage) return

  server.tool(
    'forge_link_resume_to_jd',
    'Link a resume to a job description. Tracks which resume targets which job.',
    {
      job_description_id: z.string().describe('Job description ID'),
      resume_id: z.string().describe('Resume ID to link'),
    },
    async (params) => {
      // TODO: Remove `as any` once Phase 60 SDK types are available (see Phase 60)
      const result = await (sdk.jobDescriptions as any).linkResume(
        params.job_description_id,
        params.resume_id,
      )
      return respond(result)
    },
  )

  server.tool(
    'forge_unlink_resume_from_jd',
    'Remove the link between a resume and a job description.',
    {
      job_description_id: z.string().describe('Job description ID'),
      resume_id: z.string().describe('Resume ID to unlink'),
    },
    async (params) => {
      // TODO: Remove `as any` once Phase 60 SDK types are available (see Phase 60)
      const result = await (sdk.jobDescriptions as any).unlinkResume(
        params.job_description_id,
        params.resume_id,
      )
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- When `flags.jdResumeLinkage` is `false`, the function returns immediately, registering zero tools.
- When `flags.jdResumeLinkage` is `true`, both tools register with correct parameter schemas.
- `forge_link_resume_to_jd` calls `sdk.jobDescriptions.linkResume(jdId, resumeId)`.
- `forge_unlink_resume_from_jd` calls `sdk.jobDescriptions.unlinkResume(jdId, resumeId)`.
- All `as any` casts have TODO comments referencing Phase 60.

**Failure criteria:**
- Tools registered unconditionally -- runtime crash when SDK methods are missing.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| Both tools register when flags.jdResumeLinkage is true | Unit | Mock SDK with linkResume/unlinkResume |
| Function returns early when flags.jdResumeLinkage is false | Unit | Count registered tools = 0 |

---

### T72.5: Register Tier 2 Monitoring Tools (2 tools)

**File:** `packages/mcp/src/tools/tier2-monitoring.ts`

[IMPORTANT] Both tools take no parameters. `forge_review_pending` returns a `ReviewQueue` object. `forge_check_drift` returns a `DriftReport`. Both can produce large payloads -- apply the truncation utility.

[IMPORTANT] Both tools should be feature-flagged via `flags.reviewAvailable` and `flags.integrityAvailable` respectively. If the SDK resource is undefined, the tool is skipped.

```typescript
import { mapResult } from '../response'  // Phase 71 helper

export function registerTier2MonitoringTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
  flags: FeatureFlags,
): void {
  if (flags.reviewAvailable) {
    server.tool(
      'forge_review_pending',
      'Get all bullets and perspectives awaiting human review.',
      {},
      async () => {
        const result = await sdk.review.pending()
        return respond(result)
      },
    )
  }

  if (flags.integrityAvailable) {
    server.tool(
      'forge_check_drift',
      'Detect stale content snapshots in derivation chains and stale embeddings.',
      {},
      async () => {
        const result = await sdk.integrity.drift()
        return respond(result)
      },
    )
  }
}
```

**Acceptance criteria:**
- Both tools register with empty parameter schemas (`{}`).
- `forge_review_pending` calls `sdk.review.pending()` and returns the `ReviewQueue`.
- `forge_check_drift` calls `sdk.integrity.drift()` and returns the `DriftReport`.
- Large payloads are truncated by the `respond` helper (which calls truncation internally).
- Tools are feature-flagged off when their respective SDK resources are unavailable.

**Failure criteria:**
- Tools define unnecessary parameters.
- Response payloads exceed 50KB without truncation.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_review_pending` returns review queue | Unit | Mock SDK returning `{ bullets: [...], perspectives: [...] }` |
| `forge_check_drift` returns drift report | Unit | Mock SDK returning drift entries |
| `forge_review_pending` skipped when flags.reviewAvailable is false | Unit | Mock SDK without review |
| `forge_check_drift` skipped when flags.integrityAvailable is false | Unit | Mock SDK without integrity |

---

### T72.6: Register Tier 2 Search Tools (3 tools)

**File:** `packages/mcp/src/tools/tier2-search.ts`

[IMPORTANT] `forge_search_organizations` includes a `tag` filter (singular string). The SDK `OrganizationFilter` uses `tag` (singular string). The MCP tool schema matches the SDK directly.

```typescript
import { mapResult } from '../response'  // Phase 71 helper

export function registerTier2SearchTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
): void {
  server.tool(
    'forge_search_organizations',
    'Search organizations by name, type, status, or tag.',
    {
      search: z.string().optional().describe('Full-text search on name'),
      org_type: z.enum([
        'company', 'nonprofit', 'government', 'military',
        'education', 'volunteer', 'freelance', 'other',
      ]).optional().describe('Filter by organization type'),
      status: z.enum([
        'backlog', 'researching', 'exciting',
        'interested', 'acceptable', 'excluded',
      ]).optional().describe('Filter by pipeline status'),
      tag: z.string().optional()
        .describe('Filter by tag (e.g., "employer", "defense", "remote")'),
      offset: z.number().optional().describe('Pagination offset (default 0)'),
      limit: z.number().optional().describe('Page size (default 20)'),
    },
    async (params) => {
      const result = await sdk.organizations.list(params)
      return respond(result)
    },
  )

  server.tool(
    'forge_search_job_descriptions',
    'Search job descriptions by status, organization, or text.',
    {
      status: z.enum([
        'interested', 'analyzing', 'applied', 'interviewing',
        'offered', 'rejected', 'withdrawn', 'closed',
      ]).optional().describe('Filter by pipeline status'),
      organization_id: z.string().optional()
        .describe('Filter by organization ID'),
      search: z.string().optional().describe('Full-text search'),
      offset: z.number().optional().describe('Pagination offset (default 0)'),
      limit: z.number().optional().describe('Page size (default 20)'),
    },
    async (params) => {
      const result = await sdk.jobDescriptions.list(params)
      return respond(result)
    },
  )

  server.tool(
    'forge_search_summaries',
    'Search summaries by template flag or text content.',
    {
      is_template: z.boolean().optional()
        .describe('Filter to templates only'),
      search: z.string().optional().describe('Full-text search'),
      offset: z.number().optional().describe('Pagination offset (default 0)'),
      limit: z.number().optional().describe('Page size (default 20)'),
    },
    async (params) => {
      const result = await sdk.summaries.list(params)
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_search_organizations` constrains `org_type` and `status` to valid enums.
- `forge_search_organizations` uses `tag` (singular string) matching the SDK filter type.
- All 3 tools include `offset` and `limit` pagination parameters.
- `forge_search_job_descriptions` constrains `status` to the JD status enum.

**Failure criteria:**
- `tags` (plural array) used instead of `tag` (singular string) -- SDK expects `tag`.
- Status enums accept arbitrary strings.
- Missing pagination parameters.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_search_organizations` passes tag as string to SDK filter | Unit | Mock SDK, verify `tag` in filter |
| `forge_search_job_descriptions` passes status filter | Unit | Mock SDK |
| `forge_search_summaries` passes is_template as boolean | Unit | Mock SDK |

---

### T72.7: Register Tier 3 Update Tools (5 tools)

**File:** `packages/mcp/src/tools/tier3-update.ts`

<!-- Cross-ref: see also tier3-assembly.ts for resume-level structural mutations (add/remove/reorder entries and skills) -->

[CRITICAL] `forge_update_resume_entry` uses copy-on-write semantics. Setting `content` overrides the perspective content for this specific resume. Setting `content` to `null` resets to the original perspective content. The Zod schema must allow `null` for `content` to support the reset behavior.

[IMPORTANT] **Nullable SDK type verification:** The SDK `UpdateResumeEntry` type must accept `content: null`. Verify at implementation time that `sdk.resumes.updateEntry()` correctly handles `null` content. If the SDK type does not include `null` in the union, a `as any` cast is required with a TODO referencing the SDK type issue.

```typescript
import { mapResult } from '../response'  // Phase 71 helper

export function registerTier3UpdateTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
): void {
  server.tool(
    'forge_update_bullet',
    'Edit bullet content, metrics, domain, or notes.',
    {
      bullet_id: z.string().describe('Bullet ID'),
      content: z.string().optional().describe('Updated bullet text'),
      metrics: z.string().nullable().optional()
        .describe('Quantifiable metrics (set null to clear)'),
      domain: z.string().nullable().optional()
        .describe('Domain slug (set null to clear)'),
      notes: z.string().nullable().optional()
        .describe('Notes (set null to clear)'),
    },
    async (params) => {
      const { bullet_id, ...input } = params
      const result = await sdk.bullets.update(bullet_id, input)
      return respond(result)
    },
  )

  server.tool(
    'forge_update_perspective',
    'Edit perspective content, domain, framing, or notes.',
    {
      perspective_id: z.string().describe('Perspective ID'),
      content: z.string().optional().describe('Updated perspective text'),
      domain: z.string().nullable().optional()
        .describe('Domain slug (set null to clear)'),
      framing: z.enum(['accomplishment', 'responsibility', 'context']).optional()
        .describe('Perspective framing type'),
      notes: z.string().nullable().optional()
        .describe('Notes (set null to clear)'),
    },
    async (params) => {
      const { perspective_id, ...input } = params
      const result = await sdk.perspectives.update(perspective_id, input)
      return respond(result)
    },
  )

  server.tool(
    'forge_update_resume_entry',
    'Edit a resume entry (copy-on-write). Set content to override perspective text for this resume; set content to null to reset to original.',
    {
      resume_id: z.string().describe('Resume ID'),
      entry_id: z.string().describe('Entry ID'),
      content: z.string().nullable().optional()
        .describe('Content override (null resets to perspective reference)'),
      notes: z.string().nullable().optional()
        .describe('Entry-level notes (set null to clear)'),
    },
    async (params) => {
      const { resume_id, entry_id, ...input } = params
      // NOTE: If SDK UpdateResumeEntry type does not accept `content: null`,
      // cast may be needed. Verify SDK type at implementation time.
      // TODO: Remove `as any` if needed once SDK types confirm nullable content (see Phase 62)
      const result = await sdk.resumes.updateEntry(resume_id, entry_id, input)
      return respond(result)
    },
  )

  server.tool(
    'forge_update_source',
    'Update a source\'s content or metadata.',
    {
      source_id: z.string().describe('Source ID'),
      title: z.string().optional().describe('Updated title'),
      description: z.string().optional().describe('Updated description'),
      notes: z.string().nullable().optional()
        .describe('Notes (set null to clear)'),
      start_date: z.string().nullable().optional()
        .describe('Start date (set null to clear)'),
      end_date: z.string().nullable().optional()
        .describe('End date (set null to clear)'),
    },
    async (params) => {
      const { source_id, ...input } = params
      const result = await sdk.sources.update(source_id, input)
      return respond(result)
    },
  )

  server.tool(
    'forge_update_summary',
    'Edit summary content or metadata.',
    {
      summary_id: z.string().describe('Summary ID'),
      title: z.string().optional().describe('Updated title'),
      role: z.string().nullable().optional()
        .describe('Target role (set null to clear)'),
      tagline: z.string().nullable().optional()
        .describe('One-line tagline (set null to clear)'),
      description: z.string().nullable().optional()
        .describe('Full summary paragraph (set null to clear)'),
      is_template: z.boolean().optional()
        .describe('Mark as reusable template'),
    },
    async (params) => {
      const { summary_id, ...input } = params
      const result = await sdk.summaries.update(summary_id, input)
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- All 5 tools destructure the ID field from params before passing the rest to the SDK.
- `forge_update_resume_entry` accepts `content: null` (reset to perspective reference).
- `forge_update_bullet` and `forge_update_perspective` accept nullable fields for clearing.
- `forge_update_perspective` constrains `framing` to the valid enum.

**Failure criteria:**
- ID fields included in the update payload (SDK methods take ID as first arg, input as second).
- `content` on `forge_update_resume_entry` does not accept `null` -- cannot reset copy-on-write.
- Nullable fields defined as `.optional()` instead of `.nullable().optional()`.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_update_resume_entry` passes null content for reset | Unit | Mock SDK, verify `content: null` in input |
| `forge_update_bullet` destructures `bullet_id` from input | Unit | Mock SDK, verify update called with `(id, rest)` |
| `forge_update_perspective` constrains framing enum | Unit | Zod validation |
| `forge_update_source` passes nullable dates | Unit | Mock SDK |
| `forge_update_summary` passes is_template boolean | Unit | Mock SDK |

---

### T72.8: Register Tier 3 Resume Assembly Tools (5 tools)

**File:** `packages/mcp/src/tools/tier3-assembly.ts`

<!-- Cross-ref: see also tier3-update.ts for entry-level content mutations (update bullet/perspective/entry/source/summary) -->

[CRITICAL] `forge_reorder_resume_entries` requires `sdk.resumes.reorderEntries()` which does not exist in the current SDK. This tool must be feature-flagged via `flags.reorderEntries`. The `flags` object is passed into the registration function.

> **Note:** `forge_reorder_resume_entries` is explicitly feature-flagged off until the SDK exposes `reorderEntries()`. When `flags.reorderEntries` is `false`, the tool is not registered and a log message is emitted at startup (see T72.12). Follow-up: add `reorderEntries()` to `ResumesResource` in the SDK.

[IMPORTANT] `forge_add_resume_skill` and `forge_remove_resume_skill` require three IDs (resume, section, skill). The `forge_reorder_resume_skills` method takes an array of `{skill_id, position}` objects matching the SDK signature `Array<{ skill_id: string; position: number }>`.

```typescript
import { mapResult } from '../response'  // Phase 71 helper

export function registerTier3AssemblyTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
  flags: FeatureFlags,
): void {
  server.tool(
    'forge_remove_resume_entry',
    'Remove an entry from a resume section. The underlying perspective is not deleted.',
    {
      resume_id: z.string().describe('Resume ID'),
      entry_id: z.string().describe('Entry ID to remove'),
    },
    async (params) => {
      const result = await sdk.resumes.removeEntry(
        params.resume_id,
        params.entry_id,
      )
      return respond(result)
    },
  )

  // Feature-flagged: reorderEntries not in SDK yet
  if (flags.reorderEntries) {
    server.tool(
      'forge_reorder_resume_entries',
      'Reorder entries within a resume by specifying new positions.',
      {
        resume_id: z.string().describe('Resume ID'),
        entries: z.array(z.object({
          id: z.string().describe('Entry ID'),
          position: z.number().describe('New position (0-based)'),
        })).describe('Array of entry ID + position pairs'),
      },
      async (params) => {
        // TODO: Remove `as any` once SDK exposes reorderEntries (see Phase 62)
        const result = await (sdk.resumes as any).reorderEntries(
          params.resume_id,
          params.entries,
        )
        return respond(result)
      },
    )
  }

  server.tool(
    'forge_add_resume_skill',
    'Add a skill to a resume section (skills-type sections only).',
    {
      resume_id: z.string().describe('Resume ID'),
      section_id: z.string().describe('Section ID (must be skills-type)'),
      skill_id: z.string().describe('Skill ID to add'),
    },
    async (params) => {
      const result = await sdk.resumes.addSkill(
        params.resume_id,
        params.section_id,
        params.skill_id,
      )
      return respond(result)
    },
  )

  server.tool(
    'forge_remove_resume_skill',
    'Remove a skill from a resume section.',
    {
      resume_id: z.string().describe('Resume ID'),
      section_id: z.string().describe('Section ID'),
      skill_id: z.string().describe('Skill ID to remove'),
    },
    async (params) => {
      const result = await sdk.resumes.removeSkill(
        params.resume_id,
        params.section_id,
        params.skill_id,
      )
      return respond(result)
    },
  )

  server.tool(
    'forge_reorder_resume_skills',
    'Reorder skills within a resume section by specifying new positions.',
    {
      resume_id: z.string().describe('Resume ID'),
      section_id: z.string().describe('Section ID'),
      skills: z.array(z.object({
        skill_id: z.string().describe('Skill ID'),
        position: z.number().describe('New position (0-based)'),
      })).describe('Array of skill ID + position pairs'),
    },
    async (params) => {
      const result = await sdk.resumes.reorderSkills(
        params.resume_id,
        params.section_id,
        params.skills,
      )
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_remove_resume_entry` calls `sdk.resumes.removeEntry(resumeId, entryId)`.
- `forge_reorder_resume_entries` only registers if `flags.reorderEntries` is `true` (not inline `typeof` check).
- `forge_add_resume_skill` passes all 3 IDs in the correct order.
- `forge_reorder_resume_skills` passes `skills` array matching SDK signature `Array<{ skill_id, position }>`.
- All tools return MCP-formatted responses.
- Atomic note creation deviation is documented (see Fallback Strategies).

**Failure criteria:**
- `forge_reorder_resume_entries` registered unconditionally -- crashes at call time.
- `forge_add_resume_skill` or `forge_remove_resume_skill` pass IDs in wrong order.
- `forge_reorder_resume_skills` uses `id` instead of `skill_id` in the array objects.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_remove_resume_entry` calls removeEntry with correct args | Unit | Mock SDK |
| `forge_reorder_resume_entries` skipped when flags.reorderEntries is false | Unit | Mock SDK, verify tool not registered |
| `forge_reorder_resume_entries` registers when flags.reorderEntries is true | Unit | Mock SDK with reorderEntries |
| `forge_add_resume_skill` passes 3 IDs in order | Unit | Mock SDK, verify call args |
| `forge_reorder_resume_skills` passes skill_id + position | Unit | Mock SDK |

---

### T72.9: Register Tier 3 Workflow Tools (5 tools)

**File:** `packages/mcp/src/tools/tier3-workflow.ts`

[IMPORTANT] `forge_trace_chain` reuses `sdk.perspectives.get()` which returns `PerspectiveWithChain { perspective, bullet, source }`. This is the same SDK call as `forge_get_perspective` from Tier 1, but with a different tool name and description emphasizing the provenance chain use case.

```typescript
import { mapResult } from '../response'  // Phase 71 helper

export function registerTier3WorkflowTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
): void {
  server.tool(
    'forge_reopen_bullet',
    'Reopen a rejected bullet for re-review (rejected -> pending_review).',
    {
      bullet_id: z.string().describe('Bullet ID to reopen'),
    },
    async (params) => {
      const result = await sdk.bullets.reopen(params.bullet_id)
      return respond(result)
    },
  )

  server.tool(
    'forge_reopen_perspective',
    'Reopen a rejected perspective for re-review (rejected -> pending_review).',
    {
      perspective_id: z.string().describe('Perspective ID to reopen'),
    },
    async (params) => {
      const result = await sdk.perspectives.reopen(params.perspective_id)
      return respond(result)
    },
  )

  server.tool(
    'forge_clone_summary',
    'Duplicate a summary for creating variations. Returns the new copy.',
    {
      summary_id: z.string().describe('Summary ID to clone'),
    },
    async (params) => {
      const result = await sdk.summaries.clone(params.summary_id)
      return respond(result)
    },
  )

  server.tool(
    'forge_trace_chain',
    'Get full provenance chain for a perspective: perspective -> bullet -> source.',
    {
      perspective_id: z.string().describe('Perspective ID to trace'),
    },
    async (params) => {
      const result = await sdk.perspectives.get(params.perspective_id)
      return respond(result)
    },
  )

  server.tool(
    'forge_save_as_template',
    'Save a resume\'s section structure as a reusable template.',
    {
      resume_id: z.string().describe('Resume ID to save as template'),
      name: z.string().describe('Template name'),
      description: z.string().optional().describe('Template description'),
    },
    async (params) => {
      const result = await sdk.resumes.saveAsTemplate(params.resume_id, {
        name: params.name,
        description: params.description,
      })
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_reopen_bullet` calls `sdk.bullets.reopen(id)`.
- `forge_reopen_perspective` calls `sdk.perspectives.reopen(id)`.
- `forge_clone_summary` calls `sdk.summaries.clone(id)` and returns the new copy.
- `forge_trace_chain` calls `sdk.perspectives.get(id)` and returns `PerspectiveWithChain`.
- `forge_save_as_template` passes `{ name, description }` to `sdk.resumes.saveAsTemplate()`.

**Failure criteria:**
- `forge_trace_chain` does not return the full chain (bullet + source) -- SDK method must be `.get()`, not `.list()`.
- `forge_save_as_template` passes `resume_id` inside the input object instead of as the first argument.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_reopen_bullet` calls sdk.bullets.reopen | Unit | Mock SDK |
| `forge_clone_summary` returns new summary with different ID | Unit | Mock SDK returning cloned summary |
| `forge_trace_chain` returns PerspectiveWithChain | Unit | Mock SDK returning `{ perspective, bullet, source }` |
| `forge_save_as_template` passes name/description correctly | Unit | Mock SDK |

---

### T72.10: Register Tier 3 Note Tools (2 tools)

**File:** `packages/mcp/src/tools/tier3-notes.ts`

[CRITICAL] `forge_create_note` must handle the fact that the SDK `CreateNote` type does not include `references[]`. The handler creates the note first, then calls `sdk.notes.addReference()` for each reference. If any reference call fails, the note still exists but the response includes a `warnings` array listing which references failed.

> **Known deviation from spec:** The spec implies atomic note+reference creation, but the SDK does not support it. Current implementation: create note, then loop `addReference()`. This is the expected pattern until the SDK adds batch reference support. Follow-up: file SDK issue to support `references[]` in `CreateNote`.

[IMPORTANT] `forge_create_note` should be feature-flagged via `flags.notesAvailable`. If `sdk.notes` is undefined, both note tools are skipped.

```typescript
import { mapResult } from '../response'  // Phase 71 helper
import { truncateResponse } from '../truncation'

export function registerTier3NoteTools(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
  flags: FeatureFlags,
): void {
  if (!flags.notesAvailable) return

  server.tool(
    'forge_create_note',
    'Create a note with optional entity references. References are added after creation (not atomic -- see known deviations).',
    {
      title: z.string().optional().describe('Note title'),
      content: z.string().describe('Note content'),
      references: z.array(z.object({
        entity_type: z.enum([
          'source', 'bullet', 'perspective', 'resume_entry',
          'resume', 'skill', 'organization',
        ]).describe('Entity type'),
        entity_id: z.string().describe('Entity ID'),
      })).optional().describe('Entities to reference from this note'),
    },
    async (params) => {
      // Step 1: Create the note
      const noteResult = await sdk.notes.create({
        title: params.title,
        content: params.content,
      })

      if (!noteResult.ok) {
        return respond(noteResult)
      }

      // Step 2: Attach references (best-effort)
      const warnings: string[] = []
      if (params.references?.length) {
        for (const ref of params.references) {
          try {
            const refResult = await sdk.notes.addReference(noteResult.data.id, {
              entity_type: ref.entity_type,
              entity_id: ref.entity_id,
            })
            if (!refResult.ok) {
              warnings.push(
                `Failed to add reference ${ref.entity_type}:${ref.entity_id}: ${refResult.error.message}`,
              )
            }
          } catch (err) {
            warnings.push(
              `Failed to add reference ${ref.entity_type}:${ref.entity_id}: ${String(err)}`,
            )
          }
        }
      }

      // Step 3: Return note with warnings if any, applying truncation
      const data = warnings.length > 0
        ? { ...noteResult.data, _warnings: warnings }
        : noteResult.data

      const { text } = truncateResponse(data)
      return {
        content: [{
          type: 'text' as const,
          text,
        }],
      }
    },
  )

  server.tool(
    'forge_search_notes',
    'Search notes by content or title.',
    {
      search: z.string().optional().describe('Full-text search on title + content'),
      offset: z.number().optional().describe('Pagination offset (default 0)'),
      limit: z.number().optional().describe('Page size (default 20)'),
    },
    async (params) => {
      const result = await sdk.notes.list(params)
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_create_note` creates the note first, then attaches references in sequence.
- If all references succeed, the response contains the note data without warnings.
- If any reference fails, the response includes `_warnings` array but still returns the created note.
- `forge_create_note` constrains `entity_type` to the valid entity type enum.
- `forge_create_note` applies `truncateResponse()` to the data before returning.
- `forge_create_note` description mentions non-atomic behavior.
- `forge_search_notes` passes `search`, `offset`, `limit` to `sdk.notes.list()`.
- Both tools are feature-flagged via `flags.notesAvailable`.
- Atomic note creation is documented as a known deviation (see Fallback Strategies).

**Failure criteria:**
- References attempted before note creation -- note ID not available.
- Reference failure causes the entire tool to return `isError: true` -- note is lost from the response.
- `entity_type` accepts arbitrary strings.
- Response returned without truncation -- large note data could exceed payload limits.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_create_note` creates note then attaches references | Unit | Mock SDK, verify call order |
| `forge_create_note` returns note + warnings on partial ref failure | Unit | Mock SDK with one ref failing |
| `forge_create_note` works without references | Unit | Mock SDK |
| `forge_create_note` applies truncateResponse to output | Unit | Mock SDK, verify truncation called |
| `forge_search_notes` passes filter to sdk.notes.list | Unit | Mock SDK |
| Both note tools skipped when flags.notesAvailable is false | Unit | Mock SDK without notes |

---

### T72.11: Register Tier 3 JD Update Tool (1 tool)

**File:** `packages/mcp/src/tools/tier3-jd-update.ts`

```typescript
import { mapResult } from '../response'  // Phase 71 helper

export function registerTier3JDUpdateTool(
  server: McpServer,
  sdk: ForgeClient,
  respond: typeof mapResult,
): void {
  server.tool(
    'forge_update_job_description',
    'Update a job description\'s status, text, or notes.',
    {
      job_description_id: z.string().describe('Job description ID'),
      status: z.enum([
        'interested', 'analyzing', 'applied', 'interviewing',
        'offered', 'rejected', 'withdrawn', 'closed',
      ]).optional().describe('Pipeline status'),
      raw_text: z.string().optional().describe('Updated job description text'),
      notes: z.string().nullable().optional()
        .describe('Notes (set null to clear)'),
    },
    async (params) => {
      const { job_description_id, ...input } = params
      const result = await sdk.jobDescriptions.update(job_description_id, input)
      return respond(result)
    },
  )
}
```

**Acceptance criteria:**
- Tool destructures `job_description_id` from params before passing rest to SDK.
- `status` constrained to valid JD status enum.
- `notes` accepts `null` for clearing.

**Failure criteria:**
- `job_description_id` included in the update payload.
- Status enum accepts arbitrary strings.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| `forge_update_job_description` destructures ID from input | Unit | Mock SDK, verify call args |
| `forge_update_job_description` accepts null notes | Unit | Mock SDK |

---

### T72.12: Feature Flag Implementation

**File:** `packages/mcp/src/feature-flags.ts`

[CRITICAL] Feature detection must check for SDK method existence, not for TypeScript type availability. The check is performed at runtime by testing whether the method property is a function on the SDK resource instance.

```typescript
import type { ForgeClient } from '@forge/sdk'

export interface FeatureFlags {
  /** Phase 60: JD-resume linkage SDK methods */
  jdResumeLinkage: boolean
  /** Phase 62: JD skill extraction SDK methods */
  jdSkillExtraction: boolean
  /** SDK has reorderEntries method */
  reorderEntries: boolean
  /** sdk.review resource is available */
  reviewAvailable: boolean
  /** sdk.integrity resource is available */
  integrityAvailable: boolean
  /** sdk.notes resource is available */
  notesAvailable: boolean
}

/**
 * Detect which optional SDK methods are available.
 *
 * Checks for method existence on the SDK resource instances.
 * Methods added by Phase 60 and Phase 62 may not be present
 * if those phases have not been implemented yet.
 */
export function detectFeatures(sdk: ForgeClient): FeatureFlags {
  const flags: FeatureFlags = {
    jdResumeLinkage:
      typeof (sdk.jobDescriptions as any).linkResume === 'function' &&
      typeof (sdk.jobDescriptions as any).unlinkResume === 'function',

    jdSkillExtraction:
      typeof (sdk.jobDescriptions as any).extractSkills === 'function' &&
      typeof (sdk.jobDescriptions as any).addSkill === 'function' &&
      typeof (sdk.jobDescriptions as any).removeSkill === 'function',

    reorderEntries:
      typeof (sdk.resumes as any).reorderEntries === 'function',

    reviewAvailable:
      typeof (sdk as any).review !== 'undefined' &&
      typeof (sdk as any).review?.pending === 'function',

    integrityAvailable:
      typeof (sdk as any).integrity !== 'undefined' &&
      typeof (sdk as any).integrity?.drift === 'function',

    notesAvailable:
      typeof (sdk as any).notes !== 'undefined' &&
      typeof (sdk as any).notes?.create === 'function',
  }

  // Log feature flag state at startup
  const flagged: string[] = []
  if (!flags.jdResumeLinkage) {
    flagged.push('forge_link_resume_to_jd, forge_unlink_resume_from_jd (Phase 60)')
  }
  if (!flags.jdSkillExtraction) {
    flagged.push(
      'forge_extract_jd_skills, forge_tag_jd_skill, forge_untag_jd_skill (Phase 62)',
    )
  }
  if (!flags.reorderEntries) {
    flagged.push('forge_reorder_resume_entries (SDK missing reorderEntries — feature-flagged off)')
  }
  if (!flags.reviewAvailable) {
    flagged.push('forge_review_pending (sdk.review not available)')
  }
  if (!flags.integrityAvailable) {
    flagged.push('forge_check_drift (sdk.integrity not available)')
  }
  if (!flags.notesAvailable) {
    flagged.push('forge_create_note, forge_search_notes (sdk.notes not available)')
  }

  if (flagged.length > 0) {
    console.error(
      `[forge:mcp] Feature-flagged tools (not registered):\n` +
      flagged.map((t) => `  - ${t}`).join('\n'),
    )
  } else {
    console.error('[forge:mcp] All 57 tools registered (no feature flags active)')
  }

  return flags
}
```

**Acceptance criteria:**
- `detectFeatures()` returns a `FeatureFlags` object with boolean fields.
- Each flag checks all required methods for that feature (e.g., `jdSkillExtraction` requires all 3 methods).
- Startup logs list which tools are feature-flagged off.
- When all methods exist, logs confirm all 57 tools are registered.
- `reviewAvailable`, `integrityAvailable`, and `notesAvailable` flags detect missing SDK resources.
- `reorderEntries` flag uses `flags.reorderEntries` (not inline `typeof` check at registration site).

**Failure criteria:**
- Feature detection uses `'linkResume' in sdk.jobDescriptions` instead of `typeof ... === 'function'` -- would return true for non-function properties.
- Only one method checked per feature -- partial availability would register tools that crash.
- No logging -- silent tool omission is confusing to debug.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| All flags true when all methods exist | Unit | Mock SDK with all methods |
| `jdResumeLinkage` false when linkResume missing | Unit | Mock SDK without linkResume |
| `jdSkillExtraction` false when extractSkills missing | Unit | Mock SDK without extractSkills |
| `reorderEntries` false when method missing | Unit | Mock SDK without reorderEntries |
| `reviewAvailable` false when sdk.review missing | Unit | Mock SDK without review resource |
| `integrityAvailable` false when sdk.integrity missing | Unit | Mock SDK without integrity resource |
| `notesAvailable` false when sdk.notes missing | Unit | Mock SDK without notes resource |
| Logs list flagged tools when some flags false | Unit | Capture console.error |

---

### T72.13: End-to-End Workflow Test

**File:** `packages/mcp/tests/e2e/full-resume-workflow.test.ts`

[CRITICAL] This test verifies the full resume build workflow described in the spec. It uses a mock SDK (not a live server) to test the MCP tool orchestration layer. Each step calls the MCP tool handler directly and verifies the response format.

[IMPORTANT] The test does not verify SDK behavior -- it only verifies that the MCP tool layer correctly translates between MCP parameters and SDK calls, and that the response format matches MCP expectations.

```typescript
import { describe, it, expect } from 'bun:test'
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
// ... imports for tool registration and mock SDK

describe('Full Resume Build Workflow (E2E)', () => {
  // Test fixture: mock SDK with canned responses
  // Each SDK method returns a pre-defined Result<T>

  it('Step 1: forge_health verifies server connectivity', async () => {
    // Call forge_health tool
    // Assert: response.content[0].type === 'text'
    // Assert: parsed JSON contains { server: 'ok', api_version: string }
  })

  it('Step 2: forge_create_organization creates employer', async () => {
    // Call forge_create_organization with name, org_type, tag
    // Assert: response.content[0].type === 'text'
    // Assert: parsed JSON contains { id: string, name: string, org_type: 'company' }
  })

  it('Step 3: forge_ingest_job_description stores JD', async () => {
    // Call forge_ingest_job_description with title, raw_text, org ID
    // Assert: parsed JSON contains { id: string, title: string, embedding_status: any }
    // Assert: embedding_status is present (null or value)
  })

  it('Step 4: forge_match_requirements finds matching perspectives', async () => {
    // Call forge_match_requirements with JD ID, entity_type: 'perspective'
    // Assert: parsed JSON contains { candidates: Array, match_count: number }
  })

  it('Step 5: forge_create_resume creates resume from template', async () => {
    // Call forge_create_resume with name, target_role, archetype, template_id
    // Assert: parsed JSON contains { id: string, name: string, sections: Array }
  })

  it('Step 6: forge_add_resume_entry adds best matches', async () => {
    // Call forge_add_resume_entry with resume_id, section_id, perspective_id
    // Assert: parsed JSON contains { id: string, perspective_id: string }
  })

  it('Step 7: forge_gap_analysis checks domain coverage', async () => {
    // Call forge_gap_analysis with resume_id
    // Assert: parsed JSON contains { covered: Array, missing: Array, thin: Array }
  })

  it('Step 8: forge_align_resume checks alignment score', async () => {
    // Call forge_align_resume with JD ID, resume ID
    // Assert: parsed JSON contains { overall_score: number }
    // Assert: overall_score is between 0 and 1
  })

  it('Step 9: forge_export_resume exports as markdown', async () => {
    // Call forge_export_resume with resume_id, format: 'markdown'
    // Assert: response.content[0].type === 'text'
    // Assert: text contains markdown heading (starts with #)
  })
})

describe('Feature-flagged tool behavior', () => {
  it('skips Phase 60 tools when SDK methods missing', async () => {
    // Create mock SDK WITHOUT linkResume/unlinkResume
    // Register all tools
    // const tools = await server.listTools()
    // Assert: tools.find(t => t.name === 'forge_link_resume_to_jd') === undefined
    // Assert: tools.find(t => t.name === 'forge_unlink_resume_from_jd') === undefined
  })

  it('skips Phase 62 tools when SDK methods missing', async () => {
    // Create mock SDK WITHOUT extractSkills/addSkill/removeSkill
    // Register all tools
    // const tools = await server.listTools()
    // Assert: tools.find(t => t.name === 'forge_extract_jd_skills') === undefined
    // Assert: tools.find(t => t.name === 'forge_tag_jd_skill') === undefined
    // Assert: tools.find(t => t.name === 'forge_untag_jd_skill') === undefined
  })

  it('registers all 57 tools when all SDK methods present', async () => {
    // Create mock SDK WITH all methods
    // Register all tools
    // const tools = await server.listTools()
    // Assert: tools.length === 57
  })
})

describe('Contract: MCP tool definitions', () => {
  it('all registered tools have valid JSON schema parameters', async () => {
    // const tools = await server.listTools()
    // for (const tool of tools) {
    //   Assert: tool.inputSchema is a valid JSON schema
    //   Assert: tool.name starts with 'forge_'
    //   Assert: tool.description is non-empty
    // }
  })
})
```

> **Note:** Extend Phase 71 integration tests to verify that the 21 Tier 0+1 tools still register correctly after Phase 72 registration modules are wired in. This ensures no regressions from import order or shared state.

**Acceptance criteria:**
- All 9 workflow steps pass with mock SDK returning canned data.
- Each step verifies the MCP response format (`content[0].type === 'text'`, valid JSON in `text`).
- Each step includes concrete assertions on the parsed response data (not just format checks).
- Feature flag tests verify correct tool count under different SDK configurations.
- Contract test verifies all tools have valid schemas, `forge_` prefix, and non-empty descriptions.
- Tests run with `bun test` and complete in under 5 seconds (no network, no server).
- Tool count derived programmatically from `server.listTools()`, not hardcoded.

**Failure criteria:**
- Tests require a running Forge server -- must use mock SDK only.
- Tests don't verify MCP response format -- only check SDK calls.
- Feature flag tests don't count registered tools.
- Tool count hardcoded instead of derived from `server.listTools()`.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| Full workflow 9-step sequence | E2E | Mock SDK with canned responses |
| Feature flag: Phase 60 tools skipped | E2E | Mock SDK without Phase 60 methods |
| Feature flag: Phase 62 tools skipped | E2E | Mock SDK without Phase 62 methods |
| Feature flag: all 57 tools registered | E2E | Full mock SDK |
| Contract: all tools have valid definitions | E2E | Full mock SDK |
| `forge_create_source` with source_type 'project' | Unit | Mock SDK returning project source |

---

### T72.14: Response Truncation Utility

**File:** `packages/mcp/src/truncation.ts`

[IMPORTANT] MCP text content has no formal size limit, but large payloads degrade AI reasoning quality. Cap at 50KB and indicate truncation.

> **Portability note:** `Buffer.byteLength` is used for byte-length measurement. This is available in Node.js and Bun. If this code needs to run in a browser environment, replace with `new TextEncoder().encode(json).length`. For now, Bun is the only target runtime, so `Buffer.byteLength` is fine.

```typescript
const MAX_RESPONSE_BYTES = 50 * 1024  // 50KB

export interface TruncationResult {
  text: string
  truncated: boolean
}

/**
 * Truncate a JSON response if it exceeds MAX_RESPONSE_BYTES.
 *
 * When truncated, the response includes a _truncated flag and a message
 * telling the AI to use more specific filters or pagination.
 */
export function truncateResponse(data: unknown): TruncationResult {
  const json = JSON.stringify(data, null, 2)

  if (Buffer.byteLength(json, 'utf8') <= MAX_RESPONSE_BYTES) {
    return { text: json, truncated: false }
  }

  // For arrays, truncate by reducing items
  if (Array.isArray(data)) {
    let sliceSize = Math.max(10, Math.floor(data.length / 4))
    let truncated = {
      _truncated: true,
      _message: `Response truncated (${data.length} items). Use offset/limit pagination or more specific filters to get remaining results.`,
      _total_items: data.length,
      items: data.slice(0, sliceSize),
    }
    // Binary reduction: if still over limit, halve slice until it fits
    let result = JSON.stringify(truncated, null, 2)
    while (Buffer.byteLength(result, 'utf8') > MAX_RESPONSE_BYTES && sliceSize > 1) {
      sliceSize = Math.floor(sliceSize / 2)
      truncated.items = data.slice(0, sliceSize)
      result = JSON.stringify(truncated, null, 2)
    }
    return { text: result, truncated: true }
  }

  // For objects with array properties, truncate the largest array
  if (typeof data === 'object' && data !== null) {
    const obj = data as Record<string, unknown>
    const clone = { ...obj }

    for (const [key, value] of Object.entries(clone)) {
      if (Array.isArray(value) && value.length > 20) {
        (clone as any)[key] = value.slice(0, 20)
        ;(clone as any)[`_${key}_truncated`] = true
        ;(clone as any)[`_${key}_total`] = value.length
      }
    }

    let result = JSON.stringify(
      { ...clone, _truncated: true, _message: 'Large arrays truncated. Use pagination for full results.' },
      null,
      2,
    )
    // Post-truncation size check: if still over limit, halve array slices
    let maxItems = 20
    while (Buffer.byteLength(result, 'utf8') > MAX_RESPONSE_BYTES && maxItems > 1) {
      maxItems = Math.floor(maxItems / 2)
      for (const [key, value] of Object.entries(obj)) {
        if (Array.isArray(value) && value.length > maxItems) {
          (clone as any)[key] = value.slice(0, maxItems)
          ;(clone as any)[`_${key}_truncated`] = true
          ;(clone as any)[`_${key}_total`] = value.length
        }
      }
      result = JSON.stringify(
        { ...clone, _truncated: true, _message: 'Large arrays truncated. Use pagination for full results.' },
        null,
        2,
      )
    }
    return { text: result, truncated: true }
  }

  // Fallback: hard truncate
  const truncatedJson = json.slice(0, MAX_RESPONSE_BYTES - 200)
  const fallback = {
    _truncated: true,
    _message: 'Response too large. Use more specific filters.',
    _partial: truncatedJson,
  }
  return { text: JSON.stringify(fallback), truncated: true }
}
```

**Sub-task: Wire truncation into respond helper**

The `mapResult` helper (from Phase 71's `sdkResultToMcpResponse`) should call `truncateResponse()` on the serialized data before returning. This ensures all tools benefit from truncation without each handler needing to call it explicitly. Update `packages/mcp/src/response.ts` to import and apply `truncateResponse` internally.

**Acceptance criteria:**
- Responses under 50KB pass through unchanged.
- Array responses over 50KB are reduced to ~25% of items with `_truncated` metadata.
- Object responses with large arrays truncate those arrays to 20 items each.
- Truncated responses always include `_truncated: true` and `_message`.
- Post-truncation size check: if result is still over 50KB after initial truncation, binary reduction loop halves slice sizes until it fits.

**Failure criteria:**
- Truncation silently drops data without any indicator.
- Truncation threshold too low (< 10KB) -- many normal responses would be truncated.
- Truncation produces invalid JSON.
- Truncated result still exceeds 50KB (no post-truncation size check).

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| Small response passes through unchanged | Unit | 1KB JSON object |
| Large array response truncated with metadata | Unit | 1000-item array |
| Object with large nested array truncated | Unit | Object with 500-item array property |
| Truncated result is valid JSON | Unit | Various truncation scenarios |
| Post-truncation binary reduction keeps result under 50KB | Unit | Extremely large items (each > 1KB) |

---

### T72.15: Documentation

**File:** `packages/mcp/README.md`

[IMPORTANT] The README serves as the primary reference for users configuring Claude Code or Claude Desktop to use the Forge MCP server.

**Sections to include:**

1. **Overview** -- what the MCP server does, relationship to `@forge/sdk` and `@forge/core`
2. **Prerequisites** -- Forge HTTP server must be running, Bun runtime
3. **Installation** -- `bun install` in the monorepo
4. **Configuration for Claude Code** -- `claude mcp add` command with STDIO transport
5. **Configuration for Claude Desktop** -- `claude_desktop_config.json` example
6. **Tool Reference Table** -- all 57 tools with columns: Tool Name, Tier, Parameters, Returns, SDK Method
7. **Resources** -- list of 7 MCP resources with URI patterns
8. **Feature Flags** -- explanation of Phase 60/62 dependent tools, plus `reorderEntries` being feature-flagged off
9. **Troubleshooting** -- common errors:
   - "Cannot reach Forge server" -- start the server
   - "SDK method signature mismatch" -- version mismatch
   - "Response truncated" -- use pagination
   - Feature-flagged tools not appearing -- Phase 60/62 not implemented
10. **Environment Variables** -- `FORGE_API_URL` (default `http://localhost:3000`)

**Tool reference table format:**

```markdown
| Tool | Tier | Description |
|------|------|-------------|
| `forge_health` | 0 | Check connectivity to Forge server |
| `forge_search_sources` | 1 | Search experience sources |
| ... | ... | ... |
| `forge_search_notes` | 3 | Search notes by content |
```

**Claude Code configuration example:**

```bash
claude mcp add forge -- bun run --cwd /path/to/forge/packages/mcp src/index.ts
```

**Claude Desktop configuration example:**

```json
{
  "mcpServers": {
    "forge": {
      "command": "bun",
      "args": ["run", "--cwd", "/path/to/forge/packages/mcp", "src/index.ts"],
      "env": {
        "FORGE_API_URL": "http://localhost:3000"
      }
    }
  }
}
```

**Acceptance criteria:**
- README includes all 57 tools in the reference table.
- Claude Code and Claude Desktop configuration examples are correct and tested.
- Troubleshooting section covers the 4 most common errors.
- Feature flag section explains which tools may be missing and why, including `reorderEntries`.
- Environment variable is `FORGE_API_URL` (matching Phase 71).
- README is well-structured, scannable, and suitable as primary onboarding doc.
- README quality: no broken links, no placeholder text, all code examples are syntactically valid.

**Failure criteria:**
- Tool reference table is incomplete or has wrong tier assignments.
- Configuration examples use wrong command syntax.
- No troubleshooting section.
- Environment variable inconsistent with Phase 71 (`FORGE_BASE_URL` vs `FORGE_API_URL`).

---

### T72.16: Wire Registration in Main Entry Point

**File:** `packages/mcp/src/index.ts`

[CRITICAL] The main entry point must call `detectFeatures()` first, then pass the `FeatureFlags` to each registration function that needs it. The order of registration does not matter for MCP, but grouping by tier makes the startup logs readable.

```typescript
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { ForgeClient } from '@forge/sdk'
import { detectFeatures } from './feature-flags'
import { mapResult } from './response'  // Phase 71 helper (standardized name)
// ... import all registration functions

const baseUrl = process.env.FORGE_API_URL ?? 'http://localhost:3000'
const sdk = new ForgeClient({ baseUrl })
const server = new McpServer({ name: 'forge', version: '1.0.0' })

// Phase 71: Tier 0 + 1 tools already registered here

// Phase 72: Detect feature flags
const flags = detectFeatures(sdk)

// Phase 72: Tier 2 tools
registerTier2EntityCreationTools(server, sdk, mapResult)
registerTier2ResumeMetadataTools(server, sdk, mapResult)
registerTier2JDTools(server, sdk, mapResult, flags)
registerTier2JDLinkageTools(server, sdk, mapResult, flags)
registerTier2MonitoringTools(server, sdk, mapResult, flags)
registerTier2SearchTools(server, sdk, mapResult)

// Phase 72: Tier 3 tools
registerTier3UpdateTools(server, sdk, mapResult)
registerTier3AssemblyTools(server, sdk, mapResult, flags)
registerTier3WorkflowTools(server, sdk, mapResult)
registerTier3NoteTools(server, sdk, mapResult, flags)
registerTier3JDUpdateTool(server, sdk, mapResult)

// Derive tool count programmatically for startup log
const registeredTools = await server.listTools()
console.error(`[forge:mcp] ${registeredTools.length} tools registered`)

// Connect transport
const transport = new StdioServerTransport()
await server.connect(transport)
```

**Acceptance criteria:**
- `detectFeatures()` called before any tier 2/3 registration.
- `flags` passed to `registerTier2JDTools`, `registerTier2JDLinkageTools`, `registerTier2MonitoringTools`, `registerTier3AssemblyTools`, and `registerTier3NoteTools`.
- All 11 registration functions called in tier order.
- Server connects via STDIO transport after all tools are registered.
- Environment variable is `FORGE_API_URL` (matching Phase 71).
- Response helper imported as `mapResult` (standardized name from Phase 71).
- Tool count derived programmatically from `server.listTools()`, not hardcoded.

**Failure criteria:**
- Feature detection called after tool registration -- flags not available.
- `flags` not passed to functions that need it -- feature-flagged tools registered unconditionally.
- Missing import for any registration function.
- Environment variable uses `FORGE_BASE_URL` instead of `FORGE_API_URL`.

**Tests:**

| Test | Kind | Fixture |
|------|------|---------|
| All registration functions called during startup | Integration | Mock SDK, capture tool count |
| Feature flags plumbed to JD and linkage registration | Integration | Mock SDK, verify tool count matches flags |
| Tool count logged matches server.listTools().length | Integration | Mock SDK |

---

## Test Summary

| Category | Count |
|----------|-------|
| Unit tests (individual tool registration) | ~35 |
| Unit tests (feature flags) | 8 |
| Unit tests (truncation) | 5 |
| E2E tests (workflow + feature flag scenarios) | 5 |
| E2E tests (contract: tool definitions) | 1 |
| Integration tests (wiring) | 3 |
| **Total** | **~57** |

All tests use mock SDK instances. No live server required. Expected test suite runtime: < 10 seconds.

> **Note:** Extend Phase 71 integration tests to verify that the 21 Tier 0+1 tools still register correctly after Phase 72 registration modules are wired in. This ensures no regressions from import order or shared state.

## Completion Criteria

- [ ] All 36 Tier 2+3 tools registered (57 total with Phase 71's 21)
- [ ] Feature flags correctly detect Phase 60/62 SDK method availability
- [ ] Feature flags detect `sdk.review`, `sdk.integrity`, `sdk.notes` availability
- [ ] Feature-flagged tools gracefully omitted when dependencies missing
- [ ] `forge_reorder_resume_entries` feature-flagged off via `flags.reorderEntries` (not inline `typeof`)
- [ ] `forge_create_source` correctly restructures extension fields for all 5 source types
- [ ] `forge_create_source` with `source_type: 'general'` silently ignores extension fields (documented)
- [ ] `forge_create_note` handles partial reference failures with warnings
- [ ] `forge_create_note` applies `truncateResponse()` before returning
- [ ] Atomic note creation documented as known deviation from spec (create + loop, not atomic)
- [ ] `forge_update_resume_entry` supports copy-on-write reset via `content: null` (nullable SDK type verified)
- [ ] `forge_ingest_job_description` always returns `embedding_status` (value or `null`)
- [ ] Response truncation active for payloads > 50KB with post-truncation binary reduction loop
- [ ] Startup log lists all feature-flagged tools (or confirms all 57 registered)
- [ ] Tool count derived programmatically from `server.listTools()`, not hardcoded
- [ ] E2E workflow test passes with mock SDK, each step has concrete assertions
- [ ] Contract test verifies all tools have valid MCP definitions
- [ ] README includes tool reference table (57 tools), configuration guides, troubleshooting
- [ ] README quality: no broken links, no placeholders, valid code examples
- [ ] All tests pass via `bun test`
- [ ] Environment variable standardized on `FORGE_API_URL` (matching Phase 71)
- [ ] Response helper standardized on `mapResult` name (matching Phase 71 export)
- [ ] All `as any` casts have TODO comments referencing the relevant phase (Phase 60/62)
- [ ] `forge_update_profile` registered in `tier2-resume-metadata.ts` (not entity-creation)
- [ ] Cross-reference comments in `tier3-update.ts` and `tier3-assembly.ts`
- [ ] `forge_search_organizations` uses `tag: string` (singular), not `tags: string[]`
- [ ] Test for `forge_create_source` with `source_type: 'project'`
- [ ] Phase 71 integration tests extended to verify no regressions
