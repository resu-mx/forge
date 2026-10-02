# Bug: clearance entry_type should be removed from create_resume_section

## Problem

`forge_create_resume_section` exposes `clearance` as a valid `entry_type`. But clearance is handled mechanistically in the resume header via `show_clearance_in_header` (boolean) and the credentials table. A `clearance` section creates a redundant, duplicate display of clearance information.

The Federal Resume template also included a `clearance` section — fixed in DB on 2026-04-08.

## Fix

1. Remove `clearance` from the `entry_type` enum in `forge_create_resume_section`
2. Ensure templates don't include `clearance` sections (already fixed)
3. Consider a migration to remove any existing `clearance` sections from resumes

## Templates Fixed

All templates updated on 2026-04-08 to remove redundant `freeform` summary sections and the `clearance` section from Federal Resume. Summary content is handled via `summary_id` on the resume, not a freeform section.
