# Phase 62: JD Skill Extraction AI (Spec E6)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-jd-skill-extraction.md](../refs/specs/2026-04-03-jd-skill-extraction.md)
**Depends on:** Phase 49 (JD Detail Page -- JD entity with `raw_text` field and skill tagging via `job_description_skills` junction, migration 018) AND Phase 43 (migration 017 expands `prompt_logs.entity_type` CHECK to include `'job_description'`)
**Blocks:** None
**Parallelizable with:** Phase 60 (JD Resume Linkage), Phase 61 (JD Kanban Pipeline) -- all three modify `job-descriptions.ts` routes and SDK resource; serialize changes to shared files

## Goal

Add an "Extract Skills" button to the JD skill tagging section that invokes Claude CLI with the JD's `raw_text`, validates the AI response, and presents extracted skills in a review panel. Users accept (link to JD), reject (dismiss), or edit (rename before linking) each suggested skill. No skills are auto-persisted -- human review is mandatory. Every extraction attempt is logged in `prompt_logs` for auditability.

## Non-Goals

- Auto-matching JD skills to resume skills or perspectives (Spec E4 -- gap analysis)
- Parsing salary, location, or other structured fields from JD text
- Recurring or automatic extraction (user must click the button each time)
- Extracting experience level requirements (e.g., "5+ years")
- Extracting education requirements
- Confidence-based auto-accept (all suggestions require human review)
- Batch extraction across multiple JDs
- Custom prompt editing by the user
- Caching extraction results (re-extracting always re-invokes Claude)

## Context

Phase 49 builds the JD detail page with skill tagging: migration 018 creates the `job_description_skills` junction table, the API provides `POST /api/job-descriptions/:id/skills { name }` for linking skills (with auto-creation via `capitalizeFirst` and case-insensitive dedup), and the UI has a `JDSkillPicker.svelte` for manual skill search and add.

The existing AI module (`packages/core/src/ai/`) provides `invokeClaude()` (Claude CLI wrapper), prompt templates (`renderSourceToBulletPrompt`, `renderBulletToPerspectivePrompt`), and validators (`validateBulletDerivation`, `validatePerspectiveDerivation`). This phase adds a third prompt template and validator following the identical pattern.

The `prompt_logs` table requires `entity_type = 'job_description'` -- this must be supported by migration 017 (from Phase 43 prerequisites) which expands the `prompt_logs.entity_type` CHECK constraint.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. AI extraction flow (end-to-end, design decisions) | Yes |
| 2. AI prompt design (template, examples) | Yes |
| 3. Output validation (validator, rules) | Yes |
| 4. API endpoint (`POST /:id/extract-skills`) | Yes |
| 5. SDK changes (`extractSkills(jdId)`) | Yes |
| 6. UI: skill extraction review (button, panel, accept/reject/edit, bulk actions) | Yes |
| 7. Component architecture (JDSkillExtraction, ExtractedSkillCard, ConfidenceBar) | Yes |
| 8. Files to create | Yes |
| 9. Files to modify | Yes |
| 10. Testing | Yes |
| 11. Acceptance criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/jd/JDSkillExtraction.svelte` | "Extract Skills" button + review panel |
| `packages/webui/src/lib/components/jd/ExtractedSkillCard.svelte` | Individual extracted skill row with accept/reject/edit |
| `packages/webui/src/lib/components/ConfidenceBar.svelte` | Visual confidence score bar (reusable) |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/ai/prompts.ts` | Add `renderJDSkillExtractionPrompt()` function and `JD_SKILL_EXTRACTION_TEMPLATE_VERSION` constant |
| `packages/core/src/ai/validator.ts` | Add `SkillExtractionResponse` type and `validateSkillExtraction()` function |
| `packages/core/src/ai/index.ts` | Re-export new prompt and validator symbols |
| `packages/core/src/routes/job-descriptions.ts` | Add `POST /:id/extract-skills` endpoint |
| `packages/sdk/src/resources/job-descriptions.ts` | Add `extractSkills(jdId)` method |
| `packages/sdk/src/types.ts` | Add `ExtractedSkill` and `SkillExtractionResult` types |
| `packages/core/src/types/index.ts` | Add `ExtractedSkill` and `SkillExtractionResult` types |
| `packages/webui/src/lib/components/jd/JDEditor.svelte` (or `JDSkillPicker.svelte`) | Mount `JDSkillExtraction` component next to the skill picker |

## Fallback Strategies

- **Claude CLI not installed or not found:** `invokeClaude()` returns `{ ok: false, error: 'NOT_FOUND' }`. The endpoint returns 502 with `AI_ERROR` code. The UI shows an error toast: "Claude Code CLI not found." No crash, no partial data.
- **Claude CLI times out:** `invokeClaude()` returns `{ ok: false, error: 'TIMEOUT' }`. Logged as a failed extraction in `prompt_logs`. UI shows error toast with timeout message.
- **AI response is invalid JSON:** `parseClaudeEnvelope()` fails. Logged as error. UI shows error toast.
- **AI response passes JSON parse but fails validation:** `validateSkillExtraction()` returns `{ ok: false, error: '...' }`. Logged as error with the validation message. UI shows error toast.
- **AI returns empty skills array:** Valid response (`ok: true`) with a warning "No skills extracted." The review panel shows a message: "No skills were extracted from this job description. The text may not contain identifiable technical requirements."
- **AI returns unknown category:** Validation passes with a warning (category is informational, not structural). The unknown category is displayed as-is in the UI.
- **Confidence outside [0, 1]:** Clamped with a warning. UI displays the clamped value.
- **JD has empty `raw_text`:** The endpoint returns 400 `VALIDATION_ERROR` before invoking Claude. UI shows the error; button remains enabled.
- **User navigates away during extraction:** The extraction is async. If the component unmounts, the response is discarded. No orphaned state. Re-extracting on return starts fresh.
- **Skill auto-creation race condition:** Two concurrent `POST /api/job-descriptions/:id/skills { name: 'Python' }` calls. The existing endpoint uses case-insensitive dedup (`SELECT ... WHERE LOWER(name) = LOWER(?)`), so both resolve to the same skill. The junction insert is `INSERT OR IGNORE`, so the duplicate link is harmless.
- **`prompt_logs.entity_type` CHECK does not include `'job_description'`:** If migration 017 has not added `'job_description'` to the CHECK, the prompt log insert will fail. The extraction endpoint catches this and still returns the extracted skills (logging is best-effort). Log a warning to stderr.

---

## Tasks

### T62.1: Add Prompt Template

**File:** `packages/core/src/ai/prompts.ts`

[CRITICAL] The prompt must request JSON output with a specific schema (`{ skills: [{ name, category, confidence }] }`). The instructions must distinguish between required skills (confidence >= 0.8), preferred/nice-to-have (0.5-0.7), and implied (0.3-0.5). The prompt must NOT extract generic soft skills, years of experience, or degree requirements.

[IMPORTANT] The template version string is used for prompt logging and must be unique across all templates.

Add after the existing `renderBulletToPerspectivePrompt` function:

```typescript
// ---------------------------------------------------------------------------
// JD -> Skill Extraction
// ---------------------------------------------------------------------------

export const JD_SKILL_EXTRACTION_TEMPLATE_VERSION = 'jd-skill-extraction-v1'

/**
 * Render the JD skill extraction prompt.
 *
 * @param rawText - The full job description text to extract skills from.
 * @returns The fully rendered prompt string.
 */
export function renderJDSkillExtractionPrompt(rawText: string): string {
  return `You are a technical recruiter assistant. Given a job description, extract the
technical skills, tools, technologies, and competencies that are required or preferred
for the role. For each skill, provide:
- The skill name (normalized: proper casing, common abbreviation)
- A category (one of: language, framework, tool, platform, methodology, domain, soft_skill, certification, other)
- A confidence score (0.0 to 1.0) indicating how clearly the JD states this is required

Rules:
- Extract specific technologies, not vague terms (e.g., "Python" not "programming")
- Use the most common/recognized name for each skill (e.g., "Kubernetes" not "K8s", "AWS" not "Amazon Web Services")
- Include both required and preferred/nice-to-have skills
- Set confidence >= 0.8 for explicitly required skills
- Set confidence 0.5-0.7 for preferred/nice-to-have skills
- Set confidence 0.3-0.5 for implied skills (mentioned in context but not as a requirement)
- Do NOT extract generic job requirements (e.g., "communication skills", "team player") unless they are specifically technical competencies
- Do NOT extract years of experience as skills
- Do NOT extract degree requirements as skills

Job description:
---
${rawText}
---

Respond with a JSON object:
{
  "skills": [
    {
      "name": "skill name",
      "category": "language | framework | tool | platform | methodology | domain | soft_skill | certification | other",
      "confidence": 0.9
    }
  ]
}`
}
```

**Acceptance criteria:**
- `renderJDSkillExtractionPrompt(rawText)` returns a string containing the raw JD text.
- The prompt includes JSON output format instructions.
- `JD_SKILL_EXTRACTION_TEMPLATE_VERSION` is `'jd-skill-extraction-v1'`.
- The prompt specifies the 9 valid category values.
- The prompt specifies the confidence score ranges.
- The prompt excludes generic soft skills, years of experience, and degree requirements.

**Failure criteria:**
- Template version collides with existing versions (`source-to-bullet-v1`, `bullet-to-perspective-v1`).
- Missing category list in prompt -- AI returns arbitrary categories.
- Missing confidence guidance -- AI returns all 1.0 or inconsistent values.

---

### T62.2: Add Output Validator

**File:** `packages/core/src/ai/validator.ts`

[CRITICAL] The validator must handle gracefully: empty skills array (valid with warning), unknown categories (warning, not error), confidence outside [0, 1] (clamp with warning), extra fields (warning, not error). The validator follows the exact same pattern as `validateBulletDerivation` and `validatePerspectiveDerivation`.

[IMPORTANT] An empty skills array is valid (returns `ok: true`), unlike `validateBulletDerivation` which treats an empty array as an error. This is intentional -- a JD might have no extractable technical skills.

Add after the existing `validatePerspectiveDerivation` function:

```typescript
// ---------------------------------------------------------------------------
// Skill extraction validator
// ---------------------------------------------------------------------------

export interface SkillExtractionResponse {
  skills: Array<{
    name: string
    category: string
    confidence: number
  }>
}

const SKILL_EXTRACTION_ROOT_FIELDS = new Set(['skills'])
const SKILL_ITEM_FIELDS = new Set(['name', 'category', 'confidence'])
const VALID_SKILL_CATEGORIES = new Set([
  'language', 'framework', 'tool', 'platform',
  'methodology', 'domain', 'soft_skill', 'certification', 'other',
])

export function validateSkillExtraction(
  data: unknown,
): ValidationResult<SkillExtractionResponse> {
  const warnings: string[] = []

  if (data === null || data === undefined || typeof data !== 'object') {
    return { ok: false, error: 'Response is not an object' }
  }

  const obj = data as Record<string, unknown>

  // Extra fields at root level
  const rootExtra = extraFields(obj, SKILL_EXTRACTION_ROOT_FIELDS)
  if (rootExtra.length > 0) {
    warnings.push(`Unexpected root fields: ${rootExtra.join(', ')}`)
  }

  // .skills must exist and be an array
  if (!('skills' in obj)) {
    return { ok: false, error: 'Missing required field "skills"' }
  }

  if (!Array.isArray(obj.skills)) {
    return { ok: false, error: '"skills" must be an array' }
  }

  // Empty array is valid (JD may have no extractable skills)
  // but warn about it
  if (obj.skills.length === 0) {
    warnings.push('No skills extracted from job description')
  }

  // Validate each skill item
  const skills: SkillExtractionResponse['skills'] = []
  for (let i = 0; i < obj.skills.length; i++) {
    const item = obj.skills[i]
    const prefix = `skills[${i}]`

    if (item === null || item === undefined || typeof item !== 'object') {
      return { ok: false, error: `${prefix} is not an object` }
    }

    const skill = item as Record<string, unknown>

    // Extra fields on skill items
    const itemExtra = extraFields(skill, SKILL_ITEM_FIELDS)
    if (itemExtra.length > 0) {
      warnings.push(`${prefix}: unexpected fields: ${itemExtra.join(', ')}`)
    }

    // name -- required, non-empty string
    if (typeof skill.name !== 'string') {
      return { ok: false, error: `${prefix}.name must be a string` }
    }
    if (skill.name.trim().length === 0) {
      return { ok: false, error: `${prefix}.name must be non-empty` }
    }

    // category -- required, must be a string
    if (typeof skill.category !== 'string') {
      return { ok: false, error: `${prefix}.category must be a string` }
    }
    if (!VALID_SKILL_CATEGORIES.has(skill.category)) {
      // Warn but don't fail -- category is for display purposes
      warnings.push(`${prefix}.category "${skill.category}" is not a recognized category`)
    }

    // confidence -- required, number between 0 and 1
    if (typeof skill.confidence !== 'number') {
      return { ok: false, error: `${prefix}.confidence must be a number` }
    }
    let confidence = skill.confidence
    if (confidence < 0 || confidence > 1) {
      warnings.push(`${prefix}.confidence ${confidence} is outside [0, 1] range, clamping`)
      confidence = Math.max(0, Math.min(1, confidence))
    }

    skills.push({
      name: skill.name.trim(),
      category: skill.category,
      confidence,
    })
  }

  return { ok: true, data: { skills }, warnings }
}
```

**Acceptance criteria:**
- Valid response with skills array passes validation.
- Empty skills array passes with warning "No skills extracted from job description".
- Missing `skills` field fails (`ok: false`).
- `skills` as non-array fails.
- Skill with missing/non-string `name` fails.
- Skill with empty `name` fails.
- Skill with missing/non-string `category` fails.
- Skill with missing/non-number `confidence` fails.
- Confidence outside [0, 1] passes with warning and clamped value.
- Unknown category passes with warning.
- Extra root fields produce warning (not error).
- Extra skill item fields produce warning (not error).

**Failure criteria:**
- Empty skills array treated as error (should be valid).
- Unknown category treated as error (should be warning).
- Confidence clamping not applied -- downstream code receives values > 1 or < 0.

---

### T62.3: Update AI Module Re-exports

**File:** `packages/core/src/ai/index.ts`

Add re-exports for the new prompt template and validator:

```typescript
// Prompt templates
export {
  renderSourceToBulletPrompt,
  renderBulletToPerspectivePrompt,
  renderJDSkillExtractionPrompt,
  SOURCE_TO_BULLET_TEMPLATE_VERSION,
  BULLET_TO_PERSPECTIVE_TEMPLATE_VERSION,
  JD_SKILL_EXTRACTION_TEMPLATE_VERSION,
} from './prompts'

// Output validators
export {
  validateBulletDerivation,
  validatePerspectiveDerivation,
  validateSkillExtraction,
} from './validator'
export type {
  BulletDerivationResponse,
  PerspectiveDerivationResponse,
  SkillExtractionResponse,
  ValidationResult,
} from './validator'
```

**Acceptance criteria:**
- `renderJDSkillExtractionPrompt` and `JD_SKILL_EXTRACTION_TEMPLATE_VERSION` are importable from `@forge/core/ai`.
- `validateSkillExtraction` and `SkillExtractionResponse` are importable from `@forge/core/ai`.

---

### T62.4: Add Core Type Definitions

**File:** `packages/core/src/types/index.ts`

```typescript
/** A skill extracted from a JD by AI, pending human review. */
export interface ExtractedSkill {
  name: string
  category: string
  confidence: number
}

/** Result of AI skill extraction from a JD. */
export interface SkillExtractionResult {
  skills: ExtractedSkill[]
  warnings: string[]
}
```

**Acceptance criteria:**
- Both interfaces export from the core types barrel.
- `ExtractedSkill` matches the validator's `SkillExtractionResponse.skills[*]` shape.

---

### T62.5: Add SDK Type Definitions

**File:** `packages/sdk/src/types.ts`

Mirror from T62.4:

```typescript
export interface ExtractedSkill {
  name: string
  category: string
  confidence: number
}

export interface SkillExtractionResult {
  skills: ExtractedSkill[]
  warnings: string[]
}
```

**Acceptance criteria:**
- Both interfaces exported from SDK types.
- Field names and types match T62.4 exactly.

---

### T62.6: Add Extract-Skills API Endpoint

**File:** `packages/core/src/routes/job-descriptions.ts`

[CRITICAL] The endpoint does NOT auto-link skills. It returns suggested skills for frontend review. No data is persisted except the prompt log entry. The frontend must call `POST /api/job-descriptions/:id/skills { name }` for each accepted skill individually.

[CRITICAL] Every extraction attempt (success or failure) is logged in `prompt_logs` with `entity_type = 'job_description'` and `prompt_template = 'jd-skill-extraction-v1'`.

[IMPORTANT] The `raw_text` field must be non-empty. Return 400 if the JD has no text to extract from.

[IMPORTANT] If `rawResponse` is undefined (e.g., on TIMEOUT before any output), store empty string `''` to satisfy the NOT NULL constraint on `prompt_logs.raw_response`.

Add after the existing JD resume linkage endpoints (or after the base CRUD endpoints), before `return app`:

```typescript
  // ── JD Skill Extraction (AI) ──────────────────────────────────────

  app.post('/job-descriptions/:id/extract-skills', async (c) => {
    const { id } = c.req.param()

    // 1. Fetch the JD
    const jdResult = services.jobDescriptions.get(id)
    if (!jdResult.ok) {
      return c.json({ error: jdResult.error }, mapStatusCode(jdResult.error.code))
    }
    const jd = jdResult.data

    // 2. Validate raw_text exists and is non-empty
    if (!jd.raw_text || jd.raw_text.trim().length === 0) {
      return c.json(
        {
          error: {
            code: 'VALIDATION_ERROR',
            message: 'Job description has no text to extract skills from',
          },
        },
        400,
      )
    }

    // 3. Render prompt
    const prompt = renderJDSkillExtractionPrompt(jd.raw_text)

    // 4. Invoke Claude CLI
    const result = await invokeClaude({ prompt })

    // 5. Handle AI errors
    if (!result.ok) {
      // Log the failed attempt
      try {
        services.promptLogs.create({
          prompt_input: prompt,
          prompt_template: JD_SKILL_EXTRACTION_TEMPLATE_VERSION,
          raw_response: result.rawResponse ?? '',
          status: 'error',
          error_message: result.message,
          entity_type: 'job_description',
          entity_id: id,
        })
      } catch (logErr) {
        console.error('[forge] Failed to log prompt error:', logErr)
      }

      return c.json(
        { error: { code: 'AI_ERROR', message: result.message } },
        502,
      )
    }

    // 6. Validate response
    const validated = validateSkillExtraction(result.data)
    if (!validated.ok) {
      try {
        services.promptLogs.create({
          prompt_input: prompt,
          prompt_template: JD_SKILL_EXTRACTION_TEMPLATE_VERSION,
          raw_response: result.rawResponse,
          status: 'error',
          error_message: `Validation failed: ${validated.error}`,
          entity_type: 'job_description',
          entity_id: id,
        })
      } catch (logErr) {
        console.error('[forge] Failed to log prompt validation error:', logErr)
      }

      return c.json(
        {
          error: {
            code: 'AI_ERROR',
            message: `Invalid AI response: ${validated.error}`,
          },
        },
        502,
      )
    }

    // 7. Log successful extraction
    // [FIX] Add nullish fallback for raw_response on the success path.
    // result.rawResponse may be undefined if the AI driver only returns
    // parsed data. Fall back to stringified result.data, then empty string
    // to satisfy the NOT NULL constraint on prompt_logs.raw_response.
    try {
      services.promptLogs.create({
        prompt_input: prompt,
        prompt_template: JD_SKILL_EXTRACTION_TEMPLATE_VERSION,
        raw_response: result.rawResponse ?? JSON.stringify(result.data) ?? '',
        status: 'success',
        entity_type: 'job_description',
        entity_id: id,
      })
    } catch (logErr) {
      console.error('[forge] Failed to log prompt success:', logErr)
    }

    // 8. Return extracted skills (NOT persisted -- user must accept individually)
    return c.json({
      ok: true,
      data: {
        skills: validated.data.skills,
        warnings: validated.warnings,
      },
    })
  })
```

[GAP] The `services.promptLogs` accessor assumes a `PromptLogService` or `PromptLogRepository` is available on the services object. Verify the `Services` type includes `promptLogs`. If not, import the repository directly or extend the services.

[GAP] The `invokeClaude` and `renderJDSkillExtractionPrompt` must be imported at the top of the file. Add:

```typescript
import {
  invokeClaude,
  renderJDSkillExtractionPrompt,
  JD_SKILL_EXTRACTION_TEMPLATE_VERSION,
  validateSkillExtraction,
} from '../ai'
```

[ANTI-PATTERN] The prompt log creation is wrapped in try/catch to prevent logging failures from breaking the extraction response. This is intentional -- logging is best-effort, and the extracted skills are more important than the audit trail. However, failed log writes are logged to stderr so they are not silently swallowed.

**Acceptance criteria:**
- `POST /api/job-descriptions/:id/extract-skills` for nonexistent JD returns 404.
- Empty `raw_text` returns 400 VALIDATION_ERROR.
- Successful extraction returns 200 with `{ ok: true, data: { skills, warnings } }`.
- AI errors (timeout, parse, process, not found) return 502 with AI_ERROR code.
- Validation failure returns 502 with descriptive message.
- Prompt log created on success with `status: 'success'`.
- Prompt log created on failure with `status: 'error'` and error message.
- No skills are persisted to `job_description_skills` -- response is suggestions only.

**Failure criteria:**
- Missing existence check -- extracting from nonexistent JD returns 500.
- Missing `raw_text` check -- empty text sent to Claude, wasting API call.
- Prompt log failure crashes the extraction response.
- Skills auto-persisted to junction table (violates human review requirement).

---

### T62.7: Add SDK Method

**File:** `packages/sdk/src/resources/job-descriptions.ts`

[IMPORTANT] Add `SkillExtractionResult` to the import from `'../types'`.

```typescript
  /** Extract skills from JD text using AI. Returns suggested skills for review. */
  extractSkills(jdId: string): Promise<Result<SkillExtractionResult>> {
    return this.request<SkillExtractionResult>(
      'POST',
      `/api/job-descriptions/${jdId}/extract-skills`,
    )
  }
```

**Acceptance criteria:**
- `extractSkills(jdId)` calls `POST /api/job-descriptions/:jdId/extract-skills`.
- Return type is `Result<SkillExtractionResult>`.
- No request body (the endpoint reads `raw_text` from the JD record).

---

### T62.8: Build ConfidenceBar Component

**File:** `packages/webui/src/lib/components/ConfidenceBar.svelte`

[MINOR] This is a reusable component that visually represents a 0-1 confidence score. It can be used in future AI review UIs beyond skill extraction.

[STYLE] Use design tokens (`var(--color-...)`) where possible in all new components (ConfidenceBar, ExtractedSkillCard, JDSkillExtraction). For ECharts option values that cannot use CSS vars, use the `resolveTokenColor()` utility from Phase 59. Avoid hardcoded hex values in component `<style>` blocks -- prefer `var(--color-success)`, `var(--color-danger)`, `var(--text-muted)`, etc.

```svelte
<script lang="ts">
  let { value, max = 1.0 }: {
    value: number
    max?: number
  } = $props()

  let percentage = $derived(Math.round((value / max) * 100))
  let barColor = $derived(
    value >= 0.8 ? 'var(--confidence-high, #22c55e)' :
    value >= 0.5 ? 'var(--confidence-med, #f59e0b)' :
    'var(--confidence-low, #ef4444)'
  )
</script>

<div class="confidence-bar" title="{value.toFixed(2)}">
  <div
    class="confidence-fill"
    style="width: {percentage}%; background: {barColor};"
  ></div>
</div>

<style>
  .confidence-bar {
    width: 80px;
    height: 6px;
    background: var(--bar-bg, #e5e7eb);
    border-radius: 3px;
    overflow: hidden;
    display: inline-block;
    vertical-align: middle;
  }

  .confidence-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.2s ease;
  }
</style>
```

**Acceptance criteria:**
- Bar width proportional to `value / max`.
- Green for >= 0.8, amber for >= 0.5, red for < 0.5.
- Tooltip shows exact value on hover.
- Width fixed at 80px. Height 6px.
- Graceful rendering for values 0 and 1.

---

### T62.9: Build ExtractedSkillCard Component

**File:** `packages/webui/src/lib/components/jd/ExtractedSkillCard.svelte`

[IMPORTANT] The skill name is editable inline. Clicking the name makes it a text input. The user can rename the skill before accepting. The edited name is passed to the accept callback.

```svelte
<script lang="ts">
  import type { ExtractedSkill } from '@forge/sdk/types'
  import ConfidenceBar from '$lib/components/ConfidenceBar.svelte'

  let { skill, isLinked, onaccept, ondismiss, onedit }: {
    skill: ExtractedSkill
    isLinked: boolean
    onaccept: () => void
    ondismiss: () => void
    onedit: (newName: string) => void
  } = $props()

  let editing = $state(false)
  let editValue = $state(skill.name)

  function startEdit() {
    if (isLinked) return
    editing = true
    editValue = skill.name
  }

  function commitEdit() {
    editing = false
    if (editValue.trim() && editValue.trim() !== skill.name) {
      onedit(editValue.trim())
    }
  }

  function cancelEdit() {
    editing = false
    editValue = skill.name
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') commitEdit()
    if (e.key === 'Escape') cancelEdit()
  }

  const CATEGORY_COLORS: Record<string, string> = {
    language: '#3b82f6',
    framework: '#8b5cf6',
    tool: '#06b6d4',
    platform: '#f59e0b',
    methodology: '#10b981',
    domain: '#ec4899',
    soft_skill: '#6b7280',
    certification: '#f97316',
    other: '#9ca3af',
  }

  let categoryColor = $derived(CATEGORY_COLORS[skill.category] ?? '#9ca3af')
</script>

<div class="extracted-skill-card" class:linked={isLinked}>
  <div class="skill-info">
    {#if editing}
      <input
        type="text"
        class="skill-name-input"
        bind:value={editValue}
        onkeydown={handleKeydown}
        onblur={commitEdit}
        autofocus
      />
    {:else}
      <button
        class="skill-name"
        class:editable={!isLinked}
        onclick={startEdit}
        disabled={isLinked}
        title={isLinked ? 'Already linked' : 'Click to edit name'}
      >
        {skill.name}
      </button>
    {/if}

    <span
      class="category-badge"
      style="background: {categoryColor}20; color: {categoryColor};"
    >
      {skill.category}
    </span>

    <ConfidenceBar value={skill.confidence} />
    <span class="confidence-value">{skill.confidence.toFixed(2)}</span>
  </div>

  <div class="skill-actions">
    {#if isLinked}
      <span class="linked-label muted">Already linked</span>
    {:else}
      <button class="accept-btn" onclick={onaccept}>Accept</button>
      <button class="dismiss-btn" onclick={ondismiss} aria-label="Dismiss">&times;</button>
    {/if}
  </div>
</div>

<style>
  .extracted-skill-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-color, #f3f4f6);
    gap: 8px;
  }

  .extracted-skill-card.linked {
    opacity: 0.5;
  }

  .skill-info {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
  }

  .skill-name {
    font-weight: 500;
    font-size: 0.875rem;
    background: none;
    border: none;
    padding: 0;
    cursor: default;
    text-align: left;
  }

  .skill-name.editable {
    cursor: pointer;
  }

  .skill-name.editable:hover {
    text-decoration: underline;
    text-decoration-style: dotted;
  }

  .skill-name-input {
    font-weight: 500;
    font-size: 0.875rem;
    padding: 1px 4px;
    border: 1px solid var(--border-color, #d1d5db);
    border-radius: 3px;
    width: 140px;
  }

  .category-badge {
    font-size: 0.625rem;
    padding: 1px 6px;
    border-radius: 4px;
    white-space: nowrap;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.03em;
  }

  .confidence-value {
    font-size: 0.75rem;
    color: var(--text-muted, #6b7280);
    min-width: 28px;
    text-align: right;
  }

  .skill-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .accept-btn {
    font-size: 0.75rem;
    padding: 2px 8px;
    border: 1px solid #22c55e;
    border-radius: 4px;
    background: #f0fdf4;
    color: #16a34a;
    cursor: pointer;
  }

  .accept-btn:hover {
    background: #dcfce7;
  }

  .dismiss-btn {
    font-size: 1rem;
    padding: 0 4px;
    border: none;
    background: none;
    color: var(--text-muted, #6b7280);
    cursor: pointer;
  }

  .dismiss-btn:hover {
    color: #ef4444;
  }

  .linked-label {
    font-size: 0.75rem;
    font-style: italic;
  }
</style>
```

**Acceptance criteria:**
- Each skill row shows: name (clickable to edit), category badge (colored), confidence bar + numeric value.
- Already-linked skills show "Already linked" with muted styling; no Accept/Dismiss buttons.
- Clicking name opens inline text input. Enter commits, Escape cancels.
- Edited name is passed to `onedit` callback.
- Accept button calls `onaccept`. Dismiss button calls `ondismiss`.
- Category badge uses color mapping per category.
- Confidence bar and value render correctly for 0-1 range.

---

### T62.10: Build JDSkillExtraction Component

**File:** `packages/webui/src/lib/components/jd/JDSkillExtraction.svelte`

[CRITICAL] The extraction state is ephemeral (component state only). Navigating away clears suggestions. Re-clicking "Extract Skills" replaces any existing suggestions with a new extraction.

[CRITICAL] Accepting a skill calls the existing `POST /api/job-descriptions/:id/skills { name }` endpoint. The skill is created on accept, not on extract. This prevents orphaned skills from rejected suggestions.

[IMPORTANT] "Accept All Remaining" accepts skills sequentially (not in parallel) to avoid race conditions in skill auto-creation. Each accept waits for the previous to complete.

```svelte
<script lang="ts">
  import type { ForgeClient } from '@forge/sdk'
  import type { ExtractedSkill } from '@forge/sdk/types'
  import ExtractedSkillCard from './ExtractedSkillCard.svelte'

  interface LinkedSkill {
    id: string
    name: string
  }

  let { jdId, jdSkills, forge, onSkillsChanged }: {
    jdId: string
    jdSkills: LinkedSkill[]
    forge: ForgeClient
    onSkillsChanged: () => void
  } = $props()

  let extractedSkills = $state<ExtractedSkill[]>([])
  let extracting = $state(false)
  let showPanel = $state(false)
  let errorMessage = $state('')

  // Track which extracted skills have been accepted or dismissed
  let acceptedNames = $state<Set<string>>(new Set())
  let dismissedNames = $state<Set<string>>(new Set())

  // Editable names (user can rename before accepting)
  let editedNames = $state<Map<string, string>>(new Map())

  // Derived: skills available for review
  let reviewableSkills = $derived(
    extractedSkills
      .filter(s => !acceptedNames.has(s.name) && !dismissedNames.has(s.name))
      .filter(s => !jdSkills.some(js =>
        js.name.toLowerCase() === s.name.toLowerCase()
      ))
      .sort((a, b) => b.confidence - a.confidence)
  )

  // Derived: skills already linked (from extraction results)
  let alreadyLinkedExtracted = $derived(
    extractedSkills.filter(s =>
      jdSkills.some(js =>
        js.name.toLowerCase() === s.name.toLowerCase()
      )
    )
  )

  let acceptingAll = $state(false)

  async function handleExtract() {
    extracting = true
    errorMessage = ''
    try {
      const result = await forge.jobDescriptions.extractSkills(jdId)
      if (result.ok) {
        extractedSkills = result.data.skills
        acceptedNames = new Set()
        dismissedNames = new Set()
        editedNames = new Map()
        showPanel = true

        if (result.data.warnings.length > 0) {
          console.warn('[forge] Extraction warnings:', result.data.warnings)
        }
      } else {
        errorMessage = result.error?.message ?? 'Extraction failed'
      }
    } catch (err) {
      errorMessage = 'Network error during extraction'
    } finally {
      extracting = false
    }
  }

  async function handleAccept(skill: ExtractedSkill) {
    const nameToUse = editedNames.get(skill.name) ?? skill.name
    const result = await forge.jobDescriptions.addSkill(jdId, { name: nameToUse })
    if (result.ok) {
      acceptedNames = new Set([...acceptedNames, skill.name])
      onSkillsChanged()
    }
  }

  async function handleAcceptAll() {
    acceptingAll = true
    for (const skill of reviewableSkills) {
      await handleAccept(skill)
    }
    acceptingAll = false
  }

  function handleDismiss(skill: ExtractedSkill) {
    dismissedNames = new Set([...dismissedNames, skill.name])
  }

  function handleDismissAll() {
    for (const skill of reviewableSkills) {
      dismissedNames = new Set([...dismissedNames, skill.name])
    }
    showPanel = false
  }

  function handleEdit(skill: ExtractedSkill, newName: string) {
    editedNames = new Map([...editedNames, [skill.name, newName]])
  }
</script>

<div class="jd-skill-extraction">
  <button
    class="extract-btn"
    onclick={handleExtract}
    disabled={extracting}
  >
    {#if extracting}
      Extracting...
    {:else}
      Extract Skills
    {/if}
  </button>

  {#if errorMessage}
    <p class="error-message">{errorMessage}</p>
  {/if}

  {#if showPanel && extractedSkills.length > 0}
    <div class="extraction-panel">
      <h4>Extracted Skills</h4>

      {#if alreadyLinkedExtracted.length > 0}
        <div class="already-linked-section">
          {#each alreadyLinkedExtracted as skill (skill.name)}
            <ExtractedSkillCard
              {skill}
              isLinked={true}
              onaccept={() => {}}
              ondismiss={() => {}}
              onedit={() => {}}
            />
          {/each}
          <p class="already-linked-note muted">
            {alreadyLinkedExtracted.length} skill{alreadyLinkedExtracted.length === 1 ? '' : 's'} already linked (skipped)
          </p>
        </div>
      {/if}

      {#if reviewableSkills.length > 0}
        <div class="reviewable-section">
          {#each reviewableSkills as skill (skill.name)}
            <ExtractedSkillCard
              {skill}
              isLinked={false}
              onaccept={() => handleAccept(skill)}
              ondismiss={() => handleDismiss(skill)}
              onedit={(newName) => handleEdit(skill, newName)}
            />
          {/each}
        </div>

        <div class="bulk-actions">
          <button
            class="accept-all-btn"
            onclick={handleAcceptAll}
            disabled={acceptingAll}
          >
            {#if acceptingAll}
              Accepting...
            {:else}
              Accept All Remaining ({reviewableSkills.length})
            {/if}
          </button>
          <button class="dismiss-all-btn" onclick={handleDismissAll}>
            Dismiss All
          </button>
        </div>
      {:else if acceptedNames.size > 0 || dismissedNames.size > 0}
        <p class="muted">All extracted skills have been reviewed.</p>
      {/if}
    </div>
  {:else if showPanel && extractedSkills.length === 0}
    <div class="extraction-panel">
      <p class="muted">
        No skills were extracted from this job description. The text may not
        contain identifiable technical requirements.
      </p>
    </div>
  {/if}
</div>

<style>
  .jd-skill-extraction {
    margin-top: 12px;
  }

  .extract-btn {
    font-size: 0.875rem;
    padding: 4px 12px;
    border: 1px solid var(--border-color, #d1d5db);
    border-radius: 4px;
    background: white;
    cursor: pointer;
  }

  .extract-btn:hover:not(:disabled) {
    background: #f9fafb;
  }

  .extract-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error-message {
    color: #dc2626;
    font-size: 0.875rem;
    margin-top: 4px;
  }

  .extraction-panel {
    margin-top: 8px;
    border: 1px solid var(--border-color, #e5e7eb);
    border-radius: 6px;
    padding: 8px;
  }

  .extraction-panel h4 {
    font-size: 0.875rem;
    margin: 0 0 8px 0;
  }

  .already-linked-section {
    margin-bottom: 8px;
    border-bottom: 1px solid var(--border-color, #f3f4f6);
    padding-bottom: 8px;
  }

  .already-linked-note {
    font-size: 0.75rem;
    margin: 4px 0 0 0;
  }

  .bulk-actions {
    display: flex;
    justify-content: space-between;
    padding-top: 8px;
    border-top: 1px solid var(--border-color, #f3f4f6);
    margin-top: 8px;
  }

  .accept-all-btn {
    font-size: 0.75rem;
    padding: 4px 12px;
    border: 1px solid #22c55e;
    border-radius: 4px;
    background: #f0fdf4;
    color: #16a34a;
    cursor: pointer;
  }

  .accept-all-btn:hover:not(:disabled) {
    background: #dcfce7;
  }

  .accept-all-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .dismiss-all-btn {
    font-size: 0.75rem;
    padding: 4px 12px;
    border: 1px solid var(--border-color, #d1d5db);
    border-radius: 4px;
    background: white;
    color: var(--text-muted, #6b7280);
    cursor: pointer;
  }

  .dismiss-all-btn:hover {
    background: #fef2f2;
    color: #dc2626;
    border-color: #fecaca;
  }
</style>
```

[GAP] The `forge.jobDescriptions.addSkill(jdId, { name })` method must exist from Phase 49. Verify the method name and signature. The spec says `POST /api/job-descriptions/:id/skills { name }` is the existing endpoint.

[INCONSISTENCY] The `onSkillsChanged` callback triggers a refresh of the parent's `jdSkills` list. This is necessary so the `alreadyLinkedExtracted` derived state updates correctly after accepting a skill. The parent component must re-fetch skills and pass the updated list down.

**Acceptance criteria:**
- "Extract Skills" button visible in edit mode.
- Clicking shows loading state ("Extracting...").
- On success, review panel appears with extracted skills.
- Already-linked skills shown as muted with "Already linked" label.
- Reviewable skills sorted by confidence (highest first).
- Accept links skill to JD via existing POST endpoint.
- Dismiss removes skill from suggestions (client-side only).
- Inline edit changes skill name before accept.
- "Accept All Remaining" accepts sequentially with progress indicator.
- "Dismiss All" clears all suggestions and closes panel.
- Re-clicking "Extract Skills" replaces previous suggestions.
- Error messages shown on failure.
- Empty extraction handled with informative message.
- Navigating away clears suggestions (ephemeral state).

---

### T62.11: Mount JDSkillExtraction in JD Editor

**File:** `packages/webui/src/lib/components/jd/JDEditor.svelte` (or `JDSkillPicker.svelte`)

[IMPORTANT] The extraction component is placed next to the skill picker section, visible only in edit mode (JD must be saved first). It needs access to the current `jdSkills` array (for dedup against already-linked skills).

```svelte
<!-- In JDEditor.svelte, within the skills section -->
{#if mode === 'edit' && selectedId}
  <JDSkillExtraction
    jdId={selectedId}
    jdSkills={jdSkills}
    {forge}
    onSkillsChanged={loadJDSkills}
  />
{/if}
```

Where `loadJDSkills` is the existing function that refreshes the `jdSkills` state:

```typescript
async function loadJDSkills() {
  const result = await forge.jobDescriptions.listSkills(selectedId)
  if (result.ok) jdSkills = result.data
}
```

**Acceptance criteria:**
- Extraction component visible only in edit mode.
- Component receives current `jdSkills` array for dedup.
- `onSkillsChanged` callback triggers jdSkills refresh in parent.
- `forge` client passed correctly.

---

## Testing Support

### Prompt Tests

| Test | Assertion |
|------|-----------|
| `renderJDSkillExtractionPrompt(text)` contains text | Output includes the provided JD text |
| Prompt includes JSON format | Output includes `"skills"` and `"confidence"` |
| Template version | `JD_SKILL_EXTRACTION_TEMPLATE_VERSION === 'jd-skill-extraction-v1'` |
| Prompt excludes soft skills instruction | Output includes "Do NOT extract generic job requirements" |

### Validator Tests

| Test | Assertion |
|------|-----------|
| Valid response | `{ skills: [{ name: 'Python', category: 'language', confidence: 0.9 }] }` passes |
| Empty skills array | Passes with warning "No skills extracted" |
| Missing `skills` field | Fails: `'Missing required field "skills"'` |
| `skills` as string | Fails: `'"skills" must be an array'` |
| Missing `name` | Fails: `'skills[0].name must be a string'` |
| Empty `name` | Fails: `'skills[0].name must be non-empty'` |
| Missing `category` | Fails: `'skills[0].category must be a string'` |
| Missing `confidence` | Fails: `'skills[0].confidence must be a number'` |
| Confidence > 1 | Passes with warning, clamped to 1.0 |
| Confidence < 0 | Passes with warning, clamped to 0.0 |
| Unknown category `'devops'` | Passes with warning about unrecognized category |
| Extra root field `{ skills: [...], metadata: {} }` | Passes with warning |
| Extra item field `{ name: 'X', category: 'Y', confidence: 0.5, importance: 'high' }` | Passes with warning |
| Null response | Fails: `'Response is not an object'` |
| Skill item is null | Fails: `'skills[0] is not an object'` |

### API Tests

| Test | Assertion |
|------|-----------|
| Nonexistent JD | 404 NOT_FOUND |
| Empty `raw_text` | 400 VALIDATION_ERROR |
| Successful extraction | 200 with `{ ok: true, data: { skills, warnings } }` |
| AI timeout | 502 AI_ERROR |
| AI parse failure | 502 AI_ERROR |
| Validation failure | 502 AI_ERROR with descriptive message |
| Prompt log created on success | Row in `prompt_logs` with `status = 'success'` |
| Prompt log created on failure | Row in `prompt_logs` with `status = 'error'` |
| No skills auto-persisted | `job_description_skills` unchanged after extraction |

### SDK Tests

| Test | Assertion |
|------|-----------|
| `extractSkills(jdId)` success | Returns `{ ok: true, data: { skills, warnings } }` |
| `extractSkills(jdId)` not found | Returns `{ ok: false, error: { code: 'NOT_FOUND' } }` |

### Component Smoke Tests (Manual / Future Playwright)

| Test | What to verify |
|------|---------------|
| "Extract Skills" visible in edit mode | Button present next to skill picker |
| "Extract Skills" hidden in create mode | Button not rendered |
| Loading state | "Extracting..." shown, button disabled |
| Review panel appears | Skills listed after successful extraction |
| Already-linked skills | Shown as muted with "Already linked" label |
| Confidence sort | Highest confidence skills first |
| Accept skill | Calls POST, skill appears as tag in picker |
| Dismiss skill | Removed from suggestions, no API call |
| Inline edit | Click name, type new name, Enter commits |
| Accept All Remaining | Sequential acceptance with progress |
| Dismiss All | Clears suggestions, closes panel |
| Re-extract | New extraction replaces old suggestions |
| Error toast | Shown on AI failure |
| Empty extraction | Message: "No skills were extracted..." |
| ConfidenceBar colors | Green >= 0.8, amber >= 0.5, red < 0.5 |
| Category badge colors | Each category has distinct color |

### Integration Tests

| Test | Assertion |
|------|-----------|
| Extract + accept one | Accept "Python". Verify in `GET /api/job-descriptions/:id/skills`. |
| Auto-create skill | Accept "NewTech" (not in skills table). Verify skill created with `capitalizeFirst`. |
| Case-insensitive dedup | Accept "python" when "Python" exists. Verify no duplicate. |
| Dismiss all | Dismiss all extracted skills. Verify no skills linked. |
| Ephemeral state | Extract skills, navigate away, come back. Verify suggestions gone. |
| Prompt log audit | Extract skills. Query `prompt_logs WHERE entity_id = :jdId`. Verify log entry. |

---

## Documentation Requirements

- No new documentation files required.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on:
  - `renderJDSkillExtractionPrompt`: explains confidence ranges and category set.
  - `validateSkillExtraction`: explains empty array validity, category tolerance, confidence clamping.
  - `SkillExtractionResponse`: documents the AI response schema.
  - `JD_SKILL_EXTRACTION_TEMPLATE_VERSION`: explains versioning for prompt log auditability.
- Inline comments in the route handler for:
  - No auto-persist rationale (human review required).
  - Try/catch around prompt logging (best-effort).
  - `rawResponse ?? ''` for NOT NULL constraint.
- Inline comments in `JDSkillExtraction.svelte` for:
  - Sequential accept-all (race condition avoidance).
  - `onSkillsChanged` callback purpose (parent skill list refresh for dedup).
  - Ephemeral state lifecycle.

---

## Parallelization Notes

**Within this phase:**
- T62.1 (prompt template) and T62.2 (validator) can be written in parallel -- they have no imports between each other.
- T62.3 (re-exports) depends on T62.1 and T62.2.
- T62.4 (core types) and T62.5 (SDK types) can be written in parallel.
- T62.6 (API endpoint) depends on T62.1 (prompt), T62.2 (validator), and T62.4 (types).
- T62.7 (SDK method) depends on T62.5 (SDK types).
- T62.8 (ConfidenceBar) has no dependencies -- can be written at any time.
- T62.9 (ExtractedSkillCard) depends on T62.8 (imports ConfidenceBar) and T62.5 (SDK types).
- T62.10 (JDSkillExtraction) depends on T62.9 (imports ExtractedSkillCard) and T62.7 (SDK method).
- T62.11 (mount in editor) depends on T62.10 (component must exist).

**Recommended execution order:**
1. T62.1 + T62.2 + T62.8 (prompt, validator, ConfidenceBar -- all parallel, no dependencies)
2. T62.3 + T62.4 + T62.5 (re-exports + types -- parallel)
3. T62.6 + T62.7 + T62.9 (endpoint + SDK method + ExtractedSkillCard -- parallel)
4. T62.10 (JDSkillExtraction -- depends on T62.7 and T62.9)
5. T62.11 (mount in editor -- depends on T62.10)

**Cross-phase:**
- Phase 49 (JD Detail Page) must be complete -- this phase uses `POST /api/job-descriptions/:id/skills` and the `JDSkillPicker`/`JDEditor` components.
- Phase 60 and Phase 61 also modify `packages/core/src/routes/job-descriptions.ts`. Serialize all route additions.
- Phase 60 and Phase 61 also modify `packages/sdk/src/types.ts` and `packages/sdk/src/resources/job-descriptions.ts`. Serialize.
- This phase does not conflict with Phase 48 (GraphView) or Phase 43 (Generic Kanban).
- Migration 017 (from Phase 43) must have expanded `prompt_logs.entity_type` CHECK to include `'job_description'` before the extraction endpoint can log prompts. If 017 is not yet applied, the try/catch around logging prevents crashes.
