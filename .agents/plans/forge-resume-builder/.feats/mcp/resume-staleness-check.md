# Resume Staleness Detection

`forge_check_resume_staleness` — Detect which resumes are affected by recent taxonomy/bullet/skill changes.

## Problem
After updating bullet_skills links, merging/deleting skills, or changing bullet content, there's no way to know which resumes need review. You have to manually trace: bullet → perspective → resume_entry → resume. This session updated 93 bullet_skills links and deleted 20 skills — no visibility into resume impact.

## Proposed Interface
```
{
  resume_id?: string,        // Check one resume, or omit for all
  since?: string,            // ISO date — only show changes since this date
  check_types?: string[]     // ["skill_gaps", "stale_perspectives", "new_bullets"]
}
```

Returns per resume:
- Skills on resume with 0 bullet backing
- Perspectives whose source bullets were updated after the perspective was created
- New approved bullets from resume's sources that have no perspective yet
- Skills added/removed from bullets that feed resume entries
