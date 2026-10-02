# Phase 94: Resume Source Tab Rendering

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None
**Blocks:** Nothing
**Parallelizable with:** All phases
**Duration:** Short (3 tasks)

## Goal

Fix the Source tab on the resume editor so View mode shows rendered output instead of raw code. Markdown View shows rendered HTML. LaTeX View shows the compiled PDF inline (reusing the existing Tectonic pipeline).

## Non-Goals

- Client-side LaTeX compilation (using server-side Tectonic)
- WYSIWYG markdown editing
- Live preview while editing (Edit mode stays as CodeMirror)

---

## Tasks

### T94.1: Markdown Rendering

**Steps:**
1. Add `marked` or `markdown-it` as a dependency in `packages/webui/package.json`
2. In `SourceView.svelte`, when format is Markdown and mode is View:
   - Render markdown string to HTML using the library
   - Display in a styled `<div class="markdown-preview">` instead of CodeMirror
   - Apply prose styling (typography, headings, lists, code blocks)
3. Edit mode: keep CodeMirror as-is (raw source editing)
4. Toggle between View (rendered) and Edit (raw) is the existing mode toggle

**Acceptance Criteria:**
- [ ] Markdown View shows rendered HTML with proper typography
- [ ] Headings, lists, code blocks, links all render correctly
- [ ] Edit mode still shows CodeMirror with raw markdown
- [ ] Switching between View/Edit is smooth

### T94.2: LaTeX Rendering

**Steps:**
1. In `SourceView.svelte`, when format is LaTeX and mode is View:
   - Call the existing PDF compilation endpoint (same one PdfView uses)
   - Display the compiled PDF inline in an iframe or embed element
   - Show loading state while compiling
   - Show error state if compilation fails
2. Edit mode: keep CodeMirror as-is
3. Cache the compiled PDF to avoid recompilation on mode toggle

**Acceptance Criteria:**
- [ ] LaTeX View shows compiled PDF inline
- [ ] Loading/error states handled
- [ ] PDF cached between View/Edit toggles
- [ ] Uses existing Tectonic compilation (no new infrastructure)

### T94.3: Tests

**Acceptance Criteria:**
- [ ] Markdown rendering produces valid HTML
- [ ] LaTeX view triggers PDF compilation
- [ ] Mode toggle preserves content
