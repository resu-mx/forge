# Forge Resume Builder — Implementation Plan

**Date:** 2026-03-28 (updated 2026-03-31)
**Specs:**
- `refs/specs/2026-03-28-forge-resume-builder-design.md` (MVP)
- `refs/specs/2026-03-29-forge-schema-evolution-design.md` (Schema Evolution)
- `refs/specs/2026-03-29-resume-renderer-and-entity-updates.md` (Resume Renderer + Taxonomy + Org Updates)
- `refs/specs/2026-03-30-observability-structured-logging.md` (Structured Logging + Debug Store)
- `refs/specs/2026-03-30-observability-opentelemetry.md` (OpenTelemetry — future reference)
- `refs/specs/2026-03-30-provenance-tooltip-enhancement.md` (Provenance Tooltips)
- `refs/specs/2026-03-30-chain-view-edge-rendering.md` (Chain View Edges)
- `refs/specs/2026-03-30-dnd-role-bullet-add.md` (DnD Per-Role Bullet Add)
- `refs/specs/2026-03-30-resume-sections-as-entities.md` (Resume Sections as Entities)
- `refs/specs/2026-03-30-config-profile.md` (User Profile + Config)
- `refs/specs/2026-03-30-summaries-entity.md` (Summaries as Entities)
- `refs/specs/2026-03-30-summary-templates.md` (Summary Templates)
- `refs/specs/2026-03-30-job-descriptions-entity.md` (Job Descriptions)
- `refs/specs/2026-03-30-chain-view-modal.md` (Chain View Modal)
- `refs/specs/2026-03-30-config-export.md` (Config Export)
- `refs/specs/2026-03-30-resume-templates.md` (Resume Templates)
- `refs/specs/2026-03-30-nav-restructuring.md` (Nav Restructuring)
- `refs/specs/2026-04-03-education-subtype-fields.md` (Education Sub-Type Fields)
- `refs/specs/2026-04-03-org-kanban-board.md` (Organization Kanban Board)
- `refs/specs/2026-04-03-org-model-evolution.md` (Organization Model Evolution — retroactive + cleanup)
- `refs/specs/2026-04-03-bullet-detail-modal.md` (Bullet Detail Modal)
**Status:** Phases 0-26 complete. Phases 16-20 ready (renderer). Phases 27-28 ready (resume sections). Phases 29-38 complete (restructuring + education + kanban). Phases 39-68 planned (org cleanup, bullet modal, design system, kanban, JD, contacts, graph, charts, dashboard).

## Overview

Forge is an AI-backed resume builder with a strict derivation chain (Source → Bullet → Perspective → Resume Entry) that prevents hallucination through controlled intermediates and content snapshots. The MVP (Phases 0-9) is complete. Phases 10-15 evolved the schema for real data import, polymorphic sources, and entity-focused UI. Phases 16-20 add multi-format resume rendering (IR → DragNDrop/Markdown/LaTeX/PDF), editable taxonomy (domains/archetypes as DB entities), and organization tracking improvements. Phases 29-36 restructure the UI navigation, add user profile, summaries, job descriptions, resume templates, chain view modal, and data export.

## Architecture

- **API-First:** All consumers go through `@forge/sdk` → HTTP API → `@forge/core`
- **Runtime:** Bun (TypeScript), with Rust stubs for future rewrite
- **AI:** Claude Code CLI (`claude -p --output-format json`)
- **Storage:** SQLite via `bun:sqlite`
- **Frontend:** Svelte 5 + Vite
- **No auth** — single-user local tool

## Phases

| Phase | Title | Depends On | Duration Est. | Parallelizable |
|-------|-------|-----------|---------------|---------------|
| [0](phase/0-risk-gate-and-foundation.md) | Risk Gate & Foundation | — | Short | Task 0.1 ∥ Task 0.2 |
| [1](phase/1-shared-types-and-schema.md) | Shared Types & Schema | Phase 0.2 | Short | Task 1.1 ∥ Task 1.2 ∥ Task 1.4 |
| [2](phase/2-core-data-layer.md) | Core Data Layer (Repositories) | Phase 1 | Medium | All repos parallel after 2.1 |
| [3](phase/3-core-services-and-ai-module.md) | Core Services & AI Module | Phase 2, Phase 0.1, Phase 1.4 | Medium | Most services parallel; 3.3 needs 3.1 + Phase 2 |
| [4](phase/4-http-api.md) | HTTP API | Phase 3 | Medium | All routes parallel after 4.1 |
| [5](phase/5-sdk.md) | SDK | Phase 1 (types, early start), Phase 4 (integration tests) | Short | Types start early; integration after Phase 4 |
| [6](phase/6-cli.md) | CLI | Phase 5 (types) | Medium | ∥ Phase 7; all commands parallel after 6.1 |
| [7](phase/7-webui.md) | WebUI | Phase 5 (types) | Medium-Long | ∥ Phase 6; 7.7 first, then views parallel |
| [8](phase/8-rust-stubs.md) | Rust Stubs | Phase 3+ (captures implementation learnings) | Short | ∥ Phase 9 |
| [9](phase/9-integration-and-polish.md) | Integration & Polish | Phases 4-7 | Medium | All tasks parallel |
| | **── Schema Evolution ──** | | | |
| [10](phase/10-schema-evolution.md) | Schema Migration (002) | Phase 9 (stable baseline) | Medium | Sequential bottleneck |
| [11](phase/11-core-layer-updates.md) | Core Layer Updates | Phase 10 | Medium | T11.1-T11.6 ∥ T11.7-T11.13 |
| [12](phase/12-api-and-sdk-updates.md) | API + SDK Updates | Phase 11 | Medium | Routes ∥ SDK ∥ Tests |
| [13](phase/13-cli-and-import.md) | CLI + v1 Data Import | Phase 12 | Medium | CLI commands ∥; import sequential |
| [14](phase/14-ui-evolution.md) | UI Evolution | Phase 12 | Medium-Long | All views parallel after T14.1 |
| [15](phase/15-chain-graph-and-polish.md) | Chain Graph + Polish | Phase 14 | Medium | Graph ∥ E2E ∥ Docs |
| | **── Resume Renderer & Taxonomy ──** | | | |
| [16](phase/16-schema-migration-003.md) | Schema Migration (003) | Phase 15 (stable baseline) | Short | Sequential bottleneck |
| [17](phase/17-editable-domains-and-archetypes.md) | Editable Domains & Archetypes | Phase 16 | Medium | ∥ Phase 18, 19 |
| [18](phase/18-organization-updates.md) | Organization Updates | Phase 16 | Short | ∥ Phase 17, 19 |
| [19](phase/19-resume-ir-and-compilers.md) | Resume IR & Compilers | Phase 16 | Medium-Long | ∥ Phase 17, 18 |
| [20](phase/20-resume-renderer-ui.md) | Resume Renderer UI | Phase 19 | Medium-Long | DnD ∥ Markdown ∥ LaTeX ∥ PDF |
| | **── Observability ──** | | | |
| [21](phase/21-sdk-debug-logging.md) | SDK Debug Store + Client Logging | — (independent) | Short | Sequential |
| [22](phase/22-component-debug-helpers.md) | Component Debug Helpers | Phase 21 | Short | ∥ Phase 23 |
| [23](phase/23-server-structured-logging.md) | Server Structured Logging | — (independent) | Short | ∥ Phase 21, 22 |
| | **── Chain View & Provenance ──** | | | |
| [25](phase/25-chain-view-edge-interaction.md) | Chain View Edge Interaction | — (independent) | Medium | Execute first |
| [24](phase/24-provenance-tooltip-enhancement.md) | Provenance Tooltip Enhancement | Phase 19 + Phase 25 | Short | After Phase 25 |
| [26](phase/26-dnd-role-bullet-add.md) | DnD Per-Role Bullet Addition | Phase 24 | Short | Sequential (API → UI) |
| | **── Resume Sections ──** | | | |
| [27](phase/27-resume-sections-schema.md) | Sections Schema + Core Layer | Phase 26 | Medium-Long | T27.4 ∥ T27.7 with T27.5; T27.6 after T27.5 |
| [28](phase/28-resume-sections-ui.md) | Sections API + SDK + UI | Phase 27 | Medium-Long | T28.1 → T28.2 → T28.3 → T28.8 seq; T28.4 ∥ T28.5 ∥ T28.6 after T28.2; T28.7 in T28.8; T28.9 after T28.1 |
| | **── Restructuring (Migrations 005-008) ──** | | | |
| [29](phase/29-config-profile.md) | Config Profile | Phase 28 | Medium | Migration 005; ∥ Phase 31, 32, 36 |
| [30](phase/30-summaries-entity.md) | Summaries Entity | Phase 29 | Medium-Long | Migration 006; ∥ Phase 31, 32 |
| [31](phase/31-job-descriptions.md) | Job Descriptions | Phase 28; soft: 29, 30 (migration numbering) | Medium | Migration 007; ∥ Phase 29, 30, 32 |
| [32](phase/32-nav-restructuring.md) | Nav Restructuring | Phase 28 | Medium | UI only; ∥ Phase 29, 30, 31, 36 |
| [33](phase/33-chain-view-modal.md) | Chain View Modal | Phase 32 | Short-Medium | UI only; after Nav |
| [34](phase/34-summary-templates.md) | Summary Templates | Phase 30 | Medium | Extends summaries; no migration |
| [35](phase/35-config-export.md) | Config Export | Phase 29 | Medium | Soft deps: Phase 30, 31 |
| [36](phase/36-resume-templates.md) | Resume Templates | Phase 28 | Medium | Migration 008; ∥ Phase 29-32 |
| | **── Education Enhancement ──** | | | |
| [37](phase/37-education-subtypes.md) | Education Sub-Type Fields | Phase 28; soft: 29-36 (migration numbering) | Short-Medium | Migration 009; ∥ Phases 29-36 |
| | **── Organization Pipeline ──** | | | |
| [38](phase/38-org-kanban.md) | Organization Kanban Board | Phase 28; migration 011 (org_tags) | Medium | Migration 012; ∥ Phases 29-37 |
| | **── Org Model Cleanup ──** | | | |
| [39](phase/39-org-model-p1-cleanup.md) | Org Model P1 Cleanup | Phases 37-38 | Medium | Campus editing, OrgCombobox, compiler; no migration |
| [40](phase/40-org-model-p2p3-polish.md) | Org Model P2+P3 Polish | Phase 39 | Medium | Legacy column removal, type safety, card enrichment |
| | **── Bullet Management ──** | | | |
| [41](phase/41-bullet-detail-modal.md) | Bullet Detail Modal | Phase 28 | Medium | Backend + SDK + UI; no migration; ∥ Phases 29-40 |
| | **── Design System & UI ──** | | | |
| [42](phase/42-design-system.md) | Design System & CSS Variables | — | Medium-Long | Tokens, base.css, light/dark mode; foundational |
| [43](phase/43-generic-kanban.md) | Generic Kanban Interfaces | — | Medium-Long | Unified status model, GenericKanban, view toggle; migration 017 |
| [44](phase/44-ir-data-quality.md) | IR Data Quality | — | Short | Fix compiler: header/org/location/contact data |
| [45](phase/45-editor-restructuring.md) | Editor Restructuring | — | Medium | Editor/Preview/Source tabs, CodeMirror Compartment |
| [46](phase/46-latex-xetex-docs.md) | LaTeX/XeTeX Compat Docs | — | Short | Documentation only |
| [47](phase/47-clearance-structured.md) | Security Clearance Structured | — | Medium | Migration, enum constraints, hierarchy utility |
| [48](phase/48-generic-graphview.md) | Generic GraphView Component | — | Medium | Reusable Sigma.js wrapper; foundational for H2-H7 |
| | **── Wave 2 ──** | | | |
| [49](phase/49-jd-detail-page.md) | JD Detail Page | — | Medium | Full CRUD UI, skill tagging; foundational for E2-E6 |
| [50](phase/50-contacts-support.md) | Contacts Support | — | Medium-Long | New entity, 3 junction tables, cross-entity refs |
| [51](phase/51-graph-edge-rendering.md) | Graph Edge Rendering | Phase 48 | Short | Arrows, weight, edge hover |
| [52](phase/52-graph-node-labels.md) | Graph Node Labels | Phase 48 | Short | Slugs, hover labels, forceLabel |
| [53](phase/53-graph-filters.md) | Graph Filters | Phase 48 | Medium | Filter panel, dimming, URL persistence |
| [54](phase/54-graph-search.md) | Graph Search | Phases 48, 52 | Short | Autocomplete, camera focus |
| [55](phase/55-graph-toolbar.md) | Graph Toolbar | Phases 48, 54 | Short | Zoom, layout toggle, fullscreen |
| [56](phase/56-graph-local-widget.md) | Local Graph Widget | Phase 48 | Short | 1-hop neighborhood embed |
| [57](phase/57-ui-consistency.md) | UI Consistency | Phase 42 | Medium | Padding, grouping, type badge removal, CSS sweep |
| [58](phase/58-profile-menu.md) | Profile Button & User Menu | Phase 42 | Medium | Flyout menu, debug sub-pages, Config nav removal |
| [59](phase/59-echarts-infrastructure.md) | ECharts Infrastructure | Phase 42 | Medium | Tree-shakeable setup, Svelte wrapper, theme integration |
| | **── Wave 3 ──** | | | |
| [60](phase/60-jd-resume-linkage.md) | JD ↔ Resume Linkage | Phase 49 | Medium | Junction table, picker modals, reverse lookup |
| [61](phase/61-jd-kanban-pipeline.md) | JD Kanban Pipeline | Phases 43, 49 | Medium | 7-column pipeline, status migration |
| [62](phase/62-jd-skill-extraction.md) | JD Skill Extraction (AI) | Phase 49 | Medium | Claude CLI, prompt template, review UI |
| [63](phase/63-skills-sunburst.md) | Skills Sunburst Chart | Phase 59 | Short | ECharts sunburst + domain-archetype pie |
| [64](phase/64-skills-treemap.md) | Skills/Bullets Treemap | Phase 59 | Medium | 3 treemaps on dashboard |
| | **── Wave 4 ──** | | | |
| [65](phase/65-jd-skill-radar.md) | JD Skill Alignment Radar | Phases 49, 59 | Short | Spider chart on JD detail |
| [66](phase/66-jd-compensation.md) | JD Compensation Bullet Graph | Phases 49, 59 | Medium | Salary comparison chart, migration 021 |
| [67](phase/67-gantt-chart.md) | Application Gantt Chart | Phases 59, 61 | Medium | Timeline on dashboard, mock data first |
| [68](phase/68-choropleth-map.md) | Choropleth Map | Phases 49, 59 | Medium | US state heat map, GeoJSON, state resolver |
| | **── MCP Server & Embeddings ──** | | | |
| [69](phase/69-embedding-service.md) | Embedding Service Foundation | Phase 40 (stable schema) | Medium | Migration 020+; @xenova/transformers; ∥ Phases 42-68 |
| [70](phase/70-alignment-api.md) | Alignment & Health API | Phase 69 | Short-Medium | HTTP routes + SDK AlignmentResource; ∥ UI phases |
| [71](phase/71-mcp-server-foundation.md) | MCP Server Foundation + Tier 0-1 | Phase 70, Phase 5+ (SDK) | Medium-Long | STDIO transport, 7 resources, 21 tools |
| [72](phase/72-mcp-server-completion.md) | MCP Server Completion + Tier 2-3 | Phase 71; soft: 60, 62 | Medium | 36 tools, feature flags, e2e workflow test |
| | **── Database Research ──** | | | |
| [80](phase/80-db-research-requirements.md) | DB Research: Requirements & Evaluation Framework | — | Small | T80.1 ∥ T80.2 ∥ T80.3; T80.4 after T80.2 |
| [81](phase/81-db-research-candidate-evaluation.md) | DB Research: Candidate Deep Dives | Phase 80 | Medium | T81.1 ∥ T81.2 ∥ T81.3; T81.4 ∥ T81.5 after T81.1-3; T81.6 after all |
| [82](phase/82-db-research-deployment-architecture.md) | DB Research: Deployment & Architecture Validation | Phase 81 | Medium | T82.1 ∥ T82.2 after T81.6 top-2; T82.3 ∥ T82.4; T82.5 after all |
| [83](phase/83-db-research-decision.md) | DB Research: Decision & Migration Plan | Phase 82 | Small | T83.1 first; T83.2 XOR T83.3; T83.4 last |
| | **── Qualifications ──** | | | |
| [84](phase/84-qualifications-schema-migration.md) | Qualifications: Schema & Data Migration | — | Short | Sequential (migration bottleneck) |
| [85](phase/85-qualifications-core-layer.md) | Qualifications: Core Layer (Types, Repos, Services) | Phase 84 | Medium | T85.1 first; T85.2 ∥ T85.3; T85.4 ∥ T85.5; T85.6 ∥ T85.7 |
| [86](phase/86-qualifications-api-sdk.md) | Qualifications: API Routes & SDK | Phase 85 | Short-Medium | T86.1 ∥ T86.2; T86.3 ∥ T86.4; T86.5 last |
| [87](phase/87-qualifications-webui.md) | Qualifications: WebUI | Phase 86 | Medium | T87.1 first; T87.2 ∥ T87.3; T87.4 ∥ T87.5 |
| [88](phase/88-qualifications-ir-mcp.md) | Qualifications: IR Compiler & MCP Tools | Phase 85-86 | Short-Medium | T88.1 ∥ T88.2; T88.3 after; T88.4 last; ∥ Phase 87 |
| | **── Skills & Taxonomy ──** | | | |
| [89](phase/89-skills-expansion.md) | Skills Expansion & Technology Absorption | — | Medium | Sequential (migration + cascading type changes) |
| [90](phase/90-industries-role-types.md) | Industries & Role Types Entities | — | Short-Medium | ∥ Phase 89 |
| | **── Summaries & Taglines ──** | | | |
| [91](phase/91-summaries-rework.md) | Summaries Rework | Phase 89, 90 | Medium | T91.1 first; T91.4 ∥ T91.5; T91.6 last |
| [92](phase/92-tagline-engine.md) | Tagline Engine (IDF/IVF Keyword Relevance) | Phase 89 | Medium | T92.1 first; T92.2-T92.4 after; ∥ Phase 91 |
| | **── Profile & Resume ──** | | | |
| [93](phase/93-profile-rework.md) | Profile Rework | — | Medium | ∥ most phases |
| [94](phase/94-resume-source-rendering.md) | Resume Source Tab Rendering | — | Short | ∥ all phases |
| [95](phase/95-resume-builder-polish.md) | Resume Builder Polish | Phase 92 (soft) | Medium | T95.1-T95.5 mostly parallel |
| | **── Config & Info ──** | | | |
| [96](phase/96-config-info-scaffold.md) | Config & Info Pages Scaffold | — | Short-Medium | ∥ all phases |
| | **── Bug Fixes & Enhancements ──** | | | |
| [97](phase/97-ui-bug-fixes.md) | UI Polish & Bug Fixes | — | Short | ∥ all phases |
| [98](phase/98-gantt-chart-fix.md) | Gantt Chart Fix | — | Short-Medium | ∥ all phases |
| [99](phase/99-choropleth-enhancement.md) | Choropleth Enhancement (City/Zip + Hex Binning) | — | Medium | ∥ all phases |
| | **── External Integrations ──** | | | |
| [100](phase/100-caldav-integration.md) | CalDAV Integration | — | Medium-Long | ∥ all phases |

**Parallel execution warnings (Phases 17-19):**
- Phases 17 and 18 both modify `packages/sdk/src/types.ts`. If running in parallel, coordinate merges.
- Phases 17 (T17.4) and 19 (T19.9, T19.10) both modify `packages/core/src/services/resume-service.ts`. Execute sequentially or merge carefully.

**Parallel execution warnings (Phases 29-36):**
- Phase 29 → Phase 30 must execute sequentially (parseHeader dependency). Phases 31 and 36 are DDL-independent and can run in parallel with 29/30. All four migrations must apply in order 005→006→007→008.
- Phases 29, 30, 31, 36 all modify `packages/core/src/types/index.ts` and `packages/sdk/src/types.ts`. Merge carefully.
- Phases 29 and 30 modify `packages/core/src/services/resume-compiler.ts` (parseHeader evolution: `parseHeader(resume)` → `parseHeader(resume, profile)` in Phase 29 → `parseHeader(resume, profile, summary)` in Phase 30). Phase 35 calls the compiler but does not change the `parseHeader` signature.
- Phase 32 (Nav) modifies `+layout.svelte`. Phase 33 (Chain Modal) must apply after Phase 32.
- Phase 34 extends files created in Phase 30 (`summary-repository.ts`, `summary-service.ts`, `summaries.ts` routes). These phases are sequential — do not begin Phase 34 tasks until Phase 30 is merged.
- Phase 34 modifies `packages/webui/src/routes/data/summaries/+page.svelte` (created by Phase 32). Soft dep on Phase 32 for correct route path.

## Critical Path

```
0.1 (risk gate) ───────────────────────────────────────────► 3.1 (AI module) ──► 3.3 (derivation)
0.2 (scaffold) ──► Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5 (integration) ──► Phase 6 ∥ 7 ──► Phase 9
                    │                                                  ▲
                    └──► Phase 5 (types, early start) ─────────────────┘
                                                           Phase 8 (starts with Phase 9, off critical path)
```

**Key insight:** Phase 0 has two independent tasks with different downstream targets. Task 0.1 (risk gate) only blocks Phase 3 Task 3.1 (AI module). Task 0.2 (scaffold) blocks Phase 1. These run in parallel.

**Phase 5 early start:** The SDK base client and type definitions (Tasks 5.1-5.2) can begin after Phase 1 types are defined, building against the spec. Integration testing requires Phase 4. This means CLI and WebUI scaffolding can start earlier.

## Dependency Graph

```
Phase 0: Risk Gate & Foundation
  ├── Task 0.1: Claude CLI Verification (RISK GATE)
  │     └──► Phase 3 Task 3.1 (AI module needs verified flags)
  └── Task 0.2: Scaffold Monorepo
        └──► Phase 1

Phase 1: Shared Types & Schema
  ├── Task 1.1: Type Definitions ──► Phase 2, Phase 5 (SDK types, early start)
  ├── Task 1.2: SQLite Schema ──► Task 1.3
  ├── Task 1.3: Migration Runner ──► Phase 2
  └── Task 1.4: Archetype/Domain Constants ──► Phase 3 Task 3.4

Phase 2: Core Data Layer
  ├── Task 2.1: Connection Helper ──► Tasks 2.2-2.6 (repos can't compile without Database type)
  └── Tasks 2.2-2.6: Repositories (parallel) ──► Phase 3
      └── Task 2.6: PromptLogRepository ──► Phase 3 Task 3.3 (DerivationService writes prompt logs)

Phase 3: Core Services & AI Module
  ├── Task 3.1: AI Module (needs Phase 0.1) ──► Task 3.3
  ├── Tasks 3.2, 3.2b, 3.2c, 3.4-3.6: Services (parallel, need Phase 2 repos)
  │   └── Task 3.4: ResumeService (needs Phase 1.4 constants for gap analysis)
  └── Task 3.3: DerivationService (needs 3.1 + all Phase 2 repos) ──► Phase 4

Phase 4: HTTP API
  ├── Task 4.1: Server Setup (startup sequence: env validation → db → migrations → stale lock recovery → Claude check → serve)
  │     └──► Tasks 4.1b-4.7
  ├── Task 4.1b: Employer/Project/Skills Routes (new)
  └── Tasks 4.2-4.7: Routes (parallel) ──► Phase 5 (integration tests)

Phase 5: SDK
  ├── Tasks 5.1-5.2 (types + base client) ◄── Phase 1 (early start, build against spec)
  ├── Tasks 5.1-5.3 (integration tests) ◄── Phase 4 (needs running API)
  └── Tasks 5.1-5.3 ──► Phase 6, Phase 7

Phase 6: CLI ──────────────────┐
Phase 7: WebUI ────────────────┤──► Phase 9
Phase 8: Rust Stubs (after 3+) ┘

=== Schema Evolution (Phases 10-15) ===

Phase 9 (stable MVP) ──► Phase 10 (schema migration — BOTTLENECK)
                              │
                              ├──► Phase 11 (core layer: repos + services)
                              │         │
                              │         ├──► Phase 12 (API routes + SDK)
                              │         │         │
                              │         │         ├──► Phase 13 (CLI + v1 import)
                              │         │         └──► Phase 14 (UI evolution)
                              │         │                   │
                              │         │                   └──► Phase 15 (graph + polish)
                              │         │
                              │         └──► Phase 14 (can start with mocks)

=== Resume Renderer & Taxonomy (Phases 16-20) ===

Phase 15 (stable) ──► Phase 16 (schema 003 — BOTTLENECK)
                           │
                           ├──► Phase 17 (domains/archetypes)  ─┐
                           ├──► Phase 18 (org updates)          ├──► (independent, all parallel)
                           └──► Phase 19 (IR + compilers)      ─┘
                                     │
                                     └──► Phase 20 (renderer UI: DnD ∥ Markdown ∥ LaTeX ∥ PDF)

=== Observability (Phases 21-23 — independent, can start anytime) ===

Phase 21 (SDK debug store + logging) ──► Phase 22 (component helpers)
Phase 23 (server structured logging)    (independent, parallel with 21-22)

=== Chain View & Provenance (Phases 25 → 24 — sequential) ===

Phase 25 (chain edge interaction — independent, can start anytime)
  └──► Phase 24 (provenance tooltips — depends on Phase 19 IR compiler + Phase 25)
        └──► Phase 26 (DnD per-role bullet add — depends on Phase 24)
              └──► Phase 27 (sections schema + core — migration 004)
                    └──► Phase 28 (sections API + SDK + UI)

=== Restructuring (Phases 29-36 — migrations 005-008) ===

Phase 28 (stable with resume sections)
  │
  ├──► Phase 29 (Config Profile — migration 005)
  │     ├──► Phase 30 (Summaries Entity — migration 006)
  │     │     └──► Phase 34 (Summary Templates — extends summaries)
  │     └──► Phase 35 (Config Export — no migration)
  │
  ├──► Phase 31 (Job Descriptions — migration 007) ─── ∥ with 29, 30
  │
  ├──► Phase 32 (Nav Restructuring — UI only) ─── ∥ with 29, 30, 31, 36
  │     └──► Phase 33 (Chain View Modal — UI only)
  │
  └──► Phase 36 (Resume Templates — migration 008) ─── ∥ with 29, 30, 31, 32

=== Education Enhancement (Phase 37) ===

Phase 28 (stable)
  ├──► Phase 37 (Education Sub-Type Fields — migration 009) ─── ∥ with 29-36, 38
  └──► Phase 38 (Organization Kanban Board — migration 012) ─── ∥ with 29-37

=== Org Model Cleanup (Phases 39-40 — sequential) ===

Phase 38 (kanban complete)
  └──► Phase 39 (P1 Cleanup: campus editing, OrgCombobox, compiler)
        └──► Phase 40 (P2+P3 Polish: legacy columns, type safety, card enrichment)

=== Bullet Management (Phase 41 — independent) ===

Phase 28 (stable)
  └──► Phase 41 (Bullet Detail Modal) ─── ∥ with Phases 29-40

=== MCP Server & Embeddings (Phases 69-72 — sequential chain) ===

Phase 40 (stable schema baseline)
  └──► Phase 69 (Embedding Service: migration 020+, @xenova/transformers, JD parser)  ─── ∥ with Phases 42-68
        └──► Phase 70 (Alignment API: HTTP routes, SDK AlignmentResource, health endpoint)
              └──► Phase 71 (MCP Foundation: STDIO transport, 7 resources, Tier 0+1 tools)
                    └──► Phase 72 (MCP Completion: Tier 2+3 tools, feature flags, e2e tests)
                          ├── soft dep: Phase 60 (JD-Resume Linkage tools)
                          └── soft dep: Phase 62 (JD Skill Extraction tools)
```

## Spec Numbering Index (Phases 29-37)

| Spec # | Name | Spec File | Phase |
|--------|------|-----------|-------|
| Spec 1 | Nav Restructuring | `refs/specs/2026-03-30-nav-restructuring.md` | Phase 32 |
| Spec 2 | Summaries Entity | `refs/specs/2026-03-30-summaries-entity.md` | Phase 30 |
| Spec 3 | Summary Templates | `refs/specs/2026-03-30-summary-templates.md` | Phase 34 |
| Spec 4 | Job Descriptions | `refs/specs/2026-03-30-job-descriptions-entity.md` | Phase 31 |
| Spec 5 | Chain View Modal | `refs/specs/2026-03-30-chain-view-modal.md` | Phase 33 |
| Spec 6 | Config Profile | `refs/specs/2026-03-30-config-profile.md` | Phase 29 |
| Spec 7 | Config Export | `refs/specs/2026-03-30-config-export.md` | Phase 35 |
| Spec 8 | Resume Templates | `refs/specs/2026-03-30-resume-templates.md` | Phase 36 |
| Spec 9 | Education Sub-Type Fields | `refs/specs/2026-04-03-education-subtype-fields.md` | Phase 37 |
| Spec 10 | Organization Kanban Board | `refs/specs/2026-04-03-org-kanban-board.md` | Phase 38 |
| Spec 11 | Organization Model Evolution | `refs/specs/2026-04-03-org-model-evolution.md` | Phases 39-40 |
| Spec 12 | Bullet Detail Modal | `refs/specs/2026-04-03-bullet-detail-modal.md` | Phase 41 |
| Spec 13 | MCP Server Design | `refs/specs/2026-04-03-mcp-server-design.md` | Phases 69-72 |

## Risk Gates

| Risk | Impact | Mitigation | Phase |
|------|--------|-----------|-------|
| `claude -p --output-format json` unstable | Blocks AI module entirely | Verify first; fallback to API | Phase 0 |
| Bun workspace resolution issues | Blocks monorepo | Simple config, well-documented | Phase 0 |
| Svelte 5 runes API changes | Breaks WebUI reactivity | Pin version, use stable patterns | Phase 7 |
| Long AI derivation times (>60s) | UX issue, timeouts | 60s timeout, retry manually | Phase 3 |
| SQLite DROP COLUMN support | Blocks schema migration | Table-rebuild pattern as fallback | Phase 10 |
| v1 data import edge cases | Partial import, orphan data | Idempotent import with v1_import_map tracking | Phase 13 |
| Sigma.js bundle size | WebUI performance | Lazy-load graph view, code-split route | Phase 15 |
| Junction-only bullet_sources | Breaking change across 15+ files | Phase 10 updates all code paths before merge | Phase 10-11 |
| Tectonic not installed | PDF generation unavailable | Graceful 501 fallback, startup warning | Phase 19 |
| LaTeX special char escaping | Broken PDF output | `escapeLatex()` applied to all user content | Phase 19 |
| CodeMirror 6 in Svelte 5 | Editor initialization complexity | Compartment-based setup, cleanup in $effect | Phase 20 |
| Override staleness | Stale Markdown/LaTeX after DnD edit | Timestamp comparison + warning banner + "Regenerate" | Phase 20 |
| Migration 005-008 ordering | Parallel development creates numbering conflicts | DDL-independent; coordinate numbering before merge | Phases 29-36 |
| `sqlite3` not on PATH | Database dump fails | ENOENT catch → graceful error; document prerequisite | Phase 35 |
| Svelte 5 Set.add() reactivity | Accordion nav doesn't re-render | Use `$state<Record<string, boolean>>` instead of Set | Phase 32 |
| parseHeader evolution (2 phases) | Signature changes across 29→30 | Sequential execution for compiler changes; intermediate signatures documented | Phases 29, 30 |
| Summary data migration fragility | NULL target_role drops resume links | TypeScript migration helper with per-row UUID tracking | Phase 30 |
| @xenova/transformers Bun compat | Embedding service blocked | Fallback to TF-IDF cosine similarity (zero deps) | Phase 69 |
| Model download fails (network) | Embeddings unavailable | Skip embedding, log warning; checkStale() recovers later | Phase 69 |
| Migration 020+ numbering conflict | Schema migration fails | Audit full migration sequence before assigning number | Phase 69 |
| Embedding latency blocks entity creation | Slow bullet/perspective creation | Fire-and-forget (queueMicrotask); failures don't propagate | Phase 69 |
| Phase 60/62 not complete when MCP ships | 6 tools missing (JD skills, JD-resume linkage) | Feature flags: detect SDK methods at startup, omit tools gracefully | Phase 72 |
| PDF binary over MCP STDIO | Cannot serialize Blob to text | Write PDF to temp file, return file_path string | Phase 71 |
| STDIO resource subscriptions | Push notifications unsupported | Document polling pattern: re-read resource after mutations | Phase 71 |

## Reference Materials

All reference materials are in `refs/`:

### Taxonomy
- [Archetypes](refs/taxonomy/archetypes.md) — 6 resume archetypes
- [Domains](refs/taxonomy/domains.md) — 6 domain categories
- [Framings](refs/taxonomy/framings.md) — 3 narrative framing types
- [Statuses](refs/taxonomy/statuses.md) — entity status definitions and transitions

### Contracts
- [Result Type](refs/contracts/result-type.md) — SDK error contract
- [Entity Types](refs/contracts/entity-types.md) — all TypeScript type definitions
- [API Envelope](refs/contracts/api-envelope.md) — HTTP response format

### Schemas
- [001 Initial SQL](refs/schemas/001-initial.sql) — complete initial migration
- [ERD](refs/schemas/erd.md) — entity relationship diagram with cascade rules

### Spec
- [Dev Environment](refs/spec/dev-environment.md) — prerequisites, env vars, dev workflow
- [Production Environment](refs/spec/production-environment.md) — build, serve, deploy

### Strategies
- [DB Migration](refs/strategies/db-migration.md) — numbered SQL files approach
- [HTTP Status Codes](refs/strategies/http/status-codes.md)
- [HTTP Response Shapes](refs/strategies/http/response-shapes.md)
- [HTTP Pagination](refs/strategies/http/pagination.md)
- [Output Validation](refs/strategies/output-validation.md) — AI response validation pipeline
- [Rejected Items](refs/strategies/rejected-items.md) — rejection reason + reopen flow
- [Indexing](refs/strategies/indexing.md) — database index design
- [Tagging](refs/strategies/tagging.md) — technology tags + skill tags
- [Concurrency](refs/strategies/concurrency.md) — deriving lock, stale lock recovery
- [Versioning](refs/strategies/versioning.md) — content snapshots for chain integrity

### Examples
- [Rust Stub](refs/examples/rs-stubs/source-repository.rs) — example Rust stub file
- [SDK Error Handling](refs/examples/error/sdk/error-handling.md) — all error scenarios
- [Source → Bullet Prompt](refs/examples/prompts/source-to-bullet.md)
- [Bullet → Perspective Prompt](refs/examples/prompts/bullet-to-perspective.md)
- [E2E Derivation Chain](refs/examples/e2e/derivation-chain.md) — full workflow example
- [Gap Analysis Response](refs/examples/gap-analysis/response-shape.md)
- [Gap Analysis Algorithm](refs/examples/gap-analysis/algorithm.md)

### UI/UX
- [CLI Review Mockup](refs/uiux/mockups/forge-review-cli.md) — `forge review` terminal UX
- [WebUI Views](refs/uiux/mockups/webui-views.md) — all 5 MVP view descriptions
