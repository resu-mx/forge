# Phase 93: Profile Rework

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None
**Blocks:** Nothing (salary → Phase 66 Bullet Graph enhancement is a soft dep)
**Parallelizable with:** Most phases
**Duration:** Medium (5 tasks)

## Goal

Rework the user profile: structured name (first, middle initial, last), image upload, location as address (reuse campus pattern), website KV map (replacing individual github/website fields), salary expectations (min/target/stretch with multi-format input and auto-conversion), and phone hint fix.

## Non-Goals

- Multi-user profiles
- Profile photo cropping/editing
- Address validation API
- Salary negotiation features

---

## Tasks

### T93.1: Schema — Profile Expansion

**File:** `packages/core/src/db/migrations/034_profile_rework.sql`

**Steps:**
1. Rebuild `user_profile` to restructure fields:
   - Replace `name` TEXT with `first_name`, `middle_initial`, `last_name` (derive display name)
   - Replace `location` TEXT with `address_line1`, `address_line2`, `city`, `state`, `zipcode`, `country`
   - Replace `linkedin`, `github`, `website` with `websites` JSON (KV map: `{"github": "url", "linkedin": "url", "portfolio": "url", "blog": "url", ...}`)
   - Add `image_path` TEXT (nullable — local file path or base64)
   - Add `salary_min` INTEGER, `salary_target` INTEGER, `salary_stretch` INTEGER (stored as annual cents)
   - Add `salary_input_format` TEXT CHECK ('hourly', 'biweekly', 'monthly', 'yearly') DEFAULT 'yearly'
2. Migrate existing data: split `name` into first/last, move linkedin/github/website into JSON, preserve location as address_line1
3. Remove old `clearance` column if not already removed by Phase 84

**Acceptance Criteria:**
- [ ] Structured name fields populated from existing name
- [ ] Address fields available
- [ ] Websites JSON contains migrated links
- [ ] Salary fields exist with correct types
- [ ] No data loss

### T93.2: Update Types & Repository

**Steps:**
1. Update `UserProfile` interface with new fields
2. Add `displayName` computed helper (first + middle + last)
3. Add salary conversion utilities:
   - `toAnnual(amount, format)` / `fromAnnual(annual, format)`
   - Hourly: annual / 2080; Biweekly: annual / 26; Monthly: annual / 12
4. Websites stored as `Record<string, string>` with JSON serialization
5. Update profile repository for new column structure

**Acceptance Criteria:**
- [ ] All new fields in types
- [ ] Salary conversion utilities tested
- [ ] Display name helper works

### T93.3: Update API, SDK, IR Compiler

**Steps:**
1. Update profile routes for new fields
2. Update SDK ProfileResource
3. Update IR compiler: use structured name, address, websites map for resume header
4. IR compiler builds display name from parts

**Acceptance Criteria:**
- [ ] API accepts/returns new fields
- [ ] Resume header renders structured name correctly
- [ ] Websites render as links in resume

### T93.4: Profile Page UI Rework

**Steps:**
1. Name section: First Name, Middle Initial (single char), Last Name fields
2. Image: upload button, preview thumbnail, clear button. Store as base64 or file path.
3. Phone: fix hint to `+1 (123) 555-1234`
4. Location: address fields (line1, line2, city, state, zip, country) — reuse campus address pattern
5. Websites: KV editor — list of label+URL rows. Preset hints for GitHub, LinkedIn, Portfolio, Blog. User can add custom entries. Remove dedicated GitHub/LinkedIn/Website fields.
6. Salary Expectations: three fields (Min, Target, Stretch) with format selector (Hourly/Biweekly/Monthly/Yearly). Auto-calculate display of other formats below inputs.

**Acceptance Criteria:**
- [ ] Structured name fields
- [ ] Image upload and preview
- [ ] Phone hint corrected
- [ ] Address fields present
- [ ] Website KV editor with preset hints
- [ ] Salary inputs with format conversion display

### T93.5: Tests

**Acceptance Criteria:**
- [ ] Migration: name split, website migration, address fields
- [ ] Salary conversion: hourly↔yearly↔biweekly↔monthly roundtrip
- [ ] Profile API: CRUD with new fields
- [ ] IR compiler: structured name in resume header
