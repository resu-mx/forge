# Phase 95: Resume Builder Polish

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** Phase 92 (Tagline Engine — for full tagline experience)
**Blocks:** Nothing
**Parallelizable with:** Phase 94 (Source Rendering), Phase 96-100
**Duration:** Medium (5 tasks)

## Goal

Make DragNDrop the default resume view (no tab needed), add summary import from summaries page, add resume-centric application tracking Kanban view, fix PDF preview sidebar toggle, and add JD overlay modal from resume page.

## Non-Goals

- Tagline generation (Phase 92)
- IR/compiler changes (those are in Phase 93/94)
- Full Kanban rewrite (reuse GenericKanban from Phase 43)

---

## Tasks

### T95.1: DragNDrop as Default View

**Steps:**
1. Make DragNDrop (Editor) the default/implicit view when landing on a resume
2. Remove DragNDrop from the tab bar — it's just the page content
3. Tab bar shows only: Preview, Source (LaTeX/Markdown sub-toggle lives inside Source)
4. Tabs shift to secondary navigation pattern (not primary content switcher)

**Acceptance Criteria:**
- [ ] Landing on a resume shows DragNDrop editor immediately
- [ ] Tabs for Preview and Source are secondary
- [ ] No "Editor" or "DragNDrop" tab label

### T95.2: Summary Import into Resume

**Steps:**
1. In the resume header section, add a "Import Summary" button/link
2. Clicking opens a picker modal showing existing summaries from `/resumes/summaries`
3. Selecting a summary sets it as the resume's summary/header content
4. Uses existing SDK summary list endpoint

**Acceptance Criteria:**
- [ ] Summary picker modal shows all summaries
- [ ] Selecting imports the summary content into the resume header
- [ ] Can be cleared/replaced

### T95.3: Application Tracking Kanban (Resume-Centric)

**Steps:**
1. Add a "Pipeline" or "Tracking" tab/section on the resume page
2. Shows a Kanban board of all JDs linked to this resume, using their JD pipeline status as columns
3. Reuse `GenericKanban` component from Phase 43
4. Cards show JD title, org name, status
5. Drag-and-drop changes JD status (same as the JD Kanban page, but filtered to this resume's links)

**Acceptance Criteria:**
- [ ] Kanban shows only JDs linked to the current resume
- [ ] Drag-and-drop updates JD status
- [ ] Cards show relevant JD info
- [ ] Reuses GenericKanban component

### T95.4: PDF Preview Sidebar Toggle

**Steps:**
1. When "Generate PDF" is clicked, auto-hide the sidebar
2. PDF preview gets full width
3. Sidebar can be toggled back on via existing toggle mechanism
4. Persist preference (or always collapse on PDF generate)

**Acceptance Criteria:**
- [ ] Sidebar collapses when PDF preview opens
- [ ] User can toggle sidebar back

### T95.5: JD Overlay Modal from Resume

**Steps:**
1. In the resume's linked JDs section, clicking a JD title opens a JD detail overlay modal
2. Modal shows JD content (title, org, raw text, status, skills)
3. Read-only view (not full CRUD — that's on the JD page)
4. Close button / escape to dismiss

**Acceptance Criteria:**
- [ ] Clicking linked JD opens overlay modal
- [ ] Modal shows JD details
- [ ] Modal is read-only
- [ ] Proper z-index (above all other content)
