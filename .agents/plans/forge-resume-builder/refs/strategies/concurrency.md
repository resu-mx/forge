# Concurrency Strategy

## Problem

AI derivation takes 5-60 seconds. Without concurrency control:
- Two simultaneous `derive-bullets` on the same source creates duplicate bullet sets
- The UI may show stale state during derivation
- Status transitions can race

## Solution: Deriving Lock

### Sources

When `POST /sources/:id/derive-bullets` is called:
1. Check source status — if `deriving`, return 409 Conflict
2. Set source status to `deriving` (atomic UPDATE with WHERE status != 'deriving')
3. Spawn Claude Code CLI process
4. On success: create bullets, reset source status, set `last_derived_at`
5. On failure: reset source status, return error

```sql
-- Atomic lock acquisition
UPDATE sources SET status = 'deriving'
WHERE id = ? AND status != 'deriving'
RETURNING *;
-- If 0 rows affected → already deriving → return 409
```

### Bullets

Same pattern for `POST /bullets/:id/derive-perspectives`:
- Bullets don't have a `deriving` status in their enum (draft/pending_review/approved/rejected)
- Instead, use a `deriving_perspectives` boolean column, or check for in-flight prompt_logs
- Simpler approach for MVP: track derivation state in application memory (a Set of bullet IDs currently being derived). This works because MVP is single-process.

### SQLite Concurrency

SQLite in WAL mode supports concurrent reads with a single writer. Since Forge is single-user, single-process, write contention is minimal. The `deriving` lock is primarily about preventing duplicate AI calls, not database-level locking.

### Timeout Recovery

If the process crashes during derivation, the source remains in `deriving` status. On server startup:
```sql
-- Reset stale deriving locks (server crashed during derivation)
UPDATE sources SET status = 'approved'
WHERE status = 'deriving'
AND last_derived_at < datetime('now', '-5 minutes');
```

This cleanup runs once on startup. The 5-minute threshold ensures we don't accidentally reset a legitimate in-progress derivation.
