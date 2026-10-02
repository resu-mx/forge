# JD Skill Extraction (AI)

**Date:** 2026-04-03
**Spec:** E6 (JD Skill Extraction)
**Phase:** TBD (next available)
**Builds on:** Spec E1 (JD Detail Page — JD entity with skill tagging via `job_description_skills` junction)
**Dependencies:** Spec E1 must be complete (JD with `raw_text` field and skill tagging exists)
**Blocks:** None

## Overview

Users paste job description text into the JD detail page and manually tag required skills one by one. This is tedious for JDs with many requirements. This spec adds an "Extract Skills" button that sends the JD's `raw_text` to Claude CLI, receives a list of extracted skills with confidence scores, and presents them for human review. The user can accept (link to JD), reject (dismiss), or edit (rename before linking) each suggested skill.

This follows the same AI pattern as the existing source-to-bullet and bullet-to-perspective derivation: invoke `claude -p <prompt> --output-format json` via the `invokeClaude()` wrapper in `packages/core/src/ai/claude-cli.ts`, validate the JSON response with a dedicated validator, then persist accepted results.

## Non-Goals

- Auto-matching JD skills to resume skills or perspectives (that is Spec E4 — gap analysis)
- Parsing salary, location, or other structured fields from JD text
- Recurring or automatic extraction (user must click the button each time)
- Extracting experience level requirements (e.g., "5+ years")
- Extracting education requirements
- Confidence-based auto-accept (all suggestions require human review)
- Batch extraction across multiple JDs
- Custom prompt editing by the user
- Caching extraction results (re-extracting always re-invokes Claude)

---

## 1. AI Extraction Flow

### 1.1 End-to-End Flow

```
User clicks "Extract Skills"
        │
        ▼
Frontend calls POST /api/job-descriptions/:id/extract-skills
        │
        ▼
Backend reads JD.raw_text from database
        │
        ▼
Backend renders prompt (renderJDSkillExtractionPrompt)
        │
        ▼
Backend invokes Claude CLI (invokeClaude)
        │
        ▼
Backend validates response (validateSkillExtraction)
        │
        ▼
Backend logs the prompt/response in prompt_logs
        │
        ▼
Backend returns extracted skills to frontend
        │
        ▼
Frontend shows skills in review UI (accept / reject / edit)
        │
        ▼
User accepts skills → frontend calls POST /api/job-descriptions/:id/skills for each
        │
        ▼
Accepted skills appear as tags on the JD (same as manually added skills)
```

### 1.2 Key Design Decisions

1. **Extraction does NOT auto-link skills.** The API returns suggested skills, but none are persisted until the user explicitly accepts them. This keeps human review mandatory.
2. **New skills are created on accept, not on extract.** If the AI suggests "Kubernetes" and no skill named "Kubernetes" exists, the skill is created when the user clicks "Accept" — not when the extraction runs. This prevents orphaned skills from rejected suggestions.
3. **The extraction endpoint is stateless.** It does not store the suggestions anywhere. The frontend holds them in component state. If the user navigates away before accepting, the suggestions are lost and must be re-extracted.
4. **Prompt logging.** Each extraction is logged in `prompt_logs` for auditability (same as source-to-bullet and bullet-to-perspective derivations).

> **Note:** Requires migration 017 (from E1 prerequisites) to expand `prompt_logs.entity_type` CHECK to include `'job_description'`.
>
> **Note:** Migration 017 also adds `status TEXT` and `error_message TEXT` columns to `prompt_logs`.

---

## 2. AI Prompt Design

### 2.1 Prompt Template

Add to `packages/core/src/ai/prompts.ts`:

```typescript
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

### 2.2 Example Input

```
We are looking for a Senior Security Engineer to join our team. You will:
- Design and implement security controls for our cloud infrastructure (AWS, GCP)
- Build and maintain CI/CD security scanning pipelines using GitHub Actions
- Conduct threat modeling and security architecture reviews
- Develop automation tools in Python and Go
- Manage container security for Kubernetes workloads
- Experience with Terraform for infrastructure as code is preferred
- CISSP or equivalent certification is a plus
```

### 2.3 Example Output

```json
{
  "skills": [
    { "name": "AWS", "category": "platform", "confidence": 0.9 },
    { "name": "GCP", "category": "platform", "confidence": 0.9 },
    { "name": "GitHub Actions", "category": "tool", "confidence": 0.85 },
    { "name": "Python", "category": "language", "confidence": 0.9 },
    { "name": "Go", "category": "language", "confidence": 0.9 },
    { "name": "Kubernetes", "category": "platform", "confidence": 0.85 },
    { "name": "Terraform", "category": "tool", "confidence": 0.6 },
    { "name": "Threat Modeling", "category": "methodology", "confidence": 0.8 },
    { "name": "CISSP", "category": "certification", "confidence": 0.5 },
    { "name": "CI/CD", "category": "methodology", "confidence": 0.85 },
    { "name": "Container Security", "category": "domain", "confidence": 0.85 }
  ]
}
```

---

## 3. Output Validation

### 3.1 Validator

Add to `packages/core/src/ai/validator.ts`:

```typescript
export interface SkillExtractionResponse {
  skills: Array<{
    name: string
    category: string
    confidence: number
  }>
}

const SKILL_EXTRACTION_ROOT_FIELDS = new Set(['skills'])
const SKILL_ITEM_FIELDS = new Set(['name', 'category', 'confidence'])
const VALID_CATEGORIES = new Set([
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

    // name — required, non-empty string
    if (typeof skill.name !== 'string') {
      return { ok: false, error: `${prefix}.name must be a string` }
    }
    if (skill.name.trim().length === 0) {
      return { ok: false, error: `${prefix}.name must be non-empty` }
    }

    // category — required, must be one of valid categories
    if (typeof skill.category !== 'string') {
      return { ok: false, error: `${prefix}.category must be a string` }
    }
    if (!VALID_CATEGORIES.has(skill.category)) {
      // Warn but don't fail — category is for display purposes
      warnings.push(`${prefix}.category "${skill.category}" is not a recognized category`)
    }

    // confidence — required, number between 0 and 1
    if (typeof skill.confidence !== 'number') {
      return { ok: false, error: `${prefix}.confidence must be a number` }
    }
    if (skill.confidence < 0 || skill.confidence > 1) {
      warnings.push(`${prefix}.confidence ${skill.confidence} is outside [0, 1] range, clamping`)
      skill.confidence = Math.max(0, Math.min(1, skill.confidence))
    }

    skills.push({
      name: skill.name.trim(),
      category: skill.category,
      confidence: skill.confidence,
    })
  }

  return { ok: true, data: { skills }, warnings }
}
```

### 3.2 Validation Rules

- Response must be an object with a `skills` array
- Each skill must have `name` (non-empty string), `category` (string), `confidence` (number 0-1)
- Empty skills array is valid (returns `ok: true` with a warning) — the JD might have no extractable technical skills
- Unknown categories produce a warning but do not fail validation (the AI might use a category not in our predefined set)
- Confidence values outside [0, 1] are clamped with a warning
- Extra fields produce warnings but do not fail validation

---

## 4. API Endpoint

### 4.1 Extract Skills Endpoint

Add to `packages/core/src/routes/job-descriptions.ts`:

```
POST /api/job-descriptions/:id/extract-skills
```

**Request body:** None (reads `raw_text` from the JD record)

**Response (success):**
```json
{
  "ok": true,
  "data": {
    "skills": [
      { "name": "Python", "category": "language", "confidence": 0.9 },
      { "name": "AWS", "category": "platform", "confidence": 0.85 }
    ],
    "warnings": []
  }
}
```

**Response (AI failure):**
```json
{
  "ok": false,
  "error": {
    "code": "AI_ERROR",
    "message": "AI derivation timed out after 60000ms"
  }
}
```

### 4.2 Endpoint Implementation

```typescript
router.post('/api/job-descriptions/:id/extract-skills', async (req, res) => {
  const { id } = req.params

  // 1. Fetch the JD
  const jd = await jobDescriptionRepository.findById(id)
  if (!jd) {
    return res.status(404).json({
      ok: false,
      error: { code: 'NOT_FOUND', message: 'Job description not found' },
    })
  }

  // 2. Validate raw_text exists
  if (!jd.raw_text || jd.raw_text.trim().length === 0) {
    return res.status(400).json({
      ok: false,
      error: { code: 'VALIDATION_ERROR', message: 'Job description has no text to extract skills from' },
    })
  }

  // 3. Render prompt
  const prompt = renderJDSkillExtractionPrompt(jd.raw_text)

  // 4. Invoke Claude CLI
  const result = await invokeClaude({ prompt })

  // 5. Handle AI errors
  if (!result.ok) {
    // Log the failed attempt
    await promptLogRepository.create({
      prompt_input: prompt,
      prompt_template: JD_SKILL_EXTRACTION_TEMPLATE_VERSION,
      // NOTE: If `rawResponse` is undefined, store empty string '' instead of null
      // to satisfy the NOT NULL constraint on `prompt_logs.raw_response`.
      raw_response: result.rawResponse ?? '',
      status: 'error',
      error_message: result.message,
      entity_type: 'job_description',
      entity_id: id,
    })

    return res.status(502).json({
      ok: false,
      error: { code: 'AI_ERROR', message: result.message },
    })
  }

  // 6. Validate response
  const validated = validateSkillExtraction(result.data)
  if (!validated.ok) {
    await promptLogRepository.create({
      prompt_input: prompt,
      prompt_template: JD_SKILL_EXTRACTION_TEMPLATE_VERSION,
      raw_response: result.rawResponse,
      status: 'error',
      error_message: `Validation failed: ${validated.error}`,
      entity_type: 'job_description',
      entity_id: id,
    })

    return res.status(502).json({
      ok: false,
      error: { code: 'AI_ERROR', message: `Invalid AI response: ${validated.error}` },
    })
  }

  // 7. Log successful extraction
  await promptLogRepository.create({
    prompt_input: prompt,
    prompt_template: JD_SKILL_EXTRACTION_TEMPLATE_VERSION,
    raw_response: result.rawResponse,
    status: 'success',
    entity_type: 'job_description',
    entity_id: id,
  })

  // 8. Return extracted skills (NOT persisted — user must accept individually)
  return res.status(200).json({
    ok: true,
    data: {
      skills: validated.data.skills,
      warnings: validated.warnings,
    },
  })
})
```

### 4.3 Prompt Logging

Every extraction (success or failure) is logged in the `prompt_logs` table with:
- `prompt_template`: `'jd-skill-extraction-v1'`
- `entity_type`: `'job_description'`
- `entity_id`: the JD's ID
- `prompt_input`: the rendered prompt
- `raw_response`: the raw Claude CLI output
- `status`: `'success'` or `'error'`
- `error_message`: error details (if applicable)

---

## 5. SDK Changes

### 5.1 JobDescriptionsResource

Add to `packages/sdk/src/resources/job-descriptions.ts`:

```typescript
/** Extract skills from JD text using AI. Returns suggested skills for review. */
extractSkills(jdId: string): Promise<Result<SkillExtractionResult>> {
  return this.request<SkillExtractionResult>(
    'POST',
    `/api/job-descriptions/${jdId}/extract-skills`,
  )
}
```

### 5.2 SDK Types

Add to `packages/sdk/src/types.ts`:

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

---

## 6. UI: Skill Extraction Review

### 6.1 "Extract Skills" Button

Add an "Extract Skills" button to the JD skill tagging section in `JDEditor.svelte` (or `JDSkillPicker.svelte`), next to the existing skill search input. The button is only visible in edit mode (a JD must be saved first).

```
Required Skills:
[Python ×] [AWS ×]
[Search or add skill... ▾]  [Extract Skills]
```

### 6.2 Extraction Flow

1. User clicks "Extract Skills"
2. Button shows a loading spinner and becomes disabled ("Extracting...")
3. Frontend calls `forge.jobDescriptions.extractSkills(jdId)`
4. On success: the review panel appears below the button
5. On error: show error toast with the AI error message

### 6.3 Review Panel

```
┌─ Extracted Skills ──────────────────────────────────────────────┐
│                                                                  │
│  ✓ Python          language    ██████████ 0.90   [Accept] [×]   │
│  ✓ AWS             platform    █████████░ 0.85   [Accept] [×]   │
│  ✓ Kubernetes      platform    █████████░ 0.85   [Accept] [×]   │
│    Terraform       tool        ██████░░░░ 0.60   [Accept] [×]   │
│    CISSP           cert        █████░░░░░ 0.50   [Accept] [×]   │
│                                                                  │
│  Already linked: Python, AWS (skipped)                           │
│                                                                  │
│  [Accept All Remaining]                          [Dismiss All]   │
└──────────────────────────────────────────────────────────────────┘
```

### 6.4 Review Panel Behavior

**Display:**
- Each extracted skill shows: name, category badge, confidence bar (visual), confidence score (numeric)
- Skills already linked to the JD (from the existing `jdSkills` state) are marked with a checkmark and cannot be accepted again. They appear at the top with muted styling and a "Already linked" label.
- Remaining skills are sorted by confidence (highest first)

**Actions per skill:**
- **Accept:** Calls `POST /api/job-descriptions/:id/skills { name: skillName }`. On success, the skill moves to the "already linked" section and appears as a tag in the skill picker above. If the skill does not exist in the `skills` table, it is created automatically (using the existing `capitalizeFirst` + case-insensitive dedup logic from the POST endpoint).
- **Reject (x):** Removes the skill from the suggestions list. No API call — purely client-side dismissal.
- **Edit:** Clicking the skill name makes it editable (inline text input). User can rename the skill before accepting. This handles cases where the AI normalizes incorrectly (e.g., "K8s" instead of "Kubernetes"). After editing, Accept uses the edited name.

**Bulk actions:**
- **Accept All Remaining:** Accepts all non-dismissed, non-linked skills in sequence. Calls `POST /api/job-descriptions/:id/skills { name }` for each. Shows a progress indicator.
- **Dismiss All:** Clears all remaining suggestions.

**State lifecycle:**
- The review panel state is ephemeral (component state only)
- Navigating away from the JD or selecting a different JD clears the suggestions
- Re-clicking "Extract Skills" replaces any existing suggestions with a new extraction

### 6.5 New Skill Auto-Creation

When the user accepts a skill that does not exist in the `skills` table:
1. The `POST /api/job-descriptions/:id/skills { name }` endpoint creates the skill (this is existing behavior from Spec E1)
2. `capitalizeFirst` is applied to the name: `raw.charAt(0).toUpperCase() + raw.slice(1)`
3. Case-insensitive dedup: if "python" is suggested and "Python" already exists, the existing skill is linked (not a duplicate created)
4. The AI response name is used as-is for the POST call. The server handles normalization.

> **Note:** When creating a new skill via POST, include the AI-suggested category: `{ name: skill.name, category: skill.category }`. This ensures auto-created skills are well-categorized.

### 6.6 State Management

```typescript
// In the skill extraction section (JDEditor or a sub-component)
let extractedSkills = $state<ExtractedSkill[]>([])
let extracting = $state(false)
let showExtractionPanel = $state(false)

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
)

// Derived: skills already linked (from extraction results)
let alreadyLinkedExtracted = $derived(
  extractedSkills.filter(s =>
    jdSkills.some(js =>
      js.name.toLowerCase() === s.name.toLowerCase()
    )
  )
)

async function handleExtract() {
  extracting = true
  try {
    const result = await forge.jobDescriptions.extractSkills(jdId)
    if (result.ok) {
      extractedSkills = result.data.skills
      acceptedNames = new Set()
      dismissedNames = new Set()
      editedNames = new Map()
      showExtractionPanel = true
    } else {
      showErrorToast(result.error.message)
    }
  } finally {
    extracting = false
  }
}

async function handleAccept(skill: ExtractedSkill) {
  const nameToUse = editedNames.get(skill.name) ?? skill.name
  const result = await forge.jobDescriptions.addSkill(jdId, { name: nameToUse })
  if (result.ok) {
    acceptedNames = new Set([...acceptedNames, skill.name])
    // Refresh jdSkills to show the newly linked skill as a tag
    await loadJDSkills(jdId)
  }
}

async function handleAcceptAll() {
  for (const skill of reviewableSkills) {
    await handleAccept(skill)
  }
}

function handleDismiss(skill: ExtractedSkill) {
  dismissedNames = new Set([...dismissedNames, skill.name])
}

function handleDismissAll() {
  for (const skill of reviewableSkills) {
    dismissedNames = new Set([...dismissedNames, skill.name])
  }
  showExtractionPanel = false
}
```

---

## 7. Component Architecture

### 7.1 New Components

| Component | File | Purpose |
|-----------|------|---------|
| `JDSkillExtraction.svelte` | `packages/webui/src/lib/components/jd/JDSkillExtraction.svelte` | "Extract Skills" button + review panel |
| `ExtractedSkillCard.svelte` | `packages/webui/src/lib/components/jd/ExtractedSkillCard.svelte` | Individual extracted skill row with accept/reject/edit |
| `ConfidenceBar.svelte` | `packages/webui/src/lib/components/ConfidenceBar.svelte` | Visual confidence score bar (reusable) |

### 7.2 Component Props

```typescript
// JDSkillExtraction.svelte
let { jdId, jdSkills, forge, onSkillsChanged }: {
  jdId: string
  jdSkills: Skill[]           // currently linked skills (for dedup)
  forge: ForgeClient
  onSkillsChanged: () => void // callback to refresh parent's skill list
} = $props()

// ExtractedSkillCard.svelte
let { skill, isLinked, onaccept, ondismiss, onedit }: {
  skill: ExtractedSkill
  isLinked: boolean
  onaccept: () => void
  ondismiss: () => void
  onedit: (newName: string) => void
} = $props()

// ConfidenceBar.svelte
let { value, max }: {
  value: number   // 0.0 to 1.0
  max?: number    // default 1.0
} = $props()
```

---

## 8. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/jd/JDSkillExtraction.svelte` | "Extract Skills" button + review panel |
| `packages/webui/src/lib/components/jd/ExtractedSkillCard.svelte` | Individual extracted skill row |
| `packages/webui/src/lib/components/ConfidenceBar.svelte` | Visual confidence bar |

## 9. Files to Modify

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

---

## 10. Testing

### 10.1 Prompt Tests

- `renderJDSkillExtractionPrompt()` returns a string containing the JD text
- `renderJDSkillExtractionPrompt()` includes instructions for JSON output format
- `JD_SKILL_EXTRACTION_TEMPLATE_VERSION` is `'jd-skill-extraction-v1'`

### 10.2 Validator Tests

- Valid response with skills array passes validation
- Empty skills array passes validation with a warning
- Missing `skills` field fails validation
- `skills` as non-array fails validation
- Skill with missing `name` fails validation
- Skill with empty `name` fails validation
- Skill with missing `category` fails validation
- Skill with missing `confidence` fails validation
- Skill with `confidence` outside [0, 1] passes with warning (clamped)
- Skill with unknown `category` passes with warning
- Extra fields at root or item level produce warnings (not errors)
- Response with extra root fields passes with warning

### 10.3 API Tests

- `POST /api/job-descriptions/:id/extract-skills` for nonexistent JD returns 404
- `POST /api/job-descriptions/:id/extract-skills` for JD with empty `raw_text` returns 400 VALIDATION_ERROR
- `POST /api/job-descriptions/:id/extract-skills` with valid JD returns 200 with skills array (requires Claude CLI to be available — integration test)
- Prompt log is created on successful extraction with `prompt_template = 'jd-skill-extraction-v1'`
- Prompt log is created on failed extraction with `status = 'error'`
- Response includes `warnings` array (may be empty)

### 10.4 SDK Tests

- `forge.jobDescriptions.extractSkills(jdId)` returns `{ data: SkillExtractionResult }`
- `forge.jobDescriptions.extractSkills(jdId)` for nonexistent JD returns error

### 10.5 UI Component Tests

- "Extract Skills" button is visible in JD edit mode
- "Extract Skills" button is NOT visible in JD create mode
- Clicking "Extract Skills" shows loading state ("Extracting...")
- On successful extraction, review panel appears with extracted skills
- Skills already linked to JD show as "Already linked" with muted styling
- Remaining skills are sorted by confidence (highest first)
- Clicking "Accept" on a skill calls `POST /api/job-descriptions/:id/skills { name }`
- Accepted skill appears as a tag in the skill picker
- Clicking "x" (dismiss) removes the skill from the suggestions
- Clicking a skill name makes it editable (inline text input)
- Editing a skill name and clicking Accept uses the edited name
- "Accept All Remaining" accepts all non-dismissed, non-linked skills
- "Dismiss All" clears all remaining suggestions
- Re-clicking "Extract Skills" replaces previous suggestions
- Confidence bar displays correctly for values between 0 and 1
- Category badge shows the skill category
- Error toast shown when AI extraction fails
- Loading spinner shown during extraction

### 10.6 Integration Tests

- Extract skills from a JD with known requirements. Accept one. Verify it appears in `GET /api/job-descriptions/:id/skills`.
- Extract skills from a JD. Accept a skill that does not exist in the `skills` table. Verify the skill is created with `capitalizeFirst` applied.
- Extract skills. Accept "python" when "Python" already exists. Verify no duplicate skill is created (case-insensitive dedup).
- Extract skills. Dismiss all. Verify no skills are linked to the JD.
- Extract skills. Navigate away. Come back. Verify suggestions are gone (ephemeral state).

---

## 11. Acceptance Criteria

1. "Extract Skills" button appears in the JD editor skill section (edit mode only)
2. Clicking the button invokes Claude CLI with the JD's `raw_text` and returns extracted skills
3. The AI prompt requests JSON output with `{ name, category, confidence }` per skill
4. AI response is validated by `validateSkillExtraction()` before being returned to the frontend
5. Extracted skills appear in a review panel sorted by confidence (highest first)
6. Skills already linked to the JD are identified and shown as "Already linked"
7. Each extracted skill can be individually accepted (links to JD), dismissed (removed from suggestions), or edited (rename before accepting)
8. Accepting a skill calls the existing `POST /api/job-descriptions/:id/skills { name }` endpoint
9. New skills are auto-created on accept with `capitalizeFirst` and case-insensitive dedup (existing POST behavior)
10. "Accept All Remaining" bulk-accepts all non-dismissed, non-linked skills
11. "Dismiss All" clears all remaining suggestions
12. Every extraction attempt (success or failure) is logged in `prompt_logs` with template version `jd-skill-extraction-v1`
13. AI errors (timeout, parse failure, process error, not found) show a user-friendly error toast
14. Extraction state is ephemeral — navigating away clears suggestions
15. The extraction endpoint returns suggestions only; no skills are auto-persisted
16. SDK `JobDescriptionsResource` has `extractSkills(jdId)` method
17. Confidence bar visually represents the 0-1 confidence score
18. Empty extraction result (no skills found) is handled gracefully with a message
