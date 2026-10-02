# SDK Error Handling Examples

## Not Found

```typescript
const result = await forge.bullets.approve('nonexistent-uuid')
// result = {
//   ok: false,
//   error: {
//     code: 'NOT_FOUND',
//     message: 'Bullet not found: nonexistent-uuid'
//   }
// }
```

## Invalid Status Transition

```typescript
const result = await forge.bullets.approve('bullet-in-draft-status')
// result = {
//   ok: false,
//   error: {
//     code: 'VALIDATION_ERROR',
//     message: "Cannot approve bullet: status is 'draft', expected 'pending_review'",
//     details: {
//       field: 'status',
//       current: 'draft',
//       expected: 'pending_review'
//     }
//   }
// }
```

## Derivation Conflict

```typescript
const result = await forge.sources.deriveBullets('source-already-deriving')
// result = {
//   ok: false,
//   error: {
//     code: 'CONFLICT',
//     message: 'Derivation already in progress for this source'
//   }
// }
```

## AI Timeout

```typescript
const result = await forge.sources.deriveBullets('source-with-long-description')
// result = {
//   ok: false,
//   error: {
//     code: 'GATEWAY_TIMEOUT',
//     message: 'AI derivation timed out after 60 seconds'
//   }
// }
```

## AI Malformed Output

```typescript
const result = await forge.sources.deriveBullets('source-id')
// result = {
//   ok: false,
//   error: {
//     code: 'AI_ERROR',
//     message: 'AI returned invalid output: missing required field "bullets"',
//     details: {
//       raw_response: '{"result": "something unexpected"}'
//     }
//   }
// }
```

## Delete Blocked by Dependents

```typescript
const result = await forge.sources.delete('source-with-bullets')
// result = {
//   ok: false,
//   error: {
//     code: 'CONFLICT',
//     message: 'Cannot delete source: 3 bullets depend on it',
//     details: {
//       dependent_count: 3,
//       dependent_type: 'bullets'
//     }
//   }
// }
```

## Network Error (Server Down)

```typescript
const result = await forge.sources.list()
// result = {
//   ok: false,
//   error: {
//     code: 'NETWORK_ERROR',
//     message: 'Failed to connect to http://localhost:3000'
//   }
// }
```

## Rejection Without Reason

```typescript
const result = await forge.bullets.reject('bullet-id', { rejection_reason: '' })
// result = {
//   ok: false,
//   error: {
//     code: 'VALIDATION_ERROR',
//     message: 'rejection_reason is required and must be non-empty'
//   }
// }
```
