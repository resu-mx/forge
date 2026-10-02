# Project entries render under role heading instead of project heading
**Type**: bug
**Component**: resume-compiler
**Filed**: 2026-04-08
**Status**: open

## Description

When a bullet is linked to both a project source and a role source (via `bullet_sources`), and a perspective derived from that bullet is added to the "Selected Projects" section, the resume compiler uses the **role source** (via the perspective's source chain) to determine the project heading — not the project source.

## Reproduction

1. Bullet `2c1e88a6` is linked to both:
   - `b22d5454` (arustydev/zettlekasten-mcp, **project** source)
   - `39378eff` (Principal Cloud Forensics Engineer, **role** source)
2. Perspective `aa88c7d7` was derived from this bullet. Its source chain traces through the role source.
3. When added to the "Selected Projects" section, it rendered as:
   ```
   ### Principal Cloud Forensics Engineer
   - Executed 13-phase feature implementation...
   ```
   Instead of:
   ```
   ### arustydev/zettlekasten-mcp
   - Executed 13-phase feature implementation...
   ```

5 of 6 zettelkasten-mcp bullets rendered under the wrong heading because their perspectives traced through the role source.

## Root Cause

The resume compiler resolves project headings via `perspective → bullet → bullet_sources → source`, but it picks the first/primary source without checking the section type. In a "projects" section, it should prefer `source_type = 'project'` sources over role sources.

## Fix

When rendering entries in a `projects` section, the compiler should:
1. Look up all sources for the entry's bullet (via `bullet_sources`)
2. Prefer the source with `source_type = 'project'` for the heading
3. Fall back to role source only if no project source exists
