# Phase 97: UI Polish & Bug Fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None
**Blocks:** Nothing
**Parallelizable with:** All phases
**Duration:** Short (5 tasks)

## Goal

Fix assorted UI bugs: profile modal z-index, profile modal name display, Notes create cancel button, Gap Analysis collapsible sidebar with notification bubble. These are all small, independent fixes.

## Non-Goals

- Feature work (separate phases)
- Profile page rework (Phase 93)

---

## Tasks

### T97.1: Fix Profile Modal Z-Index

**Problem:** The User Profile modal (bottom left button) renders behind main views but over the sidebar. Should render above everything.

**Steps:**
1. Find the ProfileMenu component's CSS
2. Set z-index to a value above all other content (use design token if available, e.g., `var(--z-modal)` or `z-index: 1000`)
3. Ensure backdrop also has correct z-index stacking

**Acceptance Criteria:**
- [ ] Profile modal renders above all other content
- [ ] No z-index conflicts with other modals

### T97.2: Fix Profile Modal Name Display

**Problem:** Profile modal shows "User" instead of the user's actual name.

**Steps:**
1. The layout already has `profileName = $derived(profileData?.name ?? 'User')`
2. Verify `profileData` is loaded before the modal renders
3. Ensure the modal button/display uses `profileName` or `initials` correctly
4. May be a timing issue — profile data not loaded when modal first renders

**Acceptance Criteria:**
- [ ] Profile modal shows actual user name
- [ ] Initials derived from actual name

### T97.3: Fix Notes Create Cancel Button

**Problem:** Notes create form at `/data/notes` has no Cancel button.

**Steps:**
1. Add Cancel button next to Create/Save button
2. Cancel clears the form and returns to list view / empty state
3. Use `.btn .btn-ghost` styling

**Acceptance Criteria:**
- [ ] Cancel button present on create form
- [ ] Cancelling clears form state

### T97.4: Gap Analysis Sidebar — Collapsible with Notification

**Problem:** Gap Analysis sidebar on resume page should be collapsible. When collapsed, show a notification bubble indicating gaps are present.

**Steps:**
1. Add collapse/expand toggle to Gap Analysis sidebar header
2. Collapsed state: sidebar shrinks to a thin strip or icon
3. When collapsed and gaps exist: show a notification badge/bubble (e.g., red dot with count)
4. Expanding shows full gap analysis content
5. Persist collapse preference (localStorage or state)

**Acceptance Criteria:**
- [ ] Sidebar can be collapsed and expanded
- [ ] Notification bubble shows when collapsed and gaps exist
- [ ] Preference persists across page navigation

### T97.5: Education Org No Industry Validation

**Problem:** Organizations with primary type "education" (tags include `university` or `school`) should not have an Industry field.

**Steps:**
1. In org form, conditionally hide/disable Industry field when org has education tags
2. On save, strip industry_id if org has education tags
3. Backend validation: reject industry_id for education orgs

**Acceptance Criteria:**
- [ ] Industry field hidden for education orgs
- [ ] Backend rejects industry on education orgs
- [ ] Existing education orgs with industry: cleared on next save
