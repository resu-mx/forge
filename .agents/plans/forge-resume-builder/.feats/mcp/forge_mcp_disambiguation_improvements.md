# MCP tool responses need relational context to prevent AI agent confusion
**Type**: feature-request
**Component**: forge-mcp
**Filed**: 2026-04-08
**Status**: open

## Description

MCP tool responses lack sufficient relational context, causing AI agents to misattribute entities and make incorrect assumptions. This has been observed across multiple sessions.

## Observed Issues

### 1. Source -> Organization ambiguity
When `forge_search_sources` returns a role like "DevOps Engineer III", the response doesn't include which organization it belongs to. In a previous session, an agent attributed this role to Buoyant (not in the org table) instead of Greymatter.io.

**Fix**: Always include `organization.name` (via JOIN on `source_roles.organization_id`) in source search results for role-type sources.

### 2. Project naming ambiguity
Project sources like "arustydev/agents (components)" were confused with the unrelated "superpowers" skill system in the Claude Code configuration. Without a URL or clear description in the search result, agents infer meaning from the name alone.

**Fix**: Include `source_projects.url` in search results so the GitHub URL disambiguates the project identity.

### 3. Bullet -> Source traceability in search results
`forge_search_bullets` results include `source_content_snapshot` but not the source title or source_id from `bullet_sources`. When searching bullets broadly (not filtered by source_id), there's no way to determine which source a bullet belongs to without N+1 lookups.

**Fix**: Include `sources[].title` and `sources[].source_type` in bullet search results. `forge_get_bullet` already returns this via the sources relation — the search endpoint should match.

### 4. General pattern
All list/search endpoints should return enough relational context (org name, source title, source type) to prevent the need for N+1 lookups that waste token budget and increase error probability. The cost of including a few extra fields per row is far lower than the cost of an agent making an incorrect assumption.

## Recommended Changes

| Endpoint | Add to response |
|----------|----------------|
| `forge_search_sources` (role) | `organization_name`, `work_arrangement` |
| `forge_search_sources` (project) | `url`, `is_personal`, `open_source` |
| `forge_search_sources` (education) | `organization_name`, `education_type` |
| `forge_search_bullets` | `sources[].{id, title, source_type}` |
| `forge_search_perspectives` | `source_title` (via bullet → bullet_sources → source) |
