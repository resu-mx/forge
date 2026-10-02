# Feature/Bug: forge_add_resume_entry doesn't support education entries

## Problem

`forge_add_resume_entry` requires `perspective_id`, but education entries link via `source_id` (not `perspective_id`). Education sources (degrees, certificates, courses) don't go through the bullet→perspective derivation flow — they're metadata-only records.

This means you can't add education entries to a resume's education section via MCP. Requires direct DB insert:

```sql
INSERT INTO resume_entries (id, resume_id, section_id, source_id, position, created_at, updated_at)
VALUES (uuid, resume_id, section_id, education_source_id, position, now, now);
```

## Discovered

2026-04-08 while building Federal Sales Engineer resume. Confirmed by inspecting existing education entries in other resumes — all have `source_id` set and `perspective_id` NULL.

## Options

**Option A:** Add `source_id` as an alternative parameter to `forge_add_resume_entry` (mutually exclusive with `perspective_id`)

**Option B:** Create a separate `forge_add_education_entry` tool that takes `source_id`

Option A is cleaner since it doesn't add another tool.

## Related

See also: `forge_add_resume_entry.md` (existing file about filler bullets needing perspectives)
