# forge_create_skill: Return existing skill inventory for dedup

## Problem

`forge_create_skill` creates duplicates because the MCP client has no visibility into what skills already exist. The client has to guess, get a "Conflict: Skill X already exists" error, then look up the ID separately.

## Proposed Fix

Apply the same prompt-return pattern used in `forge_extract_jd_skills`:

1. **Option A (minimal):** When `forge_create_skill` gets a conflict, return the existing skill instead of an error. Idempotent create-or-get.

2. **Option B (richer):** Add a `forge_search_skills` tool (or ensure it exists) that the MCP client can call first. The `forge_create_skill` description should instruct the client to search before creating.

Option A is simpler and matches the `getOrCreate` pattern already in `SkillService`.

## Acceptance Criteria

- `forge_create_skill("Python", "language")` when Python exists returns `{ id: "820c4f0c...", name: "Python", ... }` instead of an error
- No duplicate skills created
- MCP tool description updated to note idempotent behavior
