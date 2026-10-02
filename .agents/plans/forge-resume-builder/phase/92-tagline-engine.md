# Phase 92: Tagline Engine (IDF/IVF Keyword Relevance)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** Phase 89 (Skills — unified taxonomy for keyword matching)
**Blocks:** Nothing
**Parallelizable with:** Phase 91 (after Phase 89 completes)
**Duration:** Medium (5 tasks)

## Goal

Replace the static tagline field with an auto-generated tagline per resume using IDF/IVF keyword relevance scoring against linked JDs. Format: `<target-role> -- <keyword> + <keyword> + <keyword>`. The tagline is stored on the resume's header and regenerated when JD links change. Users can override the generated tagline.

## Non-Goals

- Full NLP / LLM-based tagline generation (this is programmatic keyword extraction)
- Tagline templates or styles
- Tagline A/B testing

## Context

Currently taglines are manual text fields on summaries. The new model extracts keywords from linked JDs' `raw_text` using TF-IDF scoring, matches them against the user's skills, and selects the top-K most relevant keywords for the tagline. When a JD is linked/unlinked from a resume, the tagline is regenerated from ALL linked JDs and the user is prompted to accept or keep their override.

---

## Tasks

### T92.1: IDF/IVF Keyword Extraction Service

**File:** `packages/core/src/services/tagline-service.ts`

**Steps:**
1. Implement TF-IDF extraction from JD `raw_text`:
   - Tokenize, remove stop words, stem/normalize
   - Compute term frequency per JD
   - Compute inverse document frequency across all JDs in the system
   - Return ranked keyword list with scores
2. Match extracted keywords against user's skills (by name, case-insensitive)
3. Score: keywords that match existing skills rank higher than unmatched terms
4. Return top-K keywords (configurable, default 3-5)
5. Format: `<JD title or role> -- <keyword1> + <keyword2> + <keyword3>`

**Acceptance Criteria:**
- [ ] TF-IDF extraction produces ranked keywords from JD text
- [ ] Skill matching boosts relevant keywords
- [ ] Output format matches spec
- [ ] Handles multiple linked JDs (union of keywords, re-ranked)

### T92.2: Resume Tagline Generation & Storage

**Steps:**
1. Add `generated_tagline` and `tagline_override` fields to resume header (or separate columns)
2. When JD is linked to resume: regenerate tagline from all linked JDs
3. When JD is unlinked: regenerate from remaining JDs
4. If user has `tagline_override`: prompt before replacing (via API response flag)
5. Store both generated and override — UI shows override if present, generated otherwise

**Acceptance Criteria:**
- [ ] Tagline auto-generates on JD link/unlink
- [ ] Override preserved, user prompted before replacement
- [ ] Both generated and override stored

### T92.3: Integration with JD Linkage

**Steps:**
1. Hook into `forge_link_resume_to_jd` / `forge_unlink_resume_from_jd` flows
2. After link/unlink, call tagline service to regenerate
3. Return regeneration result (new tagline + flag if override exists)
4. MCP tool response includes the new generated tagline for user review

**Acceptance Criteria:**
- [ ] Link/unlink triggers tagline regeneration
- [ ] Response indicates whether override was present

### T92.4: WebUI — Tagline Display & Override

**Steps:**
1. Resume editor shows tagline (generated or override) in header section
2. "Regenerate" button to force re-run
3. Edit capability to set override
4. "Reset to generated" button to clear override
5. When JD link changes and override exists: toast notification asking user to review

**Acceptance Criteria:**
- [ ] Tagline visible in resume header editor
- [ ] Override/reset/regenerate all functional
- [ ] Toast notification on JD link change with existing override

### T92.5: Tests

**Acceptance Criteria:**
- [ ] TF-IDF extraction produces sensible keywords from sample JD text
- [ ] Multi-JD keyword ranking works (union + re-rank)
- [ ] Skill matching boost works
- [ ] Link/unlink lifecycle triggers regeneration
- [ ] Override preservation works
