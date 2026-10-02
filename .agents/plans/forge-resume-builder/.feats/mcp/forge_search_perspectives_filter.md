# forge_search_perspectives search parameter does not filter results
**Type**: bug
**Component**: forge-mcp
**Filed**: 2026-04-08
**Status**: open

## Description

The `search` parameter on `forge_search_perspectives` has no effect. Regardless of search terms provided, the same paginated result set is returned.

Tested with multiple highly specific strings:
- `"Terragrunt Terraform Helm GitLab CI/CD cloud forensics toolkit"`
- `"forensic investigations threat hunting multi-cloud Azure CloudTrail GCP"`
- `"140 unit integration tests Kubernetes configuration drift"`

All returned the identical first-page results (sorted by created_at, unfiltered).

## Impact
- No way to find a specific perspective by content via MCP tools
- Blocks the normal workflow: find bullet → find its perspective → add to resume
- Forces direct DB queries as workaround
- Same bug likely affects `forge_search_summaries` search parameter

## Fix
Implement filtering on the `content` column when `search` param is provided. Options:
1. **LIKE-based**: `WHERE content LIKE '%' || ? || '%'` (simple, adequate for exact term matching)
2. **FTS5**: Create a virtual table for full-text search on perspectives.content (better for multi-word queries)
3. **Add bullet_id filter**: Even without text search, being able to filter perspectives by `bullet_id` would solve the most common lookup pattern

## Feature Request: Semantic search
Consider implementing vector similarity search for perspectives and bullets. This would allow finding related content even when exact terms don't match — especially valuable when looking for perspectives derived from a bullet (where wording often differs significantly from the original).

The `embeddings` table already exists in the schema, suggesting infrastructure for this may already be partially in place.
