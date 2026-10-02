# Phase 90: Industries & Role Types Entities

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None
**Blocks:** Phase 91 (Summaries Rework)
**Parallelizable with:** Phase 89 (Skills Expansion)
**Duration:** Short-Medium (5 tasks)

## Goal

Create `industries` and `role_types` as lightweight runtime-addable entity tables (like domains). Migrate the free-text `organizations.industry` column to reference the new `industries` table. These entities support structured grouping/filtering of summaries, organizations, and JDs.

## Non-Goals

- Summaries integration (Phase 91)
- JD integration (future)
- Predefined seed data beyond initial migration from existing free-text values

---

## Tasks

### T90.1: Write Migration 032 — Industries & Role Types

**File:** `packages/core/src/db/migrations/032_industries_role_types.sql`

**Steps:**
1. Create `industries` table (id, name UNIQUE, description, created_at)
2. Create `role_types` table (id, name UNIQUE, description, created_at)
3. Seed industries from distinct `organizations.industry` values (deduplicated, trimmed)
4. Add `industry_id` FK column to `organizations` (nullable, references industries)
5. Populate `industry_id` from matching industry names
6. Consider keeping `organizations.industry` text as deprecated (or drop if confident in migration)

**Acceptance Criteria:**
- [ ] Both tables created with UNIQUE name constraints
- [ ] Existing org industry values migrated to `industries` table
- [ ] `organizations.industry_id` FK populated correctly
- [ ] No data loss

### T90.2: Types, Repositories, Services

**Steps:**
1. Add `Industry` and `RoleType` interfaces (id, name, description, created_at)
2. Add `CreateIndustry`, `UpdateIndustry`, `CreateRoleType`, `UpdateRoleType` inputs
3. Create `industry-repository.ts` and `role-type-repository.ts` (CRUD + findByName for combobox)
4. Create thin service layers
5. Wire into Services interface

**Acceptance Criteria:**
- [ ] Full CRUD for both entities
- [ ] `findByName()` supports combobox pattern (select existing or create new)

### T90.3: API Routes & SDK

**Steps:**
1. Create `/api/industries` routes (GET list, POST create, PATCH update, DELETE)
2. Create `/api/role-types` routes (same pattern)
3. Create SDK `IndustryResource` and `RoleTypeResource`
4. Wire into server and client

**Acceptance Criteria:**
- [ ] Both APIs functional
- [ ] SDK exposes both resources

### T90.4: Update Organization UI

**Steps:**
1. Replace free-text `industry` field with `IndustryCombobox` (select existing or create new — same pattern as OrgCombobox)
2. Add validation: if org has tag `university` or `school` (education primary type), hide/disable industry field
3. Update org cards to show industry from linked entity

**Acceptance Criteria:**
- [ ] Industry is a combobox, not free text
- [ ] Education orgs cannot have industry (per bug fix requirement)
- [ ] Org cards display industry name

### T90.5: Tests

**Acceptance Criteria:**
- [ ] CRUD tests for both entities
- [ ] Migration test: existing industries migrated correctly
- [ ] Org UI: education org industry validation
