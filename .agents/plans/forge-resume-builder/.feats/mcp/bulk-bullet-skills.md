# Bulk Bullet-Skill Tagging

`forge_bulk_add_bullet_skills` — Add multiple skill links to multiple bullets in one call.

## Problem
Tagging skills to bullets one-at-a-time via `forge_add_bullet_skill` (or raw SQL) is slow and error-prone during taxonomy alignment sweeps. A single session required 93+ individual INSERT statements.

## Proposed Interface
```
{
  links: [
    { bullet_id: "...", skill_id: "..." },
    { bullet_id: "...", name: "Python", category: "language" }
  ]
}
```

Returns: `{ created: number, skipped: number, errors: [] }`

Idempotent — existing links are skipped, not errored.
