# Skill Merge Tool

`forge_merge_skills` — Merge one skill into another, migrating all links atomically.

## Problem
Deduplicating skills (e.g., "Elastic" → "Elasticsearch", "RAG Pipelines" → "RAG") requires manually checking and migrating links across 4 junction tables (bullet_skills, resume_skills, certification_skills, skill_domains), then deleting the source skill. Error-prone and tedious.

## Proposed Interface
```
{
  source_skill_id: string,   // Skill to merge FROM (will be deleted)
  target_skill_id: string    // Skill to merge INTO (will be kept)
}
```

Behavior:
1. Migrate all bullet_skills links (INSERT OR IGNORE to handle overlaps)
2. Migrate all resume_skills links
3. Migrate all certification_skills links
4. Migrate all skill_domains links
5. Delete source skill
6. Return summary of migrated/skipped counts per table
