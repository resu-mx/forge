# Phase 89: Skills Expansion & Technology Absorption

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Spec:** `refs/specs/2026-04-05-qualifications-credentials-certifications.md` (related — skills taxonomy)
**Depends on:** None
**Blocks:** Phase 91 (Summaries Rework), Phase 92 (Tagline Engine)
**Parallelizable with:** Phase 90 (Industries & Role Types)
**Duration:** Medium (7 tasks)

## Goal

Expand the `skills` entity with a structured `category` enum and domain linkage via a new `skill_domains` junction table. Absorb the free-text `bullet_technologies` system into Skills so all technical labels participate in JD alignment and skill matching. Drop the `bullet_technologies` table.

## Non-Goals

- Changing the JD skill extraction AI (it already targets skills)
- Changing the embedding/alignment algorithm
- UI redesign of skill pages (just adding category/domain fields)
- Renaming existing skills

## Context

Technologies are currently free-text strings on a `bullet_technologies` junction table, separate from the curated `skills` entity. This means technologies don't participate in JD alignment, can't be linked to JDs, and have no category or domain context. Consolidating them into Skills gives a single unified taxonomy.

---

## Tasks

### T89.1: Write Migration 031 — Skills Expansion

**File:** `packages/core/src/db/migrations/031_skills_expansion.sql`

**Steps:**
1. Rebuild `skills` table to add `category` column with CHECK constraint:
   ```
   CHECK (category IN ('language', 'framework', 'platform', 'tool', 'library',
     'methodology', 'protocol', 'concept', 'soft_skill', 'other'))
   ```
   Default: `'other'`
2. Create `skill_domains` junction table (skill_id FK, domain_id FK, composite PK)
3. For each unique `bullet_technologies.technology` string:
   - Match to existing skill by `lower(name) = lower(technology)`
   - If no match: INSERT new skill with `category = 'other'`
4. Convert `bullet_technologies` rows to `bullet_skills` rows (using the skill IDs from step 3)
5. Drop `bullet_technologies` table
6. Index: `idx_skill_domains_domain` on `skill_domains(domain_id)`

**Acceptance Criteria:**
- [ ] Skills table has `category` column with CHECK constraint
- [ ] `skill_domains` junction exists
- [ ] All technologies migrated to skills (no data loss)
- [ ] `bullet_technologies` table dropped
- [ ] No duplicate skill names created
- [ ] Existing `bullet_skills` rows preserved (no duplicates from technology merge)

### T89.2: Update Types

**File:** `packages/core/src/types/index.ts`

**Steps:**
1. Add `SkillCategory` type union
2. Add `category: SkillCategory` to `Skill` interface (with `'other'` default)
3. Add `SkillWithDomains` interface extending `Skill` with `domains: Domain[]`
4. Add `category` to `CreateSkill` and `UpdateSkill`
5. Remove `BulletTechnology` type if it exists
6. Remove technologies from `Bullet` interface (replace with skills coverage)

**Acceptance Criteria:**
- [ ] All new types exported
- [ ] No references to `bullet_technologies` or `BulletTechnology`
- [ ] TypeScript compiles cleanly

### T89.3: Update Skill Repository

**File:** `packages/core/src/db/repositories/skill-repository.ts`

**Steps:**
1. Add `category` to create/update/find queries
2. Add `findByCategory(category)` method
3. Add domain junction management: `addDomain()`, `removeDomain()`, `getDomains()`
4. Add `findAllWithDomains()` and `findByIdWithDomains()` methods
5. Filter support: `findAll({ category?, domain_id? })`

**Acceptance Criteria:**
- [ ] Category CRUD works
- [ ] Domain junction CRUD works
- [ ] Filtering by category and domain works

### T89.4: Update Bullet Repository

**File:** `packages/core/src/db/repositories/bullet-repository.ts`

**Steps:**
1. Remove all `bullet_technologies` queries (addTechnology, removeTechnology, getTechnologies)
2. Remove technology-related JOIN logic from bullet queries
3. Ensure `bullet_skills` covers what technologies used to provide
4. Update any methods that returned technologies separately

**Acceptance Criteria:**
- [ ] No references to `bullet_technologies` table
- [ ] Bullet skills queries cover former technology use cases
- [ ] All existing bullet tests updated

### T89.5: Update Services & Routes

**Steps:**
1. Update skill service: add category validation, domain management
2. Update skill routes: add category filter param, domain endpoints
3. Update bullet service: remove technology methods
4. Update bullet routes: remove technology endpoints
5. Update SDK types and resources to match

**Acceptance Criteria:**
- [ ] Skill API supports category filter and domain management
- [ ] Bullet API no longer has technology endpoints
- [ ] SDK mirrors all changes

### T89.6: Update WebUI

**Steps:**
1. Skills page: add category dropdown, domain multi-select
2. Bullet views: remove separate "Technologies" section, show all as skills
3. MCP tools: update `forge_update_bullet` — remove technologies param, document that skills covers this now

**Acceptance Criteria:**
- [ ] Skills page shows category and domain fields
- [ ] No "Technologies" section on bullet cards
- [ ] MCP tools updated

### T89.7: Tests

**Steps:**
1. Migration tests: technology → skill migration, no data loss, no duplicates
2. Repository tests: category filtering, domain junction CRUD
3. Integration tests: bullet with former technologies now shows skills

**Acceptance Criteria:**
- [ ] All existing tests pass (no regressions)
- [ ] New tests cover category, domain, and technology absorption
