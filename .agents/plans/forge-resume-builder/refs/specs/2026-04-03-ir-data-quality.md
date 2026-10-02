# IR Data Quality

**Date:** 2026-04-03
**Spec:** F1 (IR Data Quality)
**Phase:** TBD
**Builds on:** Migration 005 (`user_profile`), Migration 010 (`education_org_fk`), Migration 013 (`org_campuses`)
**Related:** [Education Sub-Type Fields](2026-04-03-education-subtype-fields.md), [Resume Renderer](2026-03-29-resume-renderer-and-entity-updates.md)

## Overview

The IR compiler (`compileResumeIR`) produces a `ResumeDocument` that feeds into the LaTeX template and Markdown renderer. Several fields are not populating correctly, producing output that diverges from the target formatting (see `zettelkasten/proj/job-hunting/applications/marriott/director-ml-devops/resume.tex`). Specifically:

1. **Header name** shows "User" instead of the actual profile name (seed migration defaulted to "User" when no resume header JSON existed)
2. **Organization name** shows "Other" for experience entries when the `source_roles.organization_id` JOIN returns NULL (org not linked, or link missing)
3. **No location** on the organization line in experience sections (the reference resume uses `{Org Name - Location (Contract)}{}` format)
4. **Missing contact info** in the header (email, phone, LinkedIn, GitHub not flowing from `user_profile`)
5. **Skills** not populating correctly when `resume_skills` entries lack proper `skill_id` foreign key linkage

The root cause is a combination of: incomplete `user_profile` seed data, missing JOINs in the experience and education queries, and the "Other" fallback masking NULL org references.

## Scope

Fix `compileResumeIR`, `buildExperienceItems`, `buildEducationItems`, and `parseHeader` to:

- Read profile data (name, email, phone, location, links) from `user_profile` table and flow all fields into the `ResumeHeader`
- JOIN `organizations` correctly in experience queries so `org_name` resolves to the actual organization name
- JOIN `org_campuses` for location data on experience and education entries
- Build experience org lines in the format `{Org Name - Location (Work Arrangement)}` matching the reference resume
- Ensure education entries resolve institution names via `organization_id` JOIN (not the deprecated `institution` column)
- Validate that all IR fields match what the LaTeX template's `renderHeader`, `renderExperienceSection`, and `renderEducationSection` expect

## Technical Approach

### 1. Header population (`parseHeader`)

The current `parseHeader` function already reads from `user_profile` via the `profile` parameter. The issue is that the `user_profile` row may have been seeded with defaults ("User" for name, NULL for everything else) if the original resume had no `header` JSON blob.

**Fix:** No code change needed in `parseHeader` itself -- the function correctly uses `profile?.name ?? resume.name`. The fix is ensuring the `user_profile` table has real data. Add a check in the profile service or a CLI seed command. For the compiler, add a warning log when `profile?.name` is "User" to surface the issue.

However, there is a subtle bug: if `profile` is NULL (no row in `user_profile`), the fallback chain goes `profile?.name ?? resume.name`, which falls through to `resume.name`. But `resume.name` is the resume title (e.g., "Marriott Director Resume"), not the user's name. This needs to be documented and the fallback should use `resume.name` only when it looks like a person name. In practice, the fix is to ensure `user_profile` is always populated.

### 2. Experience org names (`buildExperienceItems`)

Current query JOINs `source_roles sr` then `organizations o ON o.id = sr.organization_id`. The fallback in the grouping logic is:

```typescript
const orgKey = row.org_name ?? 'Other'
```

**Fixes:**

a. The JOIN is correct but needs a location component. Extend the query to also JOIN `org_campuses` for location:

```sql
LEFT JOIN source_roles sr ON sr.source_id = s.id
LEFT JOIN organizations o ON o.id = sr.organization_id
LEFT JOIN org_campuses oc ON oc.organization_id = o.id AND oc.is_headquarters = 1
```

b. Add `oc.city`, `oc.state`, `sr.work_arrangement` to the SELECT. Note: `work_arrangement` already exists in the `source_roles` DB table (migration 002). No schema change needed — only add it to the compiler's SELECT query and row type.

c. Build the organization display string to match the reference format:

```
Cisco - Remote (Contract)
Raytheon Intelligence & Space - Arlington, VA (Remote)
United States Air Force Reserve - National Capitol Region
```

Format: `{org_name}{ - location}{ (work_arrangement)}`

d. Update `ExperienceEntryRow` to include `org_city`, `org_state`, `work_arrangement`.

e. Change the grouping key from just `org_name` to include location context, or better, group by `organization_id` to avoid name collision issues, then build the display string from the first entry in each group. Use `organization_id ?? org_name ?? 'Other'` as the grouping key, falling back through org ID → org name → default.

### 3. Education institution names (`buildEducationItems`)

The current query uses `COALESCE(o.name, se.institution) AS institution`. This is correct but relies on `se.organization_id` being populated. For legacy data where only `se.institution` was set (deprecated column), the COALESCE handles it.

**Fix:** Also JOIN `org_campuses` for location data:

```sql
LEFT JOIN org_campuses oc ON oc.id = se.campus_id
```

Confirm `source_education.campus_id` exists (added by migration 013). If missing, this spec has an unlisted dependency on migration 013.

Add `oc.city`, `oc.state` to the SELECT and use them to populate `location` on the `EducationItem` when `se.location` is NULL.

### 4. Skills flow (`buildSkillItems`)

The current query joins `resume_skills rs` to `skills s` via `rs.skill_id`. Verify the data integrity: if `resume_skills` entries exist but point to nonexistent `skill_id` values, the INNER JOIN silently drops them.

**Fix:** Change to LEFT JOIN and log a warning for orphaned `resume_skills` entries. Also ensure the skills query respects category ordering for consistent output.

### 5. Experience organization display format

Update `ExperienceGroup` type or the template rendering to support the compound org line format. Two approaches:

**Approach A (IR-side):** Build the display string in the compiler and store it in `ExperienceGroup.organization`:
```typescript
organization: buildOrgDisplayString(orgName, city, state, workArrangement)
// "Raytheon Intelligence & Space - Arlington, VA (Remote)"
```

**Approach B (Template-side):** Add `location` and `work_arrangement` fields to `ExperienceGroup` and let the template compose the string.

**Decision:** Approach A. The organization display string is a presentation concern but is consistent across all templates. Building it in the compiler keeps the template simpler and matches how the reference resume formats it.

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/services/resume-compiler.ts` | Extend experience query with campus/work_arrangement JOINs; build org display strings; extend education query with campus JOIN; add logging for data quality issues |
| `packages/core/src/types/index.ts` | No changes needed (ExperienceGroup.organization is already a string) |

## Files to Create

None.

## Testing Approach

### Unit tests (`packages/core/src/services/__tests__/resume-compiler.test.ts`)

1. **Header from profile:** Seed `user_profile` with name/email/phone/links, compile resume, verify header fields match profile
2. **Header fallback:** No profile row, verify fallback to resume name
3. **Experience org name:** Seed source with `source_roles.organization_id` pointing to org, compile, verify `ExperienceGroup.organization` is the org name (not "Other")
4. **Experience org with location:** Seed org with HQ campus, verify org line includes location
5. **Experience org with work arrangement:** Seed source_role with `work_arrangement = 'Remote'`, verify format `{Org} - {Location} (Remote)`
6. **Education institution via org:** Seed education source with `organization_id`, verify institution name comes from org JOIN
7. **Education location from campus:** Seed education source with `campus_id`, verify location comes from campus
8. **Skills integrity:** Seed `resume_skills` with valid and orphaned entries, verify valid entries appear and orphaned are skipped gracefully
9. **Distinct orgs with same name:** Two different organizations with the same name produce separate experience groups (verifies grouping uses `organization_id`, not `org_name`)

### Integration tests

1. Full compile of a resume with all section types, verify LaTeX output matches expected format
2. Compile resume with missing profile data, verify graceful degradation (no crashes, sensible defaults)

### Manual validation

- Compile a real resume and diff the LaTeX output against the Marriott reference resume to verify structural parity

## Acceptance Criteria

1. Header name shows the user's actual name from `user_profile`, not "User"
2. Experience organization lines show actual org names from the `organizations` table, not "Other"
3. Experience organization lines include location when available (from `org_campuses` HQ or org `location` field)
4. Experience organization lines include work arrangement when available (e.g., "Remote", "Contract")
5. Header contact fields (email, phone, LinkedIn, GitHub, website) flow from `user_profile`
6. Education institution names resolve via `organization_id` JOIN when available
7. Education locations populate from `org_campuses` when `campus_id` is set
8. Skills section renders all properly-linked `resume_skills` entries grouped by category
9. No "Other" appears in compiled output when organization data exists in the database
10. Existing resumes with no org/campus data continue to compile without errors (graceful NULL handling)

## Non-Goals

- Template changes (the sb2nov template macros are correct; this spec fixes the data flowing into them)
- New section types
- Changes to the `user_profile` schema or migration
- Automatic profile population from external sources
- Changes to the Markdown renderer (it uses the same IR, so fixes here propagate)
- UI changes to the resume editor
