# Update Source Description

`forge_update_source` — Update a source's title, description, and metadata fields.

## Problem
No MCP tool exists to update source descriptions. This session enriched 15 project source descriptions from user interviews, but all updates had to go through raw SQL. The existing `forge_update_bullet`, `forge_update_perspective`, `forge_update_summary` tools exist for other entities but sources are missing.

## Proposed Interface
```
{
  source_id: string,
  title?: string,
  description?: string,
  source_type?: string,
  start_date?: string | null,
  end_date?: string | null,
  notes?: string | null
}
```

Returns updated Source object.
