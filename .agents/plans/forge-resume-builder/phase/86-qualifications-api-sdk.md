# Phase 86: Qualifications — API Routes & SDK

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Spec:** `refs/specs/2026-04-05-qualifications-credentials-certifications.md` (Sections 5, 6)
**Depends on:** Phase 85 (services must exist for routes to call)
**Blocks:** Phase 87 (WebUI needs SDK)
**Parallelizable with:** T86.1 ∥ T86.2 (routes); T86.3 ∥ T86.4 (SDK); T86.5 after all
**Duration:** Short-Medium (5 tasks: T86.1 through T86.5)

## Goal

Create HTTP route handlers for credentials and certifications CRUD, create SDK resource classes, wire routes into the server and SDK into the client. Full API test coverage.

## Non-Goals

- WebUI (Phase 87)
- IR compiler or MCP (Phase 88)

## Files to Create

| File | Description |
|------|-------------|
| `packages/core/src/routes/credentials.ts` | Credential CRUD routes |
| `packages/core/src/routes/certifications.ts` | Certification CRUD + skill junction routes |
| `packages/sdk/src/resources/credentials.ts` | CredentialResource class |
| `packages/sdk/src/resources/certifications.ts` | CertificationResource class |
| `packages/core/src/routes/__tests__/credentials.test.ts` | Route tests |
| `packages/core/src/routes/__tests__/certifications.test.ts` | Route tests |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/routes/server.ts` | Mount credential and certification routes |
| `packages/core/src/routes/sources.ts` | Remove clearance-specific route logic if any |
| `packages/sdk/src/client.ts` | Add CredentialResource and CertificationResource properties |
| `packages/sdk/src/index.ts` | Export new resources and types |
| `packages/sdk/src/types.ts` | Mirror core types (credential + certification types) |

---

## Tasks

### T86.1: Create Credential Routes

**File:** `packages/core/src/routes/credentials.ts`
**Goal:** CRUD routes for credentials.

**Routes:**
| Method | Path | Handler |
|--------|------|---------|
| GET | `/api/credentials` | List all credentials |
| GET | `/api/credentials/:id` | Get by ID |
| POST | `/api/credentials` | Create |
| PATCH | `/api/credentials/:id` | Update |
| DELETE | `/api/credentials/:id` | Delete |

**Steps:**
1. Create Hono route group
2. Implement each handler calling through to `services.credentials`
3. POST/PATCH validate request body
4. Return standard JSON responses with appropriate status codes (201 on create, 200 on get/update, 204 on delete)
5. Handle 404 for missing credentials

**Acceptance Criteria:**
- [ ] All 5 routes implemented
- [ ] Proper status codes and error responses
- [ ] Request body validation

### T86.2: Create Certification Routes

**File:** `packages/core/src/routes/certifications.ts`
**Goal:** CRUD routes for certifications + skill junction management.

**Routes:**
| Method | Path | Handler |
|--------|------|---------|
| GET | `/api/certifications` | List all (with skills) |
| GET | `/api/certifications/:id` | Get by ID (with skills) |
| POST | `/api/certifications` | Create |
| PATCH | `/api/certifications/:id` | Update |
| DELETE | `/api/certifications/:id` | Delete |
| POST | `/api/certifications/:id/skills` | Add skill |
| DELETE | `/api/certifications/:id/skills/:skillId` | Remove skill |

**Steps:**
1. Create Hono route group
2. CRUD handlers calling through to `services.certifications`
3. Skill junction endpoints: POST accepts `{ skill_id }` body, DELETE uses URL params
4. GET endpoints return `CertificationWithSkills` (skills array populated)

**Acceptance Criteria:**
- [ ] All 7 routes implemented
- [ ] Skill add/remove returns updated certification with skills
- [ ] Proper cascade behavior (deleting cert removes skill links)

### T86.3: Create SDK CredentialResource

**File:** `packages/sdk/src/resources/credentials.ts`
**Goal:** SDK client for credential API.

**Methods:**
- `list(): Promise<Result<Credential[]>>`
- `get(id: string): Promise<Result<Credential>>`
- `create(input: CreateCredential): Promise<Result<Credential>>`
- `update(id: string, input: UpdateCredential): Promise<Result<Credential>>`
- `delete(id: string): Promise<Result<void>>`

**Acceptance Criteria:**
- [ ] All methods implemented following existing SDK resource patterns
- [ ] Return types match core types

### T86.4: Create SDK CertificationResource

**File:** `packages/sdk/src/resources/certifications.ts`
**Goal:** SDK client for certification API.

**Methods:**
- `list(): Promise<Result<CertificationWithSkills[]>>`
- `get(id: string): Promise<Result<CertificationWithSkills>>`
- `create(input: CreateCertification): Promise<Result<Certification>>`
- `update(id: string, input: UpdateCertification): Promise<Result<Certification>>`
- `delete(id: string): Promise<Result<void>>`
- `addSkill(certId: string, skillId: string): Promise<Result<CertificationWithSkills>>`
- `removeSkill(certId: string, skillId: string): Promise<Result<CertificationWithSkills>>`

**Acceptance Criteria:**
- [ ] All methods implemented
- [ ] list/get return WithSkills variant

### T86.5: Wire Routes and SDK, Mirror Types

**Goal:** Integrate everything into server and client.

**Steps:**
1. `packages/core/src/routes/server.ts`: Mount credential and certification routes
2. `packages/core/src/routes/sources.ts`: Remove any clearance-specific handling (e.g., the `source.source_type === 'clearance'` branch in route logic)
3. `packages/sdk/src/types.ts`: Mirror all credential and certification types from core. Remove `SourceClearance` and clearance from `SourceType`.
4. `packages/sdk/src/client.ts`: Add `credentials: CredentialResource` and `certifications: CertificationResource` properties
5. `packages/sdk/src/index.ts`: Export new resources and types
6. Run full test suite (core + SDK)

**Acceptance Criteria:**
- [ ] Routes accessible at `/api/credentials` and `/api/certifications`
- [ ] SDK types in sync with core types
- [ ] SDK client exposes both resources
- [ ] All existing tests still pass (no regressions from clearance removal)
