# Phase 96: Config & Info Pages Scaffold

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None
**Blocks:** Nothing
**Parallelizable with:** All phases
**Duration:** Short-Medium (6 tasks)

## Goal

Create stub/scaffold pages for: Account (auth placeholder), Plugins, Integrations, Privacy, Data Exports, Developer Tools, About, and Product Docs. All pages are placeholder UI with descriptive content about future functionality. No backend implementation — this is navigation + UI scaffolding only.

## Non-Goals

- Implementing auth (future)
- Implementing plugin/integration systems (future)
- Building real export pipelines (future — except SQL/JSON dump which is Phase 26-like)
- Writing actual documentation content

---

## Tasks

### T96.1: Navigation Updates

**Steps:**
1. Add to profile menu or sidebar:
   - Config group: Account, Plugins, Integrations, Privacy, Exports, Debug
   - Info group: About, Docs
2. These can be under the existing profile menu flyout or as new sidebar entries
3. Route structure: `/config/account`, `/config/plugins`, `/config/integrations`, `/config/privacy`, `/config/export`, `/debug/`, `/about`, `/docs`

**Acceptance Criteria:**
- [ ] All routes navigable
- [ ] Consistent placement in nav structure

### T96.2: Account Page (Auth Stub)

**Route:** `/config/account`
**Content:** Placeholder showing future auth capabilities (OAuth, Email, Passwordless). "Coming soon" messaging. No functional auth.

**Acceptance Criteria:**
- [ ] Page renders with descriptive placeholder content
- [ ] Lists planned auth methods

### T96.3: Plugin & Integration Stubs

**Routes:** `/config/plugins`, `/config/integrations`

**Plugins page:** Placeholder listing planned plugin types: Crawlers/Spiders, Browser Automation Tools, Token Embeddings. Empty state with "No plugins installed" + description.

**Integrations page:** Placeholder listing planned integrations: LinkedIn, OpenAI, Calendars (CalDAV), GitHub. Empty state with "No integrations configured" + description.

**Acceptance Criteria:**
- [ ] Both pages render with descriptive content
- [ ] Future capability descriptions present

### T96.4: Privacy & Data Export Stubs

**Routes:** `/config/privacy`, `/config/export`

**Privacy page:** Placeholder for:
- Toggle privacy visualization (color-code what data is shared)
- PII redaction in resume previews
- Data sharing preferences
- Descriptive mockup of the privacy visualization concept

**Export page:** Placeholder for:
- Resume export (PDF/LaTeX/MD) — link to existing resume export
- Data export (JSON) — stub button, non-functional
- Backup (SQL dump) — stub button, non-functional
- Describe what each export will contain

**Acceptance Criteria:**
- [ ] Privacy page describes visualization and redaction concepts
- [ ] Export page lists all export types with descriptions

### T96.5: Developer Tools & Debug Stubs

**Route:** `/debug/`

**Sub-pages (can be tabs or separate routes):**
- Prompt Logs: placeholder table showing what prompt log data will look like
- Forge Logs (API): placeholder for API request/response logs
- Event Logs: placeholder for system events
- UI/UX Logs: placeholder for client-side interaction logs

**Acceptance Criteria:**
- [ ] Debug landing page with navigation to sub-sections
- [ ] Each sub-section has descriptive placeholder content
- [ ] Uses existing `prompt_logs` data if available for Prompt Logs section

### T96.6: About & Docs Pages

**Routes:** `/about`, `/docs`

**About page:** Static content sections:
- About Forge (project description)
- Privacy Policy (placeholder text)
- Security (placeholder)
- Get Help/Support (link to GitHub issues or similar)

**Docs page:** In-app markdown-rendered docs:
- MCP (describe forge-mcp tools)
- API (describe HTTP endpoints)
- Configuration (describe config options)
- Pricing (placeholder — "Forge is open source")
- Setup (installation/getting started)
- Initially can just be placeholder headings with TODOs

**Acceptance Criteria:**
- [ ] About page has all 4 sections
- [ ] Docs page has navigation to all 5 doc sections
- [ ] Docs renders markdown content
