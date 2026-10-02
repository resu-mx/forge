# HTTP Response Shape Strategy

See also: [API Envelope Contract](../../contracts/api-envelope.md)

## Single Entity Response

```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "title": "Cloud Forensics Platform Migration",
    "description": "Led a team of 4 engineers...",
    "status": "approved",
    "created_at": "2026-03-28T10:00:00Z",
    "updated_at": "2026-03-28T10:00:00Z"
  }
}
```

## Nested Entity Response

GET endpoints that return related data nest them:

```json
{
  "data": {
    "id": "perspective-uuid",
    "content": "Led cloud platform migration...",
    "bullet": {
      "id": "bullet-uuid",
      "content": "Led 4-engineer team migrating...",
      "source": {
        "id": "source-uuid",
        "title": "Cloud Forensics Platform Migration"
      }
    }
  }
}
```

## Derivation Response

`POST /sources/:id/derive-bullets` returns the created bullets:

```json
{
  "data": [
    {
      "id": "bullet-uuid-1",
      "content": "Migrated cloud forensics platform...",
      "status": "pending_review",
      "technologies": ["ELK", "AWS OpenSearch"],
      "source_content_snapshot": "Led a team of 4 engineers..."
    },
    {
      "id": "bullet-uuid-2",
      "content": "Reduced mean incident response time...",
      "status": "pending_review",
      "technologies": [],
      "source_content_snapshot": "Led a team of 4 engineers..."
    }
  ]
}
```

## Error Response

```json
{
  "error": {
    "code": "CONFLICT",
    "message": "Cannot delete source: 3 bullets depend on it",
    "details": {
      "dependent_count": 3,
      "dependent_type": "bullets"
    }
  }
}
```

## Review Queue Response

```json
{
  "data": {
    "bullets": {
      "count": 5,
      "items": [
        {
          "id": "uuid",
          "content": "...",
          "source_title": "Cloud Forensics Platform Migration",
          "created_at": "2026-03-28T10:00:00Z"
        }
      ]
    },
    "perspectives": {
      "count": 3,
      "items": [
        {
          "id": "uuid",
          "content": "...",
          "bullet_content": "...",
          "target_archetype": "agentic-ai",
          "created_at": "2026-03-28T10:00:00Z"
        }
      ]
    }
  }
}
```
