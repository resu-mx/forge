# Bulk Bullet-Source Cross-Linking

`forge_bulk_link_bullet_sources` — Link multiple bullets to multiple sources in one call.

## Problem
Role-sourced bullets often also belong to project sources (e.g., a Principal bullet about the DFIR pipeline should link to both the role AND the project). This session required 43 individual INSERT statements to cross-link role bullets to their project sources. No MCP tool exists for bullet_sources management at all.

## Proposed Interface
```
{
  links: [
    { bullet_id: "...", source_id: "...", is_primary?: boolean }
  ]
}
```

Returns: `{ created: number, skipped: number }`

## Also Needed
- `forge_list_bullet_sources` — show all sources for a bullet
- `forge_remove_bullet_source` — unlink a bullet from a source
