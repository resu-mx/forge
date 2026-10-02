# forge_add_resume_skill: Missing section_id in MCP tool schema

## Problem

`forge_add_resume_skill` requires `section_id` but the MCP tool's published schema only exposes `resume_id` and `skill_id`. The tool fails with:
```
Invalid input: expected string, received undefined (path: section_id)
```

The underlying API route is `POST /api/resumes/:id/sections/:section_id/skills` — it needs both the resume ID and the skills section ID.

## Root Cause

The MCP tool registration doesn't include `section_id` as a parameter in the zod schema, so the MCP client never sends it.

## Fix

Add `section_id` as a required parameter to the `forge_add_resume_skill` tool schema:
```ts
section_id: z.string().uuid().describe('Skills section UUID (from forge://resume/{id} resource)')
```

## Files

- `packages/mcp/src/tools/tier3-assembly.ts` (or wherever forge_add_resume_skill is registered)
