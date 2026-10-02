# Forge Development Backlog Session Prompt

Use this prompt to start a new session focused on Forge platform development work.

---

## Prompt

I want to work on Forge development backlog items. Check memory for `project_forge_dev_backlog_2026_04_15.md` for the full list.

The priorities are:

### Priority 1: MCP Tool Gaps (blocking resume building workflow)
Pick up the MCP feature requests in `.claude/plans/forge-resume-builder/.feats/mcp/`. Start with:
1. `forge_delete_resume_section` - most blocking, no way to remove sections via MCP
2. `forge_search_skills` - required DB queries every time we build a resume
3. `forge_add_education_entry` - education entries can't be added via MCP (needs source_id support)

For each: read the feature request file, implement the MCP tool, add tests, commit.

### Priority 2: Phase 101 Cleanup
- Extract `generateMarkdownFromIR()` from ResumeEditor + ResumePreview into a shared util
- Fix the ResumeSummaryCard test failure
- Consider extracting the view-tabs-container CSS fix into the component itself rather than inline in +page.svelte

### Priority 3: Resume Compiler Refactor
- The resume compiler at `packages/core/src/services/resume-compiler.ts` line 117 does raw SQL to fetch the profile, then manually JOINs address + URLs. Refactor to use ProfileService.getProfile() instead.

### Notes
- Dev server: `just dev` or `just inspect` (includes MCP inspector)
- Tests: `just test` (all), `just test-core` (core only)
- MCP tools are in `packages/mcp/src/tools/`
- The MCP tool registration pattern: check existing tools for the zod schema + handler pattern
