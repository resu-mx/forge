# SDK Result Type Contract

All SDK methods return `Promise<Result<T>>`. Never throws.

## Type Definition

```typescript
type Result<T> =
  | { ok: true; data: T }
  | { ok: false; error: ForgeError }

interface ForgeError {
  code: string        // machine-readable error code
  message: string     // human-readable description
  details?: unknown   // optional structured details
}
```

## Error Codes

| Code | HTTP Status | Description |
|---|---|---|
| `NOT_FOUND` | 404 | Entity does not exist |
| `VALIDATION_ERROR` | 400 | Invalid input or invalid status transition |
| `CONFLICT` | 409 | Entity has dependents (delete blocked) or derivation in progress |
| `GATEWAY_TIMEOUT` | 504 | AI derivation timed out (60s) |
| `AI_ERROR` | 502 | AI returned malformed or invalid output |
| `NOT_IMPLEMENTED` | 501 | Feature stubbed for post-MVP |
| `NETWORK_ERROR` | — | SDK could not reach the server |
| `UNKNOWN_ERROR` | 5xx | Unexpected server error |

## Pagination Wrapper

```typescript
type PaginatedResult<T> =
  | { ok: true; data: T[]; pagination: Pagination }
  | { ok: false; error: ForgeError }

interface Pagination {
  total: number
  offset: number
  limit: number
}
```

List methods return `Promise<PaginatedResult<T>>`.

## SDK Implementation Notes

The SDK needs two internal request helpers:

```typescript
// For single-entity endpoints (GET/:id, POST, PATCH, DELETE)
private async request<T>(method, path, body?): Promise<Result<T>>
  // Returns { ok: true, data: json.data } or { ok: false, error: json.error }

// For list endpoints (GET with pagination)
private async requestList<T>(method, path, params?): Promise<PaginatedResult<T>>
  // Returns { ok: true, data: json.data, pagination: json.pagination }
  // or { ok: false, error: json.error }
```

The review queue endpoint (`GET /review/pending`) returns a nested structure. It uses the standard `request<ReviewQueue>` helper — the SDK deserializes `json.data` which contains the nested `{ bullets: {...}, perspectives: {...} }` object.
