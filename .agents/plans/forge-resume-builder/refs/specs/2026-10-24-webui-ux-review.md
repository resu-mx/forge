# Forge WebUI — UI/UX audit and redesign recommendation

**Date:** 2026-10-24
**Scope:** `packages/webui/` only (SvelteKit 2, Svelte 5, Vite, `adapter-static`)
**Status:** Recommendation. Nothing here is decided or scheduled.

> **Provenance.** This is the unedited report of a read-only review agent (Fable model)
> briefed as a senior frontend/product designer. Three claims were spot-checked against the
> code by the session that commissioned it, and all three held: the Derive button is
> `{#if false}` at `SourcesView.svelte:1419`; only `SourcesView` and `BulletsView` import
> `forge-changed`; and `Drawer`, `Entry`, `PaddedEntry`, `SectionedList` and `TagsList` have
> no importers and no markup use outside `components/index.ts`. Everything else, including
> the measured contrast ratios, click counts and the `bun run check` totals, is the agent's
> own and was not re-verified. Findings marked *inferred* come from code reading only.
>
> The screenshots it cites (118, by filename below) were saved to a session scratchpad and
> are **not committed**; treat the filenames as references to evidence that may no longer
> exist, and re-capture before relying on one.

## How this was done

Code read: `nav.ts`, `tokens.css`, `base.css`, all shared components, `+layout.svelte`,
every route, the six design-system specs plus the nav/kanban/graph/resume-preview specs.
App run in wasm mode (`just wasm-bundle typst-bundle` + Vite on :5173), populated with the
generated `cleared-security-engineer` demo dataset (14 sources, 40 bullets, 54 perspectives,
17 orgs, 10 JDs, 5 resumes, 39 skills), driven by Playwright (headless Chromium, persistent
OPFS profile) because no Chrome is installed on the review machine and the Claude-in-Chrome
and browser-use bridges could not attach. 42 routes captured at 1440 light, 11 key routes at
1440 dark and at 1024/768/390 light; all six task walks executed; keyboard walks; agent
writes simulated through `window.forge`. `bun run check` and the wasm core-loop acceptance
spec were run.

Not verifiable in that environment: the PDF `<iframe>` (headless has no PDF viewer; the PDF
bytes were checked instead: `%PDF-`, 60 KB, 119 ms), and the legacy
`e2e/accessibility.spec.ts`, which needs the TS API server (skipped; it only asserts three
ARIA roles anyway).

---

## 1. Verdict

The bones are good: a coherent token set with a working dark theme, a real shared-component
layer, a clean `SplitPanel`/`PageWrapper` contract, live refresh wired through
`forge:changed`, and the browser-first core loop passes end to end (`core-loop.spec.ts`: 1
passed, 6.5 s). The derivation chain (source → bullet → perspective → entry) is a genuinely
strong product idea and the resume builder's WYSIWYG editor with provenance tooltips is the
best screen in the app.

The single biggest problem: **the UI is organised around the database, not around the
job.** The nav is six entity groups; the review-and-approve loop is split across
"Experience" (sources), "Data › Bullets" (titled "Content Atoms"), a dashboard that goes
stale the moment the agent writes, and a derive action that has no entry point in the UI at
all. For a product whose stated future is "an agent writes, the human approves", there is no
approval surface: approving four bullets is 12 clicks through a modal that takes no focus,
with no multi-select, and list cards cannot be reached by keyboard.

**How much redesign: restructure, not rethink.** Keep the stack, tokens, shared components
and the builder. Replace the IA, build one review surface, fix the layout shell (banners are
clipped on every app-page), fix contrast on the status system, and make the app survive
below 1024 px.

---

## 2. Findings

Severity: blocker / major / minor / nit. Effort: S (<1 day) / M (days) / L (week+).
Confidence: observed (seen in the running app or measured) / inferred (from code only).

| ID | Route / component | Observation (screenshot) | Why it matters | Sev | Eff | Conf |
|---|---|---|---|---|---|---|
| B1 | `BulletsView`, `BulletDetailModal`, `GenericKanban` — task 2 | Approving one bullet = click card → Approve → close (3 clicks, modal fetch each time); 4 pending = 12 clicks. No multi-select, no "approve all", no keyboard shortcut. The list card is a `<div onclick>` and is **not reachable by Tab**; the modal is hand-rolled (not `<Modal>`), opens with focus on `<body>`, and 14 Tabs walk the list underneath it (`kbd-modal-focus.png`, `flow2-bullet-modal.png`). | This is the product's primary human job. Today it is slow with a mouse and impossible without one. | blocker | M | observed |
| B2 | `+layout.svelte` (fixed 220 px sidebar), all `SplitPanel` pages | At 390 px the sidebar leaves 170 px for content; at 768 px the detail pane of split panels is ~180 px and dashboard stat cards clip (`390-light-data_bullets.png`, `390-light-resumes.png`, `768-light-resumes.png`, `768-light-experience_roles.png`). A `scrollWidth` probe reported "ok" only because `<main>` clips its own overflow. Spec Doc 2 §13.3 declares mobile "not yet implemented". | A SaaS job-seeker will review agent output on a phone. Nothing works below 1024. | blocker (SaaS) | L | observed |
| B3 | `StatusBadge`, `tokens.css` | White on `--color-warning` (In Review) 2.15:1, on `--color-success` (Approved) 2.28:1, on `--text-faint` (Archived) 2.54:1, Withdrawn 2.80:1; `--text-faint` on surface 2.54:1 light / 2.80:1 dark (used for `EmptyPanel`, hints, card dates); `.btn-primary` 4.32:1 at 14 px medium (fails AA). Measured from token values (`contrast.ts`). | The status pill is the one thing a reviewer reads on every card and it fails AA on its two most important states. | major | S | observed |
| M1 | `nav.ts`, `/data/*`, `/opportunities/organizations` vs `/data/organizations` | Entity-oriented IA: "Data" is a grab-bag (Bullets, Skills, Contacts, Organizations, Domains, Notes). Organizations has **two unrelated UIs over one entity** — a split-panel CRUD under Data (`1440-light-data_organizations.png`) and an "Organization Pipeline" kanban under Opportunities with a third status vocabulary (`1440-light-opportunities_organizations.png`). Sources are called "Sources" on every Experience page although the nav says Roles/Projects. | New users cannot tell where the loop starts; the same noun appears in two groups meaning two things. | major | L | observed |
| M2 | `SourcesView` (Derive button is `{#if false}`), `DerivePerspectivesDialog` | After creating a source the detail pane shows the edit form and nothing else: no derived bullets, no "derive" CTA (`flow1-source-detail-after-derive.png`; detail text contains neither "bullet" nor "derive"). The only hint the agent exists is a 4-second info toast when you click Derive in the bullet modal ("Ask Claude in Chrome to derive this…", `flow2-derive-toast.png`). | Task 1 ("add a source and get bullets") dead-ends. Provenance is invisible where the decision is made. | major | M | observed |
| M3 | Dashboard, JD page, Resumes, Orgs, Skills, Chain (`forge:changed` only in `SourcesView`/`BulletsView`) | Agent write while on the page: bullets list 40→42 live (good); dashboard pending count 6→6 (7 after reload); JD list 10→10 after `jobDescriptions.create`. | The "needs review" counter is wrong exactly when the agent is working. | major | S | observed |
| M4 | Derivation handshake (API + UI) | A commit that fails validation (`bullets[1].metrics: missing required field`) leaves the derivation "in progress"; every later `prepare` returns `CONFLICT`. The UI shows nothing about in-progress or wedged derivations and offers no cancel. | Agents will produce malformed commits; a human must be able to see and clear the lock. | major | M (needs an API route too) | observed |
| M5 | `PageWrapper`, legacy `KanbanBoard` (`calc(100vh - padding*2)` + negative margin) | On every app-page the storage/alpha banners are clipped to a 6 px strip or overlapped by the page header: Skills, Contacts, Notes, Organizations, Job Descriptions, Resumes board, Org pipeline (`1440-light-data_organizations.png`, `1440-light-opportunities_job-descriptions.png`, `flow4-resumes-board.png`). | The banner that says "export your data or lose it" is unreadable on half the app; the runtime "waiting/error" banners would be too. | major | S | observed |
| M6 | `/` dashboard | With data: 4,300 px of seven charts under two pending cards; sunburst labels overlap and are illegible; dark mode chart titles/labels are unthemed dark-on-dark (`1440-light-dashboard-full.png`, `1440-dark-dashboard.png`). Empty: six "No data" sections and one CTA (`00-first-run-dashboard-1440-full.png`). Content is capped at 900 px leaving a third of a 1440 screen blank. | The home page answers no question a job-seeker has ("what needs me?", "what did the agent do?"). | major | M | observed |
| M7 | `/chain`, `ChainViewModal` | 108 nodes / 54 edges: labels overlap, edges near-invisible, no interaction cues; dark mode labels dark-on-dark (`1440-light-chain.png`, `1440-dark-chain.png`). Spec H1 explicitly excludes accessibility and dark mode. | Provenance is the product's differentiator and this is its only dedicated view; it is decorative at 100 nodes and unusable at 1,000. | major | M | observed |
| M8 | Resume builder right rail (`routes/resumes/+page.svelte` gap panel) | Gap Analysis renders 11–12 near-identical cards "UNUSED BULLET — This approved bullet has no perspective for archetype 'public-sector'" without naming the bullet; the panel takes one third of the width even when no resume is selected and during the create form (`flow4-builder-new-tall.png`, `flow4-new-resume-form.png`). | The one analysis the builder offers cannot be acted on. | major | S | observed |
| M9 | `BulletsView` board mode, `GenericKanban` | Two stacked filter systems (page search + status select, then the board's own Sources/Domains/search); the page search is ignored in board mode (typed "Zeek", board unchanged — `flow5-search-bullets-zeek.png`); board selects stretch to 968 px; filtering to zero items hides the filter bar with it (`GenericKanban.svelte:124-133`, inferred). | Users cannot find things, then cannot undo the filter that hid them. | major | S | observed / inferred |
| M10 | `ProfileMenu` | All config, settings, export, debug, about and theme live in a 20-item flyout behind the name button; includes "Login / Logout" → a "coming soon" page, and four Debug log pages (`flow6-profile-menu.png`, `1440-light-config_account.png`). | Task 6 is findable only by guessing the gear glyph; developer surfaces are exposed to end users. | major | S | observed |
| M11 | Design-system drift (whole package) | 193 hardcoded hex lines in 36 Svelte files; 315 hardcoded `font-size` lines in 55 files; 179 page-scoped button selectors in 42 files; 9 buttons use a variant class without `.btn` (JDEditor's Save/Delete render unstyled — bottom of `flow3-jd-editor-tall.png`); 14 hand-rolled dialogs vs 5 `<Modal>` uses; 49 `outline: none`; **0 usages** of `Drawer`, `Entry`, `PaddedEntry`, `SectionedList`, `TagsList`; page root `max-width` is 640/720/760/800/900/1000/1100/1400 px depending on the page. `bun run check`: 90 errors, 93 warnings (31 a11y). | The rules in `packages/webui/AGENTS.md` are not enforced and the spec's "CI tests are defined but not yet blocking" (ADR-004) is where it stays. | major | M | observed |
| M12 | Keyboard / structure | No skip link; 13 sidebar tab stops before any content; `<nav>` has no label; group toggles have no `aria-expanded`; `document.title` is "Forge" on 41 of 42 routes; no `<h1>` on any split-panel page (Roles, JDs, Orgs, Skills, Contacts, Notes, Credentials); status `<select>` has no visible focus (kbd logs). | Screen-reader and keyboard users have no landmarks, no page names, no way past the nav. | major | S | observed |
| M13 | Board drag-and-drop (`svelte-dnd-action`) | Drag is the only board transition; drop on Rejected opens the reason dialog, drop on Draft/Archived errors by toast; kanban spec lists keyboard DnD as a non-goal. Columns overflow with no visible affordance; Rejected/Archived cut off at the right (`flow2-bullets-board.png`, `flow4-resumes-board.png`). | A gesture-only workflow for the core approval step. | major | M | observed |
| M14 | States: `StorageBanner` + list pages; `/does-not-exist` | Second tab (runtime `waiting`) shows the correct banner but the list beneath says "No bullets found — No items match the current filters" (misleading). 404 is SvelteKit's default "404 / Not Found" with no link home (`1440-light-does-not-exist.png`). `friendlyError` tells wasm users to run `just api` in one branch. | Lists lie when the database is unavailable. | minor | S | observed |
| m1 | `/qualifications/credentials`, `/certifications` | Split panel collapses to content height (white box ends after two rows); list labels truncate at 320 px behind two badges (`1440-light-qualifications_credentials.png`). | Inconsistent with every other split panel. | minor | S | observed |
| m2 | Naming | "Content Atoms / Unified view of bullets and perspectives"; raw enum values as labels (`systems_engineering`, `security-engineer`, `ai_ml`) with three casings on one card; "Builder" nav item; "Sources" header on the Roles page. | Jargon leaks from the schema into the primary UI. | minor | S | observed |
| m3 | Domains / Archetypes tables, editors | Solid red Delete on every row; Delete sits beside Save in every editor; built-in templates show a disabled Delete instead of none (`1440-light-data_domains.png`). | Destructive actions carry the most visual weight on reference-data pages. | minor | S | observed |
| m4 | Toasts | Instructional content in a 4 s toast; two toasts stack over the header actions (`flow2-derive-toast.png`). | Instructions disappear before they are read. | minor | S | observed |
| m5 | Reduced motion | `base.css` covers modal/drawer/sidebar/collapsed-column only; `LoadingSpinner`, `StatusBadge` pulse, `Toast` slide, kanban flip (200 ms), `DragNDropView` are uncovered (grep). | WCAG 2.3.3. | minor | S | inferred |
| m6 | Dark theme | Banner link (`Settings → Storage`) is dark blue on dark (`1440-dark-dashboard.png`); chart titles/labels and graph labels unthemed. Everything else in dark is coherent. | Dark mode is 90 % done; the misses are the link and charts. | minor | S | observed |
| m7 | Source form (`SourcesView`) | 16+ fields for Education with Type as the first control; required markers inconsistent; Create button after Notes (`flow1-new-source-education-form.png`). | Long single-column forms with no progressive disclosure. | minor | M | observed |
| m8 | `/config/profile` | Inline `.page-header` instead of `PageHeader`; autosaves on input *and* has a Save button. | Two save semantics on one form. | minor | S | observed |
| m9 | Widths at 1440 | Dashboard (900) and Bullets (1000) leave the right third empty while split panels fill the width (`1440-light-data_bullets.png`). | No content-width rule. | minor | S | observed |
| n1 | First run | Alpha banner + storage banner stack; the Dashboard h1 sits 4 px under them (`00-first-run-dashboard-1440.png`). | Two warnings before the first word of product. | nit | S | observed |
| n2 | Org cards | "(1)" alias counts unexplained; "Worked" plain text label. | Unlabeled metadata. | nit | S | observed |
| n3 | Glyphs | Chevrons, gear, "GH/LI/WB" social buttons are text glyphs. | Reads as unfinished. | nit | S | observed |
| S1 | SaaS fit | `/config/account` "coming soon" with planned auth methods; About says "single-user local tool", "no authentication required"; Docs hardcodes port 5174 and `yourorg/forge`; menu "Login / Logout". | Copy and surfaces assert the single-user corner. | major (product) | S | observed |
| S2 | SaaS fit | No notion of workspace, role, team or org switcher anywhere; `localStorage` keys (`forge-theme`, `forge:viewMode:*`, `forge-gap-sidebar-collapsed`) are per-browser not per-user; preferences would not follow a user across devices. | Must be designed in before teams/billing. | major (product) | M | inferred |
| S3 | SaaS fit | Debug log pages, SQL dump and "Extension Config" in the user menu; `window.forge` exposed as the agent API in wasm mode. | Fine for dogfooding, not for tenants. | minor | S | observed |

Lenses with no further findings: **typography scale** (the 6-step scale and weights are
sound and consistently used where tokens are used); **toast system** beyond m4;
**Modal/ConfirmDialog** focus handling (correct where used).

---

## 3. Quick wins (all within the existing design system)

1. **Fix the shell.** Make `.content` a flex column; `PageWrapper` becomes `flex: 1;
   min-height: 0` with no `100vh`/negative-margin math; delete the legacy `KanbanBoard`'s
   own height calc. Fixes M5 on seven pages. (S)
2. **One live-refresh helper.** Export `useLiveList(load)` from `forge-changed.ts` and call
   it in every list page and the dashboard. Fixes M3. (S)
3. **Status pills on `-subtle`/`-text` token pairs** (bg `--color-warning-subtle`, fg
   `--color-warning-text` etc.), raise `--text-faint` to ≥4.5:1 on surface, add a
   `--focus-ring` token and a global `:focus-visible` rule; drop the 49 `outline: none`.
   Fixes B3, part of M12. (S)
4. **Make list cards buttons and route `BulletDetailModal` through `<Modal>`** (focus trap,
   initial focus, Escape). Add Approve / Reject buttons directly on in-review cards (list
   and board). Halves the clicks of B1 and makes it keyboard-operable. (S–M)
5. **Bulk approve.** Checkbox on in-review cards + sticky action bar (Approve n / Reject n).
   Completes B1. (M)
6. **Per-page `<svelte:head><title>`, an `h1` on split pages, a skip link, `aria-label` on
   nav, `aria-expanded` on groups.** (S)
7. **Rename and default.** "Content Atoms" → "Review" with the status filter defaulting to
   In Review when anything is pending; humanise enum labels in one `label()` helper. (S)
8. **Settings page.** `/settings` with left tabs (Profile, Preferences, Work authorization,
   EEO, Storage & export, Integrations, Plugins, Account); flyout keeps Profile / Theme /
   Sign out; Debug behind `import.meta.env.DEV`. (S)
9. **Gap panel: aggregate.** One card per gap type with the affected bullets listed and
   linked; collapse the rail until a resume is selected. (S)
10. **Board filters: one row**, selects `max-width: 240px`, page search applies in both
    modes, filter bar stays when the result is empty. (S)

---

## 4. Redesign proposal

### 4.1 IA and nav (goal-oriented)

```
Inbox            /                 what needs you: in-review bullets & perspectives, drifted
                                   entities, wedged derivations, recent agent activity
Experience       /experience       tabs: Roles · Projects · Education · Presentations · Other
                                   · Credentials · Certifications        (one entity family, one page)
                 /experience/:id   source detail with its bullets + perspectives + "derive" CTA
Content          /content          bullets & perspectives (tabs), list + board, bulk actions
Opportunities    /opportunities    tabs: Jobs · Companies · Contacts
                 /opportunities/jobs/:id   JD detail with alignment, linked resumes, pipeline stepper
Resumes          /resumes          list → /resumes/:id builder; Summaries and Templates as tabs
Insights         /insights         the charts (sunburst, treemaps, timeline, map, optional graph)
Settings         /settings/*       profile, preferences, work auth, EEO, storage, integrations, account
Library          /settings/library skills · domains · archetypes · notes  (reference data, rarely visited)
```

Moves: **Data group dies** (Bullets → Content; Skills/Domains/Notes → Library; Contacts →
Opportunities). **Organizations merge** into one Companies entity with a list view and a
"targeting" board view (the pipeline statuses become a field, not a second entity).
**Dashboard becomes Inbox**; charts move to Insights. **Chain View dies as a page**;
provenance becomes a panel on every entity detail (source ↔ bullets ↔ perspectives ↔ resume
entries) with `LocalGraphWidget` as an optional 1-hop widget. `/experience/clearances`,
`/data/*`, `/config/*`, `/chain` stay as redirects (the pattern already exists in the
repo). Nav depth stays at two levels; tabs replace most children so the sidebar is eight
items.

### 4.2 Screens

**Inbox / review surface** (replaces Dashboard + the review half of Content Atoms)

```
┌──────────┬──────────────────────────────────────────────────────────────────────┐
│ Forge    │ Inbox                                            [Agent activity ▾] │
│ ▸ Inbox 7│ ┌────────────────────────────────────────────────────────────────┐   │
│ Experien.│ │ 6 bullets · 3 perspectives waiting   1 source locked by agent  │   │
│ Content  │ │ [All] [Bullets] [Perspectives]   from: [Any source ▾]  ⌕ search │   │
│ Opportun.│ ├──┬─────────────────────────────────────────────┬───────────────┤   │
│ Resumes  │ │☐ │ Cut mean time to patch from 30 to 9 days …  │ Approve Reject│   │
│ Insights │ │  │ ← Agent Patch Program · Python · 2 min ago  │               │   │
│          │ │☐ │ Built the patch-compliance dashboard …      │ Approve Reject│   │
│          │ │☑ │ Designed identity-aware access for a …      │ Approve Reject│   │
│          │ │  │   perspective of "Integrated device posture" │               │   │
│ ─────────│ ├──┴─────────────────────────────────────────────┴───────────────┤   │
│ CT Casey │ │ ▶ Preview: full text, source excerpt (snapshot vs current),     │   │
│   ⚙      │ │   skills, drift diff, [Edit] — opens inline, not in a modal    │   │
│          │ └────────────────────────────────────────────────────────────────┘   │
│          │ ▇ 2 selected   [Approve 2] [Reject 2…] [Archive]        j/k a r e ? │
└──────────┴──────────────────────────────────────────────────────────────────────┘
```

Annotations: rows are `<button>`s in a `role="list"`; checkbox column enables the sticky
bulk bar; provenance (source chip, "n min ago", agent client id) is on the row because that
is where the decision is made; the preview pane is a `ContextRail`, not a modal; keyboard:
j/k move, a approve, r reject, e edit, ? help. Live via `forge:changed`. Locked/wedged
derivations appear as a row with "Clear lock".

**Source detail** (Experience › Roles › item; fixes M2)

```
┌ Senior Security Engineer, Ridgeback Federal Solutions         Approved  [Edit] [⋯] ┐
│ Jul 2023 – Present · Ridgeback Federal Solutions · remote                            │
│ Description ……………………………………………………………………………………………………………………………… │
│ Skills: Ansible  Linux  OpenSCAP  +2                                                  │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ Derived bullets (5)                        [Ask agent to derive ▾] [+ Write bullet]  │
│   ● In Review  Cut a federal program's annual assessment …      3 perspectives  ▸    │
│   ● Approved   Led three mission systems through the RMF …      2 perspectives  ▸    │
│   ○ Draft      Brought a mission program's 1,200 servers …      0                ▸    │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ Used in resumes (2): DSFN – Lead Cybersecurity Engineer · Larkspur – ISSO             │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

Annotations: "Ask agent to derive" calls `derivations.prepare` and shows the returned
`instructions` in a copyable `AgentPrompt` panel with the expiry, and surfaces the
in-progress state until commit; the bullets list is the provenance panel (replaces Chain
View for this entity); "Used in resumes" closes the loop upward.

**Resume builder** (keeps the current editor; restructures the chrome)

```
┌ ← Resumes   DSFN – Lead Cybersecurity Engineer (Federal)   Approved   [Preview] [Export ▾] ┐
│ Targets: [Lead Cybersecurity Engineer · DSFN ✕] [+ Job]   Archetype: public-sector      │
├────────────┬───────────────────────────────────────────────┬────────────────────────────┤
│ Outline    │  Casey Thornbury                               │ Rail  [Gaps 3] [Match] [Add]│
│ ─ Summary  │  Lead Cybersecurity Engineer -- …   [tagline ▾]│ ▸ Missing domain:          │
│ ─ Clearance│  Active TS/SCI w/ CI Poly   [hide]             │   systems_engineering (0)   │
│ ● Experien.│  SUMMARY OF QUALIFICATIONS            ⋯        │   → 4 approved bullets have │
│   418th    │  Eleven years securing …                       │     no public-sector        │
│   Ridgebac.│  PROFESSIONAL EXPERIENCE              ⋯        │     perspective [show]      │
│   Granite  │  418th Expeditionary Signal Battalion          │ ▸ JD match: 5/6 skills     │
│ ─ Skills   │   ≡ Defended a deployed tactical network …  ✎ ✕│   missing: Terraform        │
│ ─ Education│   ≡ Closed 30 findings from a command …     ✎ ✕│ ▸ Add from this role:       │
│ + Section  │   [+ Add from this role]                       │   2 approved perspectives   │
├────────────┴───────────────────────────────────────────────┴────────────────────────────┤
│ Preview: PDF | Markdown     Format: Visual | LaTeX | Markdown        Saved · fresh PDF  │
└───────────────────────────────────────────────────────────────────────────────────────────┘
```

Annotations: three panes with `min-width` rules that collapse to tabs below 1024; the right
rail is a `ContextRail` whose tabs replace the gap sidebar, the perspective picker modal and
the JD link list; Preview is a toggle on the same screen, not a tab that hides the editor;
entry provenance stays on hover *and* in the rail's "Add" tab.

**Job detail** (Opportunities › Jobs › item; task 3)

```
┌ Senior DevSecOps Engineer · Halyard Analytics Group           Discovered ▸ Analyzing ▸ Applying ▸ … ┐
│ Remote (US) · $166k–$190k · jobs.example.com ↗                                     [Edit] [Build resume]│
├───────────────────────────────────────────────┬─────────────────────────────────────────────────────┤
│ Description (raw text, read mode)             │ Alignment   5 of 5 required skills matched  ▮▮▮▮▮ 100%│
│ # Senior DevSecOps Engineer …                 │ Required: DevSecOps ✓ Docker ✓ Go ✓ Kubernetes ✓ …     │
│                                               │ [Extract skills via agent] [+ add skill]                │
│                                               │ Compensation vs your range  ├──■■■──┤ min·target·stretch│
│                                               │ Resumes: Tessellate – Cloud Security Architect (draft)  │
│                                               │ Notes (0) [+]                                           │
└───────────────────────────────────────────────┴─────────────────────────────────────────────────────┘
```

Annotations: status becomes a stepper (the pipeline is a progression, not nine select
options); read mode by default so the 10-row textarea stops dominating; the radar chart is
replaced by a matched/missing list (a 3-axis radar at 100 % says nothing a list cannot).

### 4.3 Design-system changes

Tokens: add `--status-{pending,approved,rejected,draft,archived,info}-{bg,fg}` pairs and JD
pipeline equivalents (removes the hex in `StatusBadge`/`JDOverlayModal`/column accents);
`--focus-ring`; `--color-link`; `--content-max` (one content width, e.g. 1120 px) and
`--sidebar-collapsed-width`; breakpoint custom properties documented in `tokens.css`; chart
text/axis color tokens consumed by `echarts-theme` and the Sigma label renderer; raise
`--text-faint` to 4.5:1 or restrict it to non-text.

New shared components: `AppShell` (content as flex column with a banner slot — kills the
PageWrapper math), `Page` (title, subtitle, actions, sets `document.title` and renders the
`h1`; replaces `PageHeader` + inline headers), `SelectableList` + `BulkBar`, `StatusPill`
(replaces `StatusBadge`), `FilterBar` (one row, chips, "clear"), `ContextRail` (the right
rail; `Drawer` repurposed to this and finally used), `ProvenanceCard`, `AgentPrompt`,
`Stepper`, `Toast` with an action slot.

Retire: `ChainViewModal` as a full-screen view (keep `LocalGraphWidget`); legacy
`KanbanBoard`/`KanbanColumn`/`KanbanCard` (org board moves to `GenericKanban`); `Entry`,
`PaddedEntry`, `SectionedList`, `TagsList` (unused); `EmptyPanel` (fold into an
`EmptyState` variant); `.page-title`, `.list-panel`, `.editor-panel` legacy classes in
`base.css`; `ListPanelHeader`'s private button (use `.btn .btn-primary .btn-sm`, closing
ADR-003's debt item).

Rule changes to `packages/webui/AGENTS.md` (proposed, with reasons): (9) every dialog goes
through `<Modal>`/`<ConfirmDialog>` — 14 hand-rolled dialogs is why focus handling is
inconsistent; (10) no `outline: none` without a `:focus-visible` replacement; (11) status
colour only via `StatusPill`; (12) one page per `Page` component so titles and h1s cannot be
forgotten; and make the ADR-004 grep tests blocking in CI — they exist and are the cheapest
enforcement available.

### 4.4 Alternative considered and rejected

**Board-first review** — make the kanban (already built, already in the acceptance spec) the
primary approval surface and polish drag-and-drop. Rejected: a gesture is the slowest way to
make 40 binary decisions, it cannot be bulk-operated, the kanban spec itself lists keyboard
DnD as a non-goal, and the five-status model turns "approve" into a spatial move with two
columns (Draft, Archived) that reject the drop. Boards stay as an optional view for
pipeline-shaped data (job search stages), not for review.

---

## 5. Migration path

Each step is shippable alone; `just test`, the e2e specs and the Rust/TS parity job stay
green. Nothing below changes the API contract except the one flagged item.

1. **Shell and tokens** (S, 1–2 days): `AppShell` flex layout, `PageWrapper` simplification,
   status/focus/faint tokens, `StatusPill` as a drop-in for `StatusBadge`, dark-mode link and
   chart text colours. No route or selector changes; `layout-padding.spec.ts` may need its
   padding assertions updated.
2. **Live data and titles** (S, 1 day): `useLiveList`, `<svelte:head>` titles, `h1`s, skip
   link, nav ARIA. Pure additions.
3. **Review surface v1 in place** (M, 3–5 days): on `/data/bullets` keep the route and the
   `Bullet Details` dialog name (the core-loop spec depends on both), move the modal onto
   `<Modal>`, make cards buttons, add row-level Approve/Reject, checkboxes and the bulk bar,
   unify the filter row. The spec's board drag still passes because the board is untouched.
4. **Settings page** (S–M, 2 days): `/settings` with tabs; `/config/*` and `/settings/*`
   keep working via redirects; trim the flyout.
5. **Source detail with derivations** (M, 3 days): derived-bullets section, `AgentPrompt`,
   "used in resumes". **Flagged:** surfacing wedged derivations needs a "list/cancel
   in-progress derivations" route in both runtimes — schedule with a parity-tier0 test so
   the job stays green.
6. **Inbox + Insights** (M, 3 days): `/` becomes Inbox (pending, drift, locks, activity);
   charts move to `/insights`; `dashboard.spec.ts` updated.
7. **IA move** (L, 1 week): new `nav.ts`, `/content`, `/opportunities/*`, Organizations
   merge, redirects for every old path (same pattern as `/bullets` → `/data/bullets`),
   `nav.test.ts` and `sidebar.spec.ts` updated last; e2e paths flip in one commit after
   redirects prove out.
8. **Builder restructure** (L, 1–2 weeks): outline pane, `ContextRail` with Gaps/Match/Add,
   aggregated gaps, preview toggle. `resume-targeting.spec.ts` and the core-loop `Preview`
   tab assertion updated to the toggle.
9. **Responsive shell** (L, 1–2 weeks): collapsible sidebar (rail + sheet), stacked split
   panels below 1024, bottom bar on phones; add a 390 px e2e sweep.
10. **Enforcement and retirement** (M, 2–3 days): make the grep checks blocking, delete
    unused components and legacy classes, fix the 90 `svelte-check` errors so `bun run
    check` can gate.

---

## 6. Open questions for the product owner

1. **Who drives derivation in the SaaS?** Today the agent is Claude in Chrome on
   `window.forge`. If the hosted product runs its own agent, "Ask agent to derive" becomes a
   real button with progress, not a copyable prompt — that changes the source-detail and
   Inbox designs materially.
2. **Is phone review in scope?** It decides whether B2 is a blocker or a later milestone.
3. **Does the five-status model stay for bullets/perspectives?** If Draft is only ever
   agent-written, Inbox can treat Draft+In Review as one queue and the boards lose two
   columns.
4. **Organizations vs. targeting pipeline:** is the Backlog/Researching/Targeting/Excluded
   board a property of a company record (merge) or a separate "campaign" object (keep,
   rename)?
5. **Workspaces:** one job-seeker with many resumes, or coaches/agencies managing many
   candidates? Determines whether the left nav needs a workspace switcher from day one.
6. **Which charts have an owner question?** If none, Insights can be a single page or
   nothing; keeping seven charts on the home page is the costliest default in the app.
7. **Must Chain View survive as a page**, or is per-entity provenance sufficient?
8. **Is `metrics` meant to be required on commit?** The strictness is what wedged the
   source; if it is intentional, the UI needs the lock indicator and a cancel route
   regardless.
9. **Brand colour:** is changing `--color-primary` acceptable to reach AA on primary
   buttons, or should button text/size change instead?
10. **Copy stance:** About/Docs currently say "local-first, no authentication, single user".
    Is that the OSS distribution's voice, and does the SaaS build get different copy?
