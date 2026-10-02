# Default section ordering in Professional Experience with DnD override
**Type**: feature-request
**Component**: webui, resume-compiler
**Filed**: 2026-04-09
**Status**: open

## Description

Professional Experience entries should have a sensible default ordering with the ability to override via drag-and-drop.

## Current Behavior

Experience entries render in insertion order or by position field, with no automatic sorting logic. New entries append to the end regardless of chronology or relevance.

## Expected Behavior

### Default ordering (automatic)
Entries within each role subheading should default to a sensible order:
- **By relevance**: strongest/most relevant bullets first (could be informed by JD skill overlap scoring)
- **Or by insertion order**: preserve the order the user added them (current implicit behavior)

### Override (DnD)
- In the resume editor's DnD view, users can drag entries to reorder within a role subheading
- Reordering persists explicit position values that override the default sort
- A "Reset to default order" option clears overrides

### Scope
This applies to:
- **Bullet ordering within a role** (e.g., which Raytheon Principal bullet comes first)
- **Role ordering within an org** (e.g., Principal before Cloud Forensics Engineer — currently driven by date)
- **Org ordering** (e.g., Cisco before Raytheon — currently driven by date)

Date-based ordering should be the default for roles and orgs. Bullet ordering within a role is where DnD override is most valuable.
