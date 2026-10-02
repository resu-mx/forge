# forge_search_sources type filter broken + pagination type mismatch
**Type**: bug
**Component**: forge-mcp
**Filed**: 2026-04-08
**Status**: open

## Description

`forge_search_sources` has two compounding bugs:

### 1. Type filter does not work
When called with `type=project`, it returns **all** source types (education, presentation, role, AND project) mixed together instead of filtering to only project sources.

### 2. offset/limit parameters fail with type mismatch
Both `offset` and `limit` parameters fail with:
```
Invalid input: expected number, received string
```
This prevents pagination entirely.

### Combined effect
Only the first 20 of 50 sources are visible, with no way to page through. In our case, only 5 of 28 project sources were discoverable, causing us to miss 23 project sources and all their linked bullets.

## Impact
- Users/agents can't discover all sources of a given type
- Missing sources → missing bullets → incomplete resume building
- Direct DB queries required as workaround

## Fix
1. Ensure `type` filter is applied in the SQL WHERE clause (verify the query builder respects this param)
2. Fix `offset`/`limit` parameter types in the Zod schema — likely needs `z.coerce.number()` instead of `z.number()`

## Feature Request: Semantic search
Consider adding vector similarity search to source search. Currently even text filtering is absent — adding semantic search would allow finding sources by description similarity, which is valuable when source titles are ambiguous (e.g., "Goose" vs "UntitledGooseTool").
