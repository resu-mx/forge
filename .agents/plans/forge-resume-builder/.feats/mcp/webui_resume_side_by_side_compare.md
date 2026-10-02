# Compare two arbitrary resumes side by side
**Type**: feature-request
**Component**: webui
**Filed**: 2026-04-09
**Status**: open

## Description

When building multiple resumes for the same team (e.g., Principal vs Lead SWE-AI at Thomson Reuters), users need to compare them side by side to ensure differentiation is correct and content isn't accidentally duplicated or missing.

## Expected Behavior

- User can select any two resumes and view them rendered side by side
- Differences should be visually highlighted (added/removed/changed entries)
- Shared entries (same perspective_id) should be visually linked
- Summary, experience, skills, projects, certs, education sections all comparable

## Use Cases

1. **Same team, different levels**: Verify summary tone differs, experience bullets are appropriate for each level, skills are correctly scoped
2. **Same role, different archetypes**: Compare infrastructure vs agentic-ai framing of the same experience
3. **Revision tracking**: Compare current version to a previous snapshot
