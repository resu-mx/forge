# Deleting a section in the UI does not delete its entries
**Type**: bug
**Component**: webui
**Filed**: 2026-04-08
**Status**: open

## Description

When a user deletes a resume section in the web UI, the section is removed but its `resume_entries` remain in the database as orphans. The user must manually delete each entry within the section first, then delete the empty section.

## Expected Behavior

Deleting a section should cascade-delete all `resume_entries` belonging to that section. This matches the mental model: "remove this section from my resume" means removing everything in it.

## Current Behavior

- Section row is deleted from `resume_sections`
- `resume_entries` rows with `section_id` pointing to the deleted section remain
- These orphaned entries are invisible in the UI but persist in the database
- They may cause issues if the section is recreated or entries are audited

## Fix

Either:
1. **Backend**: Add `ON DELETE CASCADE` to `resume_entries.section_id` FK (if not already present), and ensure the delete API uses a single DELETE that triggers the cascade
2. **API layer**: Wrap section deletion in a transaction that first deletes all entries with matching `section_id`, then deletes the section
3. **UI layer**: Before calling the section delete API, call remove-entry for each entry in the section (least preferred — fragile)
