# Skill Usage Report

`forge_skill_usage` — Show a skill's complete usage across all entities in one view.

## Problem
Understanding a skill's footprint requires querying 5 tables: bullet_skills, resume_skills, certification_skills, skill_domains, job_description_skills. This session required separate queries for each to decide whether to keep, merge, or delete skills. A single "skill X-ray" view would replace 5 queries.

## Proposed Interface
```
{
  skill_id: string
}
```

Returns:
- skill: { id, name, category, notes }
- bullets: [{ id, content_preview, status }]
- resumes: [{ resume_id, resume_name, section_title }]
- certifications: [{ cert_id, cert_name }]
- domains: [{ domain_id, domain_name }]
- job_descriptions: [{ jd_id, jd_title }]
- summary: { total_references: number, is_orphan: boolean }
