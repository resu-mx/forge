# Phase 84: Qualifications — Schema & Data Migration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Spec:** `refs/specs/2026-04-05-qualifications-credentials-certifications.md` (Sections 1, 11)
**Depends on:** None (schema-only, no upstream phase dependencies)
**Blocks:** Phase 85 (core layer), Phase 86 (API + SDK), Phase 87 (WebUI)
**Parallelizable with:** Nothing — migration is a sequential bottleneck
**Duration:** Short (5 tasks: T84.1 through T84.5)

## Goal

Create the `credentials` and `certifications` tables, migrate existing clearance data from `source_clearances` → `credentials`, remove `source_type = 'clearance'` from the sources CHECK constraint, remove `user_profile.clearance` column, and update `note_references` to accept the new entity types. This is migration 030.

## Non-Goals

- Repository, service, or route code (Phase 85-86)
- UI changes (Phase 87)
- IR compiler changes (Phase 88)
- MCP tool registration (Phase 88)
- Certification skill corroboration in JD matching (deferred)

## Context

Security clearance is currently modeled as `source_type = 'clearance'` with a `source_clearances` extension table. This migration removes clearance from the Source derivation chain and introduces two new entities: credentials (boolean qualifiers) and certifications (skill-validating earned credentials). The clearance data migrates from the source system to the new credentials table.

This is a complex migration with multiple table rebuilds (sources, user_profile, note_references) and a data migration. The `PRAGMA foreign_keys = OFF` pattern from migration 019/024 applies.

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/migrations/030_qualifications.sql` | credentials + certifications + certification_skills tables, data migration, source/profile/note_references rebuilds |
| `packages/core/src/db/__tests__/migration-030.test.ts` | Migration-specific tests: data migration integrity, CHECK constraints, orphan cleanup |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/db/migrate.ts` | If migration 030 uses table rebuilds, ensure it's in the PRAGMA-outside-transaction list |

---

## Tasks

### T84.1: Write Migration 030 — Create New Tables

**File:** `packages/core/src/db/migrations/030_qualifications.sql`
**Goal:** Create the `credentials`, `certifications`, and `certification_skills` tables.

**Steps:**
1. Create `credentials` table with all columns from spec section 1.1 (id, credential_type, label, status, organization_id FK, details JSON, issued_date, expiry_date, timestamps)
2. Create `certifications` table from spec section 1.2 (id, name, issuer, date_earned, expiry_date, credential_id, credential_url, education_source_id FK, timestamps)
3. Create `certification_skills` junction table from spec section 1.3 (certification_id + skill_id composite PK, CASCADE deletes)
4. Add indices: `idx_credentials_type`, `idx_credentials_org`, `idx_certifications_source`

**Acceptance Criteria:**
- [ ] All three tables created with correct CHECK constraints
- [ ] FKs reference correct parent tables with appropriate ON DELETE behavior
- [ ] Indices created for query-path columns

### T84.2: Write Migration 030 — Data Migration

**Goal:** Migrate existing clearance data from `source_clearances` → `credentials`.

**Steps:**
1. INSERT INTO credentials from source_clearances JOIN sources, using the migration SQL from spec section 1.4
2. Handle `clearance_access_programs` junction table — aggregate into JSON array in `details`
3. Use `sponsor_organization_id` → `organization_id` mapping
4. Derive `label` from `level` enum (e.g., `top_secret` → "Top Secret")
5. Map `s.start_date` → `issued_date`
6. Check for bullets referencing clearance sources (`bullet_sources` junction) — log warning if found
7. Delete orphaned clearance source rows after data migration

**Acceptance Criteria:**
- [ ] All source_clearances rows migrated to credentials with correct details JSON
- [ ] Access programs aggregated correctly
- [ ] Sponsor org linkage preserved
- [ ] Orphaned clearance sources cleaned up
- [ ] No data loss for any clearance-related information

### T84.3: Write Migration 030 — Table Rebuilds

**Goal:** Remove clearance from sources CHECK, remove clearance from user_profile, update note_references.

**Steps:**
1. Set `PRAGMA foreign_keys = OFF` (outside transaction — use the migrate.ts pattern)
2. Rebuild `sources` table: remove `'clearance'` from `source_type` CHECK constraint → `CHECK (source_type IN ('role', 'project', 'education', 'general'))`
3. Rebuild `user_profile`: drop the `clearance` column entirely
4. Rebuild `note_references`: add `'credential'` and `'certification'` to `entity_type` CHECK constraint
5. Drop `clearance_access_programs` table
6. Drop `source_clearances` table
7. Re-enable foreign keys and verify integrity with `PRAGMA foreign_key_check`

**Acceptance Criteria:**
- [ ] `source_type = 'clearance'` no longer valid (INSERT rejected)
- [ ] `user_profile` has no `clearance` column
- [ ] `note_references` accepts `'credential'` and `'certification'` entity types
- [ ] `source_clearances` and `clearance_access_programs` tables no longer exist
- [ ] All existing data preserved through rebuilds
- [ ] Foreign key integrity check passes

### T84.4: Register Migration in PRAGMA List

**File:** `packages/core/src/db/migrate.ts`
**Goal:** Ensure migration 030 runs with `PRAGMA foreign_keys = OFF` outside the transaction.

**Steps:**
1. Check if `migrate.ts` has a list of migrations requiring the PRAGMA-outside-transaction pattern
2. Add `030_qualifications` to that list (it has table rebuilds like 019 and 024)

**Acceptance Criteria:**
- [ ] Migration runner applies PRAGMA correctly for migration 030
- [ ] Migration runs cleanly on a fresh database
- [ ] Migration runs cleanly on a database with existing clearance data

### T84.5: Write Migration Tests

**File:** `packages/core/src/db/__tests__/migration-030.test.ts`
**Goal:** Verify the migration handles all edge cases.

**Tests:**
1. Fresh database: migration creates all three tables, no data migration needed (no source_clearances rows)
2. Database with clearance data: source_clearances rows migrate to credentials correctly
3. Access programs junction: multiple programs per clearance aggregate into JSON array
4. Sponsor org linkage: organization_id preserved from source_clearances
5. Source type CHECK: inserting `source_type = 'clearance'` fails after migration
6. Profile clearance: `user_profile` no longer has `clearance` column
7. Note references: can insert `entity_type = 'credential'` and `'certification'`
8. Orphan cleanup: clearance source rows deleted, no dangling bullet_sources
9. Certification skills: junction table allows valid inserts, rejects invalid FKs
10. Rollback safety: if migration fails mid-way, database is not left in corrupt state

**Acceptance Criteria:**
- [ ] All 10 test scenarios pass
- [ ] Tests use real database (not mocks — per project testing convention)
