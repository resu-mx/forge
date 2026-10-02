# Phase 85: Qualifications — Core Layer (Types, Repositories, Services)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Spec:** `refs/specs/2026-04-05-qualifications-credentials-certifications.md` (Sections 2, 3, 4)
**Depends on:** Phase 84 (migration must exist for repository tests)
**Blocks:** Phase 86 (API + SDK)
**Parallelizable with:** T85.1 (types) must go first; T85.2 ∥ T85.3 (repos); T85.4 ∥ T85.5 (services) after repos
**Duration:** Medium (7 tasks: T85.1 through T85.7)

## Goal

Add TypeScript type definitions for credentials and certifications, create CRUD repositories for both entities, create service layers with validation, and clean up clearance-related types from the Source model. All code compiles, all tests pass.

## Non-Goals

- HTTP routes or SDK (Phase 86)
- WebUI (Phase 87)
- IR compiler or MCP (Phase 88)

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/db/repositories/credential-repository.ts` | Credential CRUD + type filtering |
| `packages/core/src/db/repositories/certification-repository.ts` | Certification CRUD + skill junction management |
| `packages/core/src/services/credential-service.ts` | Validation layer over credential repository |
| `packages/core/src/services/certification-service.ts` | Validation layer over certification repository |
| `packages/core/src/db/repositories/__tests__/credential-repository.test.ts` | Repository tests |
| `packages/core/src/db/repositories/__tests__/certification-repository.test.ts` | Repository tests |
| `packages/core/src/services/__tests__/credential-service.test.ts` | Service tests |
| `packages/core/src/services/__tests__/certification-service.test.ts` | Service tests |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | Add credential/certification types; remove SourceClearance, clearance from SourceType; remove clearance from UserProfile |
| `packages/core/src/services/index.ts` | Add CredentialService + CertificationService to Services interface and createServices() |
| `packages/core/src/db/repositories/source-repository.ts` | Remove clearance extension handling from create/update/findById |
| `packages/core/src/db/repositories/__tests__/source-repository.test.ts` | Remove or update clearance source tests |

---

## Tasks

### T85.1: Add Credential & Certification Types

**File:** `packages/core/src/types/index.ts`
**Goal:** Add all new types from spec section 2, remove clearance source types.

**Steps:**
1. Add types from spec section 2.1: `CredentialType`, `CredentialStatus`, `ClearanceDetails`, `DriversLicenseDetails`, `BarAdmissionDetails`, `MedicalLicenseDetails`, `CredentialDetails`, `Credential`, `CreateCredential`, `UpdateCredential`
2. Add types from spec section 2.1: `Certification`, `CreateCertification`, `UpdateCertification`, `CertificationWithSkills`
3. Update `SourceType`: remove `'clearance'` → `'role' | 'project' | 'education' | 'general'`
4. Remove `SourceClearance` interface
5. Remove `SourceClearance` from the `Source.extension` union type
6. Remove clearance extension fields from `CreateSource` and `UpdateSource` (level, polygraph, clearance_status, clearance_type, access_programs)
7. Remove `clearance: string | null` from `UserProfile` interface
8. Keep existing `ClearanceLevel`, `ClearancePolygraph`, `ClearanceStatus`, `ClearanceType`, `ClearanceAccessProgram` enums — they're now used by `ClearanceDetails`

**Acceptance Criteria:**
- [ ] All new types exported from barrel
- [ ] `SourceType` no longer includes `'clearance'`
- [ ] `SourceClearance` removed
- [ ] `UserProfile` has no `clearance` field
- [ ] Existing clearance enums retained for ClearanceDetails
- [ ] TypeScript compiles cleanly

### T85.2: Create Credential Repository

**File:** `packages/core/src/db/repositories/credential-repository.ts`
**Goal:** Standard CRUD repository for credentials with JSON details serialization.

**Methods:**
- `create(input: CreateCredential): Promise<Credential>`
- `findById(id: string): Promise<Credential | null>`
- `findAll(): Promise<Credential[]>`
- `findByType(type: CredentialType): Promise<Credential[]>`
- `update(id: string, input: UpdateCredential): Promise<Credential>`
- `delete(id: string): Promise<void>`

**Key implementation details:**
- `details` column: `JSON.stringify()` on write, `JSON.parse()` on read
- `findAll()` ordered by `credential_type, label`
- `update()` merges partial `details` with existing (read-modify-write for the JSON column)
- UUID generation via `crypto.randomUUID()`

**Tests:** CRUD, type filtering, details JSON roundtrip, partial details update, org FK linkage

**Acceptance Criteria:**
- [ ] All 6 methods implemented and tested
- [ ] JSON details serialize/deserialize correctly for all 4 credential types
- [ ] Partial details update merges correctly (doesn't clobber unmentioned fields)

### T85.3: Create Certification Repository

**File:** `packages/core/src/db/repositories/certification-repository.ts`
**Goal:** CRUD repository for certifications with skill junction management.

**Methods:**
- `create(input: CreateCertification): Promise<Certification>`
- `findById(id: string): Promise<Certification | null>`
- `findAll(): Promise<Certification[]>`
- `findByIdWithSkills(id: string): Promise<CertificationWithSkills | null>`
- `findAllWithSkills(): Promise<CertificationWithSkills[]>`
- `update(id: string, input: UpdateCertification): Promise<Certification>`
- `delete(id: string): Promise<void>`
- `addSkill(certId: string, skillId: string): Promise<void>`
- `removeSkill(certId: string, skillId: string): Promise<void>`
- `getSkills(certId: string): Promise<Skill[]>`

**Key implementation details:**
- `findAllWithSkills()` uses LEFT JOIN on `certification_skills` + `skills`, groups in application code
- `addSkill()` uses INSERT OR IGNORE to be idempotent
- `delete()` cascades via FK (certification_skills cleaned automatically)

**Tests:** CRUD, skill add/remove/get, findAllWithSkills returns skills array, education_source_id linkage, cascade delete

**Acceptance Criteria:**
- [ ] All 10 methods implemented and tested
- [ ] Skill junction operations are idempotent
- [ ] WithSkills variants return populated skills array
- [ ] Education source FK validated

### T85.4: Create Credential Service

**File:** `packages/core/src/services/credential-service.ts`
**Goal:** Thin validation layer over credential repository.

**Validation rules:**
- `credential_type` must be a valid enum value
- `details` must match the type-specific schema:
  - `clearance`: `level` required, `polygraph` optional, `clearance_type` required, `access_programs` optional (default `[]`)
  - `drivers_license`: `class` required, `state` required, `endorsements` optional (default `[]`)
  - `bar_admission`: `jurisdiction` required, `bar_number` optional
  - `medical_license`: `license_type` required, `state` required, `license_number` optional
- `organization_id` if provided must reference an existing org (validate via org repo)
- `status` must be a valid enum value

**Tests:** Validation pass/fail for each credential type, org FK validation, invalid details rejection

**Acceptance Criteria:**
- [ ] Type-specific details validation for all 4 credential types
- [ ] Invalid details rejected with clear error messages
- [ ] Org FK validated against repository

### T85.5: Create Certification Service

**File:** `packages/core/src/services/certification-service.ts`
**Goal:** Thin validation layer over certification repository.

**Validation rules:**
- `name` required, non-empty
- `education_source_id` if provided must reference a source with `source_type = 'education'`
- `skill_id` on addSkill must reference an existing skill

**Tests:** Validation pass/fail, education source type check, skill FK validation

**Acceptance Criteria:**
- [ ] Education source type validated (rejects non-education sources)
- [ ] Skill FK validated
- [ ] Name required validation

### T85.6: Clean Up Source Repository

**File:** `packages/core/src/db/repositories/source-repository.ts`
**Goal:** Remove clearance extension handling.

**Steps:**
1. Remove clearance-specific INSERT logic from `create()` (no more `source_clearances` insert)
2. Remove clearance-specific UPDATE logic from `updateExtension()`
3. Remove clearance JOIN from `findById()` and `findAll()` queries
4. Remove clearance-specific extension parsing from result mapping
5. Update any type guards or switch statements that handle `source_type = 'clearance'`

**Tests:** Update `source-repository.test.ts` — remove clearance source test cases, verify remaining types still work

**Acceptance Criteria:**
- [ ] No references to `source_clearances` table in repository code
- [ ] No `clearance` case in source type switches
- [ ] Existing role/project/education/general tests still pass

### T85.7: Wire Services into Services Interface

**File:** `packages/core/src/services/index.ts`
**Goal:** Register CredentialService and CertificationService.

**Steps:**
1. Add `credentials: CredentialService` and `certifications: CertificationService` to `Services` interface
2. Instantiate both in `createServices()` with appropriate repository injections
3. Export both services from barrel

**Acceptance Criteria:**
- [ ] Both services accessible via `services.credentials` and `services.certifications`
- [ ] Full test suite passes (core: all existing + new tests)
