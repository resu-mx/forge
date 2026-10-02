# Phase 88: Qualifications — IR Compiler & MCP Tools

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Spec:** `refs/specs/2026-04-05-qualifications-credentials-certifications.md` (Sections 8, 9, 10)
**Depends on:** Phase 86 (SDK resources for MCP tools), Phase 85 (services for IR compiler)
**Blocks:** Nothing
**Parallelizable with:** Phase 87 (WebUI) — no shared files
**Duration:** Short-Medium (4 tasks: T88.1 through T88.4)

## Goal

Update the resume IR compiler to pull clearance data from the `credentials` table instead of sources, add certifications section support, and register MCP tools for credential and certification CRUD.

## Non-Goals

- JD matching algorithm changes (deferred — data model supports it, algorithm update is separate)
- Certification skill corroboration scoring (deferred)
- Credential-aware gap analysis (deferred)

## Files to Create

| File | Description |
|------|-------------|
| `packages/mcp/src/tools/tier2-qualifications.ts` | MCP tools for credential and certification CRUD |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/services/resume-compiler.ts` | Read clearance from credentials table; add certifications section |
| `packages/mcp/src/server.ts` | Register qualifications tools |
| `packages/mcp/src/resources.ts` | Add credentials and certifications resources |
| `packages/mcp/src/utils/feature-flags.ts` | Add qualifications feature flag if needed |

---

## Tasks

### T88.1: Update IR Compiler — Clearance Section

**File:** `packages/core/src/services/resume-compiler.ts`
**Goal:** Read clearance data from `credentials` table instead of sources.

**Steps:**
1. Find where the compiler builds the clearance resume section (currently reads from sources with `source_type = 'clearance'`)
2. Replace with a query to `services.credentials.findByType('clearance')`
3. Map `Credential` → `ClearanceItem` for the IR:
   - `content`: build display string from details (e.g., "Top Secret / SCI, Full Scope Polygraph — Active")
   - `entry_id`: null (credentials don't have resume entries in the derivation chain)
   - `source_id`: null (no longer a source)
4. Keep the `ClearanceItem` IR interface unchanged — downstream renderers (Markdown, LaTeX, PDF) don't need changes

**Acceptance Criteria:**
- [ ] Clearance section renders from credentials table
- [ ] No references to `source_type = 'clearance'` in compiler
- [ ] Existing resume output unchanged (same rendered content, different data source)

### T88.2: Update IR Compiler — Certifications Section

**File:** `packages/core/src/services/resume-compiler.ts`
**Goal:** Add compiler support for certifications resume section.

**Steps:**
1. Add handler for `section.entry_type === 'certifications'` in the compiler's section switch
2. Query `services.certifications.findAll()` to get all certifications
3. Map each `Certification` to a section item:
   - Primary line: `"name — issuer (year)"` (e.g., "AWS Solutions Architect Professional — Amazon Web Services (2024)")
   - Credential ID line (if present): `"Credential ID: ABC-123"`
4. Return as IR section items (flat list, no bullets)
5. Ensure the `ResumeSection = 'certifications'` type is already handled (it should be from migration 004)

**Acceptance Criteria:**
- [ ] Certifications section renders when included in resume
- [ ] Name, issuer, date formatted correctly
- [ ] Optional credential ID displayed when present
- [ ] Empty certifications section handled gracefully

### T88.3: Register MCP Tools

**File:** `packages/mcp/src/tools/tier2-qualifications.ts`
**Goal:** Add MCP tools for credential and certification management.

**Tools:**
| Tool | Description |
|------|-------------|
| `forge_search_credentials` | List/filter credentials by type |
| `forge_create_credential` | Create a credential with type-specific details |
| `forge_update_credential` | Update credential fields |
| `forge_search_certifications` | List certifications with skills |
| `forge_create_certification` | Create a certification |
| `forge_update_certification` | Update certification fields |
| `forge_add_certification_skill` | Link a skill to a certification |
| `forge_remove_certification_skill` | Unlink a skill from a certification |

**Steps:**
1. Create tool file following existing tier2 patterns (use `registerTool` helper)
2. All tools take Zod-validated parameters
3. Wire into `packages/mcp/src/server.ts` registration sequence
4. Add `forge://credentials` and `forge://certifications` resources to `resources.ts`

**Acceptance Criteria:**
- [ ] All 8 tools registered and functional
- [ ] 2 resources registered
- [ ] Proper error mapping for not-found and validation errors
- [ ] Tools follow existing naming/parameter conventions

### T88.4: Integration Tests

**Goal:** Verify the full vertical slice works end-to-end.

**Tests:**
1. IR compiler: resume with clearance section renders credential data (not source data)
2. IR compiler: resume with certifications section renders cert data
3. MCP tools: create credential via MCP, verify via API
4. MCP tools: create certification with skill link via MCP, verify skills populated
5. Round-trip: create credential in UI → appears in resume IR → appears in MCP resource

**Acceptance Criteria:**
- [ ] Full stack works: DB → service → route → SDK → MCP
- [ ] Resume rendering unchanged for clearance (same output, different source)
- [ ] New certifications section renders correctly
