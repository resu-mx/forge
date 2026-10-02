# Entity Status Definitions

## Source Statuses

| Status | Description | Transitions To |
|---|---|---|
| `draft` | Initial state, being authored | `approved` |
| `approved` | Human-verified, ready for bullet derivation | `deriving` |
| `deriving` | Locked — AI derivation in progress | Previous status (on completion/failure) |

**Source editing rule:** Editing an approved source does NOT automatically revert status to `draft`. The source stays `approved` and the chain view shows content divergence via snapshots. If the user wants to revoke approval, they explicitly set status back to `draft` via `PATCH /sources/:id { "status": "draft" }`. This is a deliberate design choice — silent status regression on edit would break workflows where minor source refinements don't invalidate existing bullets.

## Bullet Statuses

| Status | Description | Transitions To |
|---|---|---|
| `draft` | Manually created (not AI-derived) | `pending_review`, `approved` |
| `pending_review` | AI-generated, awaiting human review | `approved`, `rejected` |
| `approved` | Human-approved, available for perspective derivation | — (terminal for MVP) |
| `rejected` | Human-rejected with reason | `pending_review` (reopen) |

## Perspective Statuses

Same as Bullet statuses.

## Resume Statuses

| Status | Description | Transitions To |
|---|---|---|
| `draft` | Being assembled | `final` |
| `final` | Ready for export | `draft` (if modified) |

## Rules

1. Only `approved` sources can have bullets derived from them (enforced by DerivationService)
2. Only `approved` bullets can have perspectives derived from them (enforced by DerivationService)
3. Only `approved` perspectives can be added to resumes (enforced by ResumeService)
4. `deriving` is a lock status — only one derivation per source/bullet at a time
5. `rejected → pending_review` (reopen) is allowed; `rejected → approved` is not (must go through review)
6. `approved → pending_review` is NOT allowed for bullets/perspectives (approved is terminal for MVP)
7. Editing an `approved` source does NOT automatically change status — the chain view shows divergence via content snapshots. Users can explicitly set status back to `draft` if they want to revoke approval.
8. Rejected bullets CAN still have their content read for context, but perspectives CANNOT be derived from them. The UI should show rejected bullets as read-only context, not as derivation candidates.
