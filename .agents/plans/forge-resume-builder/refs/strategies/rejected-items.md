# Rejected Items Strategy

## Problem

Rejected bullets and perspectives need a recovery path and audit context.

## Design

### Rejection Requires a Reason

`PATCH /bullets/:id/reject` and `PATCH /perspectives/:id/reject` require a `rejection_reason` in the request body. Without it, the request returns 400.

```json
{ "rejection_reason": "Overstates impact — source says 'contributed to' not 'led'" }
```

### Reopen Path

Rejected items can be re-opened to `pending_review` status:

```
rejected → pending_review → approved (or rejected again)
```

`PATCH /bullets/:id/reopen` and `PATCH /perspectives/:id/reopen` move the item back to `pending_review`. This does NOT clear the `rejection_reason` — the history is preserved so the reviewer can see why it was previously rejected.

### Status Transition Rules

```
draft ──────────► pending_review ──────► approved
                       │      ▲
                       ▼      │ (reopen)
                   rejected ──┘
```

Invalid transitions (return 400):
- `rejected → approved` (must go through pending_review)
- `approved → rejected` (approved items are terminal for MVP)
- `draft → approved` (must go through pending_review)
- `draft → rejected` (only pending_review items can be rejected)

### Rejected Items in the UI

- Shown in a "Rejected" tab/filter alongside pending and approved
- Rejection reason displayed inline
- "Reopen" button available on rejected items
- Rejected items are NOT available for perspective derivation or resume assembly
