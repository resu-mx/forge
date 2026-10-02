# Content-to-Skill Gap Detection

`forge_detect_skill_gaps` — Find bullets that mention technologies in their content but lack the corresponding bullet_skills link.

## Problem
Many bullets describe work using specific technologies (e.g., "migrated to Terraform") but don't have the skill tagged in bullet_skills. This session found that Go, Rust, TypeScript, and most programming languages had 0 bullet links despite being clearly used in bullet content. Detection required manual keyword searches per language.

## Proposed Interface
```
{
  skill_ids?: string[],      // Check specific skills, or omit for all
  confidence?: "high" | "medium" | "low"   // Matching strictness
}
```

Returns per skill:
- skill: { id, name }
- untagged_bullets: [{ id, content_preview, match_context }]
- already_tagged_count: number

Match strategies:
- Exact name match in content (high confidence)
- Alias/variant matching, e.g., "Golang" → "Go" (medium)
- Semantic inference, e.g., "Terragrunt modules" → "HCL" (low)
