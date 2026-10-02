# Forge Roadmap

Last updated: 2026-04-22

## Themes

| Theme | Bead | Description |
|-------|------|-------------|
| Core Platform | `forge-gqjq` | Data model, services, architecture foundation |
| Extension | `forge-3bp` | Browser extension (Chrome, Firefox, Safari) |
| Platform Integrations | `forge-orya` | External system connections and automation |

---

## Tier 0 — Bugs & Quick Wins (no deps, start anytime)

Standalone work items with no blocking dependencies.

### Bugs

| Bead | Category | Title |
|------|----------|-------|
| `forge-4g9l` | mcp | forge_approve_perspective must require human-in-the-loop review |
| `forge-uv0` | gui | Project entries render under role heading instead of project heading |
| `forge-gay` | gui | Header links missing https:// prefix |
| `forge-05v` | gui | Presentation bullet edits fail with 'Entry not found' |
| `forge-15f4` | ext | Parser L3 extractLocations() truncates multi-word city names |

### MCP Tools (independent, parallelizable)

| Bead | Title |
|------|-------|
| `forge-82e2` | forge_check_resume_staleness |
| `forge-2b8y` | forge_source_coverage report |
| `forge-6eoq` | forge_detect_duplicate_skills |
| `forge-h7gw` | forge_orphan_skills_report |
| `forge-no58` | forge_detect_skill_gaps |
| `forge-daly` | forge_skill_usage report |
| `forge-wvg` | forge_bulk_extract_jd_skills |
| `forge-gfg` | forge_bulk_link_bullet_sources |
| `forge-w4s` | forge_bulk_approve/reject_bullets |
| `forge-89a` | forge_bulk_add_bullet_skills |
| `forge-rbx` | Relational context in MCP tool responses |
| `forge-ggqp` | Per-resume skill category overrides |

### GUI Quick Wins (independent)

| Bead | Title |
|------|-------|
| `forge-4rhp` | Inline org creation during JD form submission |
| `forge-e60y` | Default ordering + drag-and-drop for experience entries |
| `forge-s7v4` | JD linked resumes should be clickable links |
| `forge-63h` | Notes: GFM Markdown preview with edit toggle |
| `forge-qcv` | Skills category drag-n-drop reorder in resume editor |
| `forge-9bb` | Design: reusable error indicator + issue resolution modal |
| `forge-4ze` | Organizations page scoped to orgs with JDs |

### Data Quality (independent)

| Bead | Title |
|------|-------|
| `forge-oy2` | Audit projects for source data completeness |
| `forge-8kd` | Extend perspectives to cover project sources |
| `forge-dr4` | Cross-entity taxonomy alignment |
| `forge-5ub` | Retrofit tests: acceptance, component, and contract coverage |

### Decisions

| Bead | Title |
|------|-------|
| `forge-1wti` | Resume compiler profile fetch refactor |

---

## Tier 1 — Foundation (unblocks Tiers 2-3)

These must ship before downstream features can start. Can be worked in parallel.

```
┌─────────────────────┐  ┌──────────────────────┐  ┌──────────────────────┐  ┌──────────────┐
│ forge-f4y            │  │ forge-6d0             │  │ forge-oop            │  │ forge-5sn    │
│ Skills Expansion     │  │ Industries &          │  │ Profile Rework       │  │ Quals        │
│ (Phase 89)           │  │ Role Types (Phase 90) │  │ (Phase 93)           │  │ (Phases      │
│                      │  │                       │  │                      │  │  84-88)      │
│ cat:subsys:skills    │  │ cat:core:datamodel    │  │ cat:subsys:profile   │  │ cat:subsys:  │
│                      │  │                       │  │                      │  │  qual        │
│ Blocks:              │  │ Blocks:               │  │ Blocks:              │  │ Blocks:      │
│  - 552 (Tagline)     │  │  - t30 (Summaries)    │  │  - 8b3h (Profile     │  │  - 2bmb (JD  │
│  - 7v6q (Spellings)  │  │                       │  │    Syncing)          │  │    quals)    │
│  - zb9u (Relations)  │  │                       │  │                      │  │  - sbba (Cert│
│  - fi01 (Pub/Priv)   │  │                       │  │                      │  │    ordering) │
│  - x20b (Matching)   │  │                       │  │                      │  │              │
│  - t30 (Summaries)   │  │                       │  │                      │  │              │
└─────────────────────┘  └──────────────────────┘  └──────────────────────┘  └──────────────┘

┌─────────────────────┐  ┌──────────────────────┐  ┌──────────────────────┐
│ forge-hzmp           │  │ forge-qmi             │  │ forge-uya            │
│ Contact Management   │  │ JD Multiple Locations │  │ Org Types: contractor│
│                      │  │                       │  │ & recruiter          │
│ cat:subsys:contact   │  │ cat:core:datamodel    │  │ cat:core:datamodel   │
│                      │  │                       │  │                      │
│ Blocks:              │  │ Blocks:               │  │                      │
│  - rr1 (Flags)       │  │  - 6ba (JD→Campus)    │  │                      │
│  - 78n (Auto-link)   │  │  - 80s (Choropleth)   │  │                      │
└─────────────────────┘  └──────────────────────┘  └──────────────────────┘
```

Also in Tier 1 (Extension track, runs in parallel):

| Bead | Title |
|------|-------|
| `forge-2xn` | Phase 97: UI Polish & Bug Fixes |
| `forge-yeh4` | Resume lifecycle statuses |

---

## Tier 2 — Core Features (depends on Tier 1)

### Summaries + Tagline (depends on Skills + Industries)

```
forge-f4y ──┐
            ├──► forge-t30 (Summaries Rework, Phase 91)
forge-6d0 ──┘
                    │
forge-f4y ──────► forge-552 (Tagline Engine, Phase 92)
```

### Skills Features (depends on Skills Expansion)

| Bead | Title | Blocked by |
|------|-------|------------|
| `forge-7v6q` | Skill alternative spellings / match patterns | forge-f4y |
| `forge-zb9u` | Skill relations design | forge-f4y |
| `forge-fi01` | Skills public/private flag | forge-f4y |
| `forge-x20b` | Skill extraction using existing data | forge-f4y |
| `forge-2bmb` | JD qualifications extraction | forge-5sn |
| `forge-mc98` | UX for correcting a skill gap in a JD | |
| `forge-vdid` | JD skills UI: Covered vs Gaps | |

### Contact & Event Features (depends on Contact Management)

| Bead | Title | Blocked by |
|------|-------|------------|
| `forge-rr1` | Contacts: Recruiter/Reference boolean flags | forge-hzmp |
| `forge-78n` | Contacts: links auto-set current contact as source | forge-hzmp |
| `forge-n6r4` | Event Management (interviews, calls, notes) | forge-hzmp |
| `forge-sbba` | Certification category ordering (drag-and-drop) | forge-5sn |

### Location Features (depends on Multi-Location JDs)

| Bead | Title | Blocked by |
|------|-------|------------|
| `forge-6ba` | JD locations → Org Campus linkage | forge-qmi |

---

## Tier 3 — Polish & Visualization (depends on Tier 2)

### Resume Polish (depends on Tagline)

```
forge-552 (Tagline) ──► forge-x03 (Resume Builder Polish, Phase 95)
```

### Visualization (depends on data enrichment)

| Bead | Title | Blocked by |
|------|-------|------------|
| `forge-80s` | Choropleth Enhancement (hex binning) | forge-qmi |
| `forge-01t` | Gantt Chart Fix | — (data quality audit first) |

### Platform (depends on Profile/Events)

| Bead | Title | Blocked by |
|------|-------|------------|
| `forge-8b3h` | Profile Syncing (Indeed, LinkedIn) | forge-oop |
| `forge-s43` | CalDAV Integration (Phase 100) | forge-n6r4 |

---

## Tier 4 — Long Horizon (ideas, no timeline)

### Extension Post-MVP

| Bead | Title |
|------|-------|
| `forge-dmcx` | Side panel UI |
| `forge-wf7x` | Resume selection for autofill |
| `forge-i1p4` | LLM-assisted field matching |
| `forge-h37u` | New job board plugin support → `forge-3kft` (optional permissions) |
| `forge-g2zi` | Safari support |

### Platform Integrations

| Bead | Title |
|------|-------|
| `forge-2cbx` | Indeed via Chrome Browser Automation |
| `forge-4iw9` | LinkedIn Messages |
| `forge-31hw` | Email Integration |
| `forge-2g2e` | Job Board Crawling (JobSpy) |

### Visualization

| Bead | Title |
|------|-------|
| `forge-cui9` | Bullets mindmap/sankey |
| `forge-lvvw` | Skills graph view (Obsidian-style) |
| `forge-idkc` | Side-by-side resume comparison |

### Architecture

| Bead | Title |
|------|-------|
| `forge-7b9p` | Phase 3: GraphQLite Adapter (research) |
| `forge-kftf` | Phase 4: DuckPGQ Adapter (research) |
| `forge-oar.2.1` | Turso / libSQL Deep Dive (research) |
| `forge-oar.2.2` | DuckDB Deep Dive (research) |
| `forge-dw4` | Phase 8: Rust Stubs → `forge-nfpz` (Dioxus Rewrite) |

### Developer Experience

| Bead | Title |
|------|-------|
| `forge-wwg1` | DevTools (fixtures, session replays, issue reporting) |
| `forge-1bl5` | Dev-mode: capture HTML fixture from current page |

---

## Dependency Graph (cross-epic)

```
f4y (Skills) ──┬──► 7v6q (Spellings)
               ├──► zb9u (Relations)
               ├──► fi01 (Public/Private)
               ├──► x20b (Matching)
               ├──► 552 (Tagline) ──► x03 (Resume Polish)
               └──► t30 (Summaries)
                        ▲
6d0 (Industries) ───────┘

5sn (Quals) ───┬──► 2bmb (JD quals extraction)
               └──► sbba (Cert ordering)

oop (Profile) ──► 8b3h (Profile Syncing)

hzmp (Contacts) ──┬──► rr1 (Recruiter/Ref flags)
                  ├──► 78n (Auto-link)
                  └──► n6r4 (Events) ──► s43 (CalDAV)

qmi (Multi-loc) ──┬──► 6ba (JD→Campus)
                  └──► 80s (Choropleth)

dw4 (Rust Stubs) ──► nfpz (Dioxus Rewrite)

h37u (New job board) ──► 3kft (Optional permissions)
```
