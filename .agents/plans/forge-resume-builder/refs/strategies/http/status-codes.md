# HTTP Status Code Strategy

## Success Codes

| Code | Meaning | Used For |
|---|---|---|
| `200 OK` | Request succeeded | GET (retrieve), PATCH (update), approve/reject/reopen |
| `201 Created` | Resource created | POST (create), derive-bullets, derive-perspectives |
| `204 No Content` | Deleted successfully | DELETE |

## Client Error Codes

| Code | Meaning | Used For |
|---|---|---|
| `400 Bad Request` | Invalid input | Missing required fields, invalid status transitions, malformed JSON, reject without reason |
| `404 Not Found` | Resource doesn't exist | GET/PATCH/DELETE with unknown ID |
| `409 Conflict` | State conflict | Delete blocked by dependents, derivation already in progress, duplicate unique constraint |

## Server Error Codes

| Code | Meaning | Used For |
|---|---|---|
| `500 Internal Server Error` | Unexpected failure | Unhandled exceptions, database errors |
| `501 Not Implemented` | Feature stubbed | Export endpoint (MVP) |
| `502 Bad Gateway` | AI returned invalid output | Malformed Claude Code response |
| `504 Gateway Timeout` | AI timed out | Claude Code exceeded 60s timeout |

## Route-Specific Status Codes

| Route | Success | Common Errors |
|---|---|---|
| `POST /sources` | 201 | 400 (missing title/description) |
| `GET /sources/:id` | 200 | 404 |
| `PATCH /sources/:id` | 200 | 404, 409 (deriving) |
| `DELETE /sources/:id` | 204 | 404, 409 (has bullets) |
| `POST /sources/:id/derive-bullets` | 201 | 404, 409 (already deriving), 502, 504 |
| `PATCH /bullets/:id/approve` | 200 | 404, 400 (wrong status) |
| `PATCH /bullets/:id/reject` | 200 | 404, 400 (wrong status, missing reason) |
| `POST /resumes/:id/export` | 501 | — |
