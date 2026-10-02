# API Response Envelope

All HTTP responses from `@forge/core` follow a consistent envelope format.

## Success — Single Entity

```json
{
  "data": { "id": "uuid", "title": "...", ... }
}
```

HTTP status: `200` (GET, PATCH) or `201` (POST)

## Success — List (Paginated)

```json
{
  "data": [
    { "id": "uuid", "title": "...", ... },
    { "id": "uuid", "title": "...", ... }
  ],
  "pagination": {
    "total": 42,
    "offset": 0,
    "limit": 50
  }
}
```

HTTP status: `200`

## Success — No Content

No response body.

HTTP status: `204` (DELETE)

## Error

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Cannot approve bullet: status is 'draft', expected 'pending_review'",
    "details": {
      "field": "status",
      "current": "draft",
      "expected": "pending_review"
    }
  }
}
```

HTTP status: `400`, `404`, `409`, `501`, `504`, or `500`

## Headers

All responses include:
- `Content-Type: application/json` (except 204)
- `X-Request-Id: <uuid>` — for log correlation
