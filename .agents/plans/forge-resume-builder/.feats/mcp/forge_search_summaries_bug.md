# forge_search_summaries: limit parameter type mismatch

## Problem

Calling `forge_search_summaries` with `limit: 20` (number) fails with:
```
Invalid input: expected number, received string
```

The MCP transport is serializing the number as a string, but the zod schema expects a number. This is likely the same serialization issue affecting other tools with numeric parameters.

## Root Cause

The MCP SDK may be stringifying all parameters during JSON-RPC transport. The zod schema uses `z.number()` but receives `"20"` (string) instead of `20` (number).

## Fix

Either:
1. Use `z.coerce.number()` instead of `z.number()` in the schema to accept string-encoded numbers
2. Or add `.transform(Number)` to numeric params
3. Check if this affects other tools with numeric limit/offset params

## Files

- `packages/mcp/src/tools/search.ts` (or wherever forge_search_summaries is registered)
