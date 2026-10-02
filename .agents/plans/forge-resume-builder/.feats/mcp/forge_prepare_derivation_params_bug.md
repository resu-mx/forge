# forge_prepare_derivation: params object serialization bug

## Problem

When calling `forge_prepare_derivation` with `params: {archetype, domain, framing}`, the MCP tool's zod schema defines `params` with optional sub-fields:

```ts
params: z.object({
  archetype: z.string().optional(),
  domain: z.string().optional(),
  framing: z.string().optional(),
}).optional()
```

The MCP client serializes `params` as a JSON string instead of an object, causing:
```
Invalid input: expected object, received string
```

## Root Cause

The zod schema for `params` sub-fields uses `.optional()` on each field. The MCP SDK may be stringifying the nested object during transport. Additionally, the fields should NOT be optional — they are required for bullet→perspective derivations.

## Fix

1. Change the schema to make sub-fields required (not optional) since they're always needed for bullet derivations
2. Test with the MCP inspector to verify the object is received correctly
3. Consider whether the `params` object should be flattened into top-level fields (`archetype`, `domain`, `framing`) to avoid nested object serialization issues

## Additional Issue

The first failed prepare attempt creates a lock in `pending_derivations` that blocks retries. When the jq parse error occurred (prompt text has control characters/newlines), the derivation_id couldn't be extracted, leaving orphaned locks. The 2-minute timeout eventually clears them, but an immediate retry fails with CONFLICT.

**Mitigation:** Consider adding a `force` flag or detecting same-client re-prepare and replacing the lock.

## Files

- `packages/mcp/src/tools/derive.ts` — zod schema definition
- `packages/core/src/services/derivation-service.ts` — lock conflict handling
