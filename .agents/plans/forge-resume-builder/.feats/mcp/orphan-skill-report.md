# Orphan Skill Report

`forge_orphan_skills_report` — List skills with no bullet links, grouped by category and usage context.

## Problem
Finding orphan skills requires a custom SQL query joining skills against bullet_skills, resume_skills, and certification_skills. This session started with 107 orphan skills (57% of inventory) — identifying and triaging them required multiple ad-hoc queries.

## Proposed Interface
```
{
  include_usage?: boolean    // Also check resume_skills, certification_skills, jd_skills
}
```

Returns per skill:
- id, name, category
- bullet_count (always 0 for orphans)
- on_resumes: string[] (resume names that list this skill)
- on_certs: string[] (cert names that link this skill)
- on_jds: number (count of JDs requiring this skill)
- recommendation: "delete" | "tag_bullets" | "aspirational"
