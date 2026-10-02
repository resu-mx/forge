# Pagination Strategy

## Query Parameters

All list endpoints accept:

| Parameter | Type | Default | Max | Description |
|---|---|---|---|---|
| `offset` | integer | 0 | — | Number of records to skip |
| `limit` | integer | 50 | 200 | Number of records to return |

## Response Shape

```json
{
  "data": [...],
  "pagination": {
    "total": 142,
    "offset": 50,
    "limit": 50
  }
}
```

- `total` — total number of records matching the filter (before pagination)
- `offset` — the offset used in this request
- `limit` — the limit used in this request

## Implementation

```sql
-- Count query (for total)
SELECT COUNT(*) FROM sources WHERE status = ?;

-- Data query (for page)
SELECT * FROM sources WHERE status = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;
```

Two queries per list request. The count query is cheap on indexed columns.

## Default Sort

All list endpoints sort by `created_at DESC` (newest first) unless a sort parameter is specified. MVP does not support custom sort — this is a post-MVP enhancement.

## Edge Cases

- `offset >= total` → returns empty `data: []` with correct `total`
- `limit > 200` → clamped to 200
- `limit < 1` → clamped to 1
- Negative values → 400 validation error
