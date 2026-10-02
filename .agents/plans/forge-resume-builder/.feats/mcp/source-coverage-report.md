# Source Coverage Report

`forge_source_coverage` — Show bullet coverage per source, identifying thin or empty sources.

## Problem
This session discovered that all 32 project sources had 0 bullets. There's no tool to see which sources have good bullet coverage vs which are empty shells. The audit required a custom SQL join across sources, bullet_sources, and bullets.

## Proposed Interface
```
{
  source_type?: "role" | "project" | "education" | "general",
  min_bullets?: number,    // Filter: only show sources with fewer than N bullets
  max_bullets?: number
}
```

Returns per source:
- id, title, source_type
- bullet_count (non-rejected)
- bullet_statuses: { draft: N, approved: N, in_review: N }
- has_perspectives: boolean
- on_resumes: string[] (resume names using perspectives from this source)
