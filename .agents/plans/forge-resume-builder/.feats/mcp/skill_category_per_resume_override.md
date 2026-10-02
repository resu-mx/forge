# Skills section should support per-resume category overrides
**Type**: feature-request
**Component**: resume-compiler, webui
**Filed**: 2026-04-08
**Status**: open

## Description

The Technical Skills section groups skills by their global `category` field from the `skills` table. This means the same skill always appears under the same category heading regardless of which resume it's on or how the user wants to present it.

## Problem

- **MLFlow, Feast, LakeFS** have `category = 'infrastructure'` globally, but on an AI-focused resume they'd be better grouped under "MLOps" or "Data Systems"
- **Prompt Engineering** has `category = 'methodology'` which creates a single-item "Methodology" row, when it would read better under "AI/ML"
- **GitLab CI/CD, GitHub Actions** are `infrastructure` but would be cleaner as a dedicated "CI/CD" row
- The user has no way to control this per-resume — changing the global category affects all resumes

## Expected Behavior

Each `resume_skills` entry should optionally override the display category for that skill on that specific resume.

## Proposed Schema Change

Add `category_override` to `resume_skills`:

```sql
ALTER TABLE resume_skills ADD COLUMN category_override TEXT;
```

Renderer logic:
```
display_category = resume_skills.category_override ?? skills.category
```

## Web UI

- Skills section editor should show the current category grouping
- DnD to move a skill between category groups sets the `category_override`
- A "Reset to default" option clears the override
