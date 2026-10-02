# forge_search_bullets: Add source_type filter

## Problem

When building resumes, we need to filter bullets by their source type (role vs project vs education) to place them in the correct resume section. Currently `forge_search_bullets` only filters by domain, status, source_id, and text search — no way to say "show me only role-sourced bullets" or "only project-sourced bullets."

This led to project-sourced bullets (Sanskrit) being added to Professional Experience instead of Selected Projects.

## Proposed Changes

### 1. Add `source_type` filter to `forge_search_bullets`

New optional parameter: `source_type: "role" | "project" | "education" | "general" | "presentation"`

Implementation: JOIN through `bullet_sources` and `sources` tables:
```sql
SELECT DISTINCT b.* FROM bullets b
JOIN bullet_sources bs ON bs.bullet_id = b.id
JOIN sources s ON bs.source_id = s.id
WHERE s.source_type = ?
```

### 2. Consider: source_type-aware `forge_add_resume_entry`

When adding a perspective to a resume section, the tool could check:
- The perspective's parent bullet's primary source type
- The target section's entry_type
- If mismatched (e.g., project bullet → experience section), return a warning in the response (not an error — the user may have a reason)

This is lower priority than the search filter.

## Files

- `packages/mcp/src/tools/search.ts` — add source_type param to forge_search_bullets
- `packages/core/src/db/repositories/bullet-repository.ts` — add source_type filter to list/search
- `packages/mcp/src/tools/tier3-assembly.ts` — optional warning in forge_add_resume_entry
