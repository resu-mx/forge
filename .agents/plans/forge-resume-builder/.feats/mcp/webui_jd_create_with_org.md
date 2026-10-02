# Create organization inline during JD creation
**Type**: feature-request
**Component**: webui
**Filed**: 2026-04-09
**Status**: open

## Description

When creating a new Job Description, the user must select an existing organization. If the org doesn't exist yet, the user has to leave the JD form, create the org separately, then return to JD creation. This breaks flow.

## Expected Behavior

The organization field on the JD creation form should support:
1. **Select existing org** from dropdown (current behavior)
2. **Create new org inline** — "New Organization" option at top/bottom of dropdown opens an inline form or modal to create the org without leaving the JD form
3. After inline creation, the new org is automatically selected

## Similar Pattern

This is the same pattern as `feedback_target_employer.md` (resume target_employer should be org dropdown with select/create). Both should share the same org selector component.
