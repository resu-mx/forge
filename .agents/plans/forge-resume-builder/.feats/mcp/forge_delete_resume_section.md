# Feature: forge_delete_resume_section

## Problem

No MCP tool exists to delete a resume section. When a resume has redundant or incorrect sections (e.g., a `clearance` section when clearance is in the header, or a `freeform` summary section when summaries use `summary_id`), there's no way to remove them via MCP. Requires direct DB access.

## Discovered

2026-04-08 while building the Federal Sales Engineer resume. The resume had pre-existing `clearance` and `freeform` summary sections from the Federal Resume template that needed removal.

## Proposed Tool

```ts
forge_delete_resume_section({
  resume_id: string,  // Resume UUID
  section_id: string, // Section UUID to delete
})
```

Should cascade-delete any entries in the section. Should refuse to delete if the section contains entries unless `force: true` is passed.
