# Duplicate Skill Detection

`forge_detect_duplicate_skills` — Find skills with overlapping or redundant names.

## Problem
Skills accumulate organically — "Elastic" vs "Elasticsearch", "RAG" vs "RAG Pipelines", "Cloud" vs "Cloud (AWS, Azure, GCP)". Detecting these requires substring-match SQL queries that also produce false positives (e.g., "C" matching everything). No automated detection exists.

## Proposed Interface
```
{
  strategy?: "substring" | "fuzzy" | "both"   // default: "both"
  min_similarity?: number                       // 0-1 threshold for fuzzy (default: 0.8)
}
```

Returns candidate pairs with:
- skill_a: { id, name, bullet_count, resume_count }
- skill_b: { id, name, bullet_count, resume_count }
- match_type: "exact_ci" | "substring" | "fuzzy"
- recommendation: "merge_into_a" | "merge_into_b" | "keep_both"
