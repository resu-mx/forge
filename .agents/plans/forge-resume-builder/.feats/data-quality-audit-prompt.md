# Data Quality Audit — Agent Prompt

Use this prompt to start the next session continuing the Forge data quality work.

---

## Prompt

```
Forge data quality audit session. Check memory for project_snorkel_resume_2026_04_07.md for full context on what was done last session.

There are 10 beads issues to work through. Run `bd list --status open` to see them all. The priority order is:

### P1 — Do first
1. **job-hunting-0nf**: Rebuild `bullet_skills`, `certification_skills`, and `skill_domains` tables. These were wiped by a cascade bug in migration 044. The `skill_domains` table needs reseeding from the domain assignments in the skills table. The `bullet_skills` table needs rebuilding — each bullet has a `technologies` field in its source_content_snapshot that lists the linked skills. The `certification_skills` table needs rebuilding from certification records.

### P2 — Core audits (can be parallelized)
2. **job-hunting-vh9**: Clean up perspectives linked to rejected bullets. After the 47-bullet dedup, orphaned perspectives need to be migrated to surviving bullets or rejected.
3. **job-hunting-94g**: Skills taxonomy audit. Review all skills at /data/skills — check distribution across categories, identify misplacements, find gaps.
4. **job-hunting-w5z**: Domains and archetypes audit. Review domains and archetypes — add better descriptions, check completeness, note that 'ml-engineer' archetype is used by the Snorkel resume but doesn't exist in the DB.
5. **job-hunting-2jk**: Audit roles for overlap and bullet coverage. Compare Raytheon's 3 roles especially. Check which roles have zero/thin bullet coverage.
6. **job-hunting-oy2**: Audit projects for source data completeness. Many projects have thin descriptions. This needs human collaboration — propose what to expand.
7. **job-hunting-3kg**: Audit education sources. Fill in missing fields (dates, descriptions, credential links).

### P3 — After audits complete
8. **job-hunting-dr4**: Cross-entity taxonomy alignment. Sweep all bullets, perspectives, and resume entries to apply updated domains/archetypes/skills. Depends on 0nf + taxonomy audits.

### Other open issues
9. **job-hunting-8kd**: Extend perspectives to cover project sources (feature design, not urgent).
10. Presentation bullet synthetic ID bug (P2, code fix needed in compiler).

### Key context
- Forge MCP tools are available via stdio (64 tools). Use them for all data operations.
- The `forge_remove_resume_skill` and `forge_remove_resume_entry` tools have a void-return serialization bug. Use sqlite3 directly for deletions.
- Valid archetypes: agentic-ai, infrastructure, security-engineer, solutions-architect, public-sector, hft
- Valid domains: systems_engineering, software_engineering, security, devops, ai_ml, leadership
- The `skill_categories` table (migration 044) maps slugs to display names. The resume compiler already uses it.
- Active bullet count: 96 (after dedup). Total perspectives: ~124.
- Work collaboratively — propose changes, wait for approval before executing bulk operations.

Start with job-hunting-0nf (P1), then propose a plan for tackling the P2 audits — some can be parallelized with subagents.
```

---

## Notes

- The prompt references beads issues by ID. Run `bd show <id>` for full descriptions.
- The P1 rebuild (0nf) should be done first because bullet_skills links are needed for the taxonomy alignment sweep.
- The taxonomy audits (94g, w5z) should be done before the cross-entity sweep (dr4).
- The roles/projects/education audits (2jk, oy2, 3kg) are independent and can run in parallel.
- The perspective cleanup (vh9) can run alongside anything.
