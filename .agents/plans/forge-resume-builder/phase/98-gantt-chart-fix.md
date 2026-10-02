# Phase 98: Gantt Chart Fix

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None
**Blocks:** Nothing
**Parallelizable with:** All phases
**Duration:** Short-Medium (4 tasks)

## Goal

Fix the Application Timeline Gantt chart on the Dashboard. Currently renders as lines on a background with no useful interaction. Needs proper rendering (time axis, JD bars with status coloring, milestones), interaction (hover tooltips, click to navigate to JD), and correct data (JD pipeline dates from status transitions).

## Non-Goals

- New pipeline statuses (use existing JD statuses)
- Calendar integration (Phase 100)
- Gantt editing (drag to change dates)

## Context

Phase 67 built the Gantt chart infrastructure with ECharts. The data model tracks JD status but may not have explicit date timestamps for each status transition. The pipeline: JD discovered → analyzing → applying → applied → interviewing → offered/rejected/withdrawn/closed. Each transition should have a timestamp to produce meaningful Gantt bars.

---

## Tasks

### T98.1: Audit Data Quality

**Steps:**
1. Check what date data exists for JD status transitions — is there a `status_history` or are transitions only tracked in `updated_at`?
2. If no transition history: add a `jd_status_history` table (jd_id, status, transitioned_at) or use the beads interactions log
3. Determine what a Gantt bar represents: each JD as a row, status durations as segments?
4. Decide on timeline axis: weeks? months?

**Acceptance Criteria:**
- [ ] Data source for Gantt identified or created
- [ ] Each JD has timeline data for rendering

### T98.2: Fix Gantt Rendering

**Steps:**
1. Fix ECharts Gantt configuration:
   - X-axis: time axis with proper date formatting
   - Y-axis: JD titles (one row per JD)
   - Bars: colored segments per status phase (discovered=gray, applying=blue, applied=green, interviewing=orange, etc.)
   - Use design tokens for status colors
2. Handle empty state (no JDs or no timeline data)
3. Responsive sizing

**Acceptance Criteria:**
- [ ] Gantt renders with proper time axis
- [ ] Each JD shows as a multi-segment bar
- [ ] Status colors match the JD pipeline theme
- [ ] Empty state handled

### T98.3: Add Interaction

**Steps:**
1. Hover: tooltip showing JD title, org, current status, duration in current status
2. Click: navigate to JD detail page
3. Legend: show status color mapping
4. Zoom: ECharts dataZoom for time axis (scroll to zoom)

**Acceptance Criteria:**
- [ ] Hover tooltips work
- [ ] Click navigates to JD
- [ ] Legend present
- [ ] Time axis zoomable

### T98.4: Tests

**Acceptance Criteria:**
- [ ] Gantt renders with mock timeline data
- [ ] Empty state renders without errors
- [ ] Interaction handlers fire correctly
