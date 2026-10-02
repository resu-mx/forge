# Bulk JD Skill Extraction

`forge_bulk_extract_jd_skills` — Extract and link skills from all unprocessed job descriptions.

## Problem
60 JDs were ingested but `job_description_skills` was empty — skills had never been extracted. `forge_extract_jd_skills` exists but works one JD at a time. During taxonomy triage, we had to grep JD raw_text manually to understand market demand for specific skills. Bulk extraction with automatic matching against the existing skills inventory would make JD-driven skill decisions data-informed.

## Proposed Interface
```
{
  jd_ids?: string[],          // Specific JDs, or omit for all unprocessed
  match_existing?: boolean,   // Auto-link to existing skills inventory (default: true)
  create_missing?: boolean    // Create new skills for unmatched extractions (default: false)
}
```

Returns per JD:
- jd_id, title
- extracted_skills: [{ name, matched_skill_id?, confidence }]
- new_skills_created: number (if create_missing)
