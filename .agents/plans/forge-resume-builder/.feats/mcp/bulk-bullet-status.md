# Bulk Bullet Status Transitions

`forge_bulk_approve_bullets`, `forge_bulk_reject_bullets` — Transition multiple bullets at once.

## Problem
After creating 15 project bullets this session, each one needs individual approval. During the earlier dedup session, 47 bullets were rejected one at a time. Bulk transitions would save significant time during audit workflows.

## Proposed Interface
```
{
  bullet_ids: string[],
  rejection_reason?: string   // Required for bulk reject
}
```

Returns: `{ transitioned: number, skipped: number, errors: [{ id, reason }] }`

Skips bullets already in the target status. Errors on invalid transitions (e.g., approving a rejected bullet without reopening first).
