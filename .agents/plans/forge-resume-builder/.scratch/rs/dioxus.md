This is a broad research question that covers multiple facets. Following the skill's Step 1, I should assess whether to decompose. The question "what should the Dioxus skill directory tree look like" has 3 distinct sub-questions:

  1. What topics does Dioxus 0.7 actually cover? (completeness — what exists in the docs)
  2. What do practitioners actually struggle with? (relevance — what deserves reference files)
  3. How do other framework skills/docs organize their content? (structure — organizational patterns)

  These need different engines and different query strategies. I'll decompose into 3 matrices.

  Search Matrix: Dioxus 0.7 Skill Directory Structure Research

  Decomposition

  This research spans 3 sub-questions with different engine needs:

  1. What is the complete topic surface of Dioxus 0.7? — Survey the full documentation to identify every distinct topic area
    - Domain: tech, Type: survey
    - Depends on: none
  2. What do Dioxus practitioners struggle with most? — Find pain points, common questions, and gaps
    - Domain: tech, Type: survey
    - Depends on: none
  3. How do similar framework skills organize reference content? — Look at existing skill structures for React, Svelte, Flutter, Leptos
    - Domain: tech, Type: comparison
    - Depends on: none

  Execution order: All 3 in parallel — independent questions.

  ---
  Matrix 1: Dioxus 0.7 Complete Topic Surface

  Context

  - Goal: Map every distinct topic area in Dioxus 0.7 to determine what reference files are needed
  - Type: survey
  - Domain: tech

  Tier 1: Primary (high-precision)

  ┌─────┬────────────────────┬───────────────────────────────────────────────────┬──────────────────────────────────────┬────────────────────────────────────────┬─────────────────────────────────────┬───────────────────────────────────────────────────────────────────┐
  │  #  │     Engine(s)      │                       Query                       │              Operators               │            Expected Results            │             Acceptance              │                              Success                              │
  ├─────┼────────────────────┼───────────────────────────────────────────────────┼──────────────────────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ 1   │ Google             │ dioxus 0.7 documentation site map                 │ site:dioxuslabs.com                  │ Full doc structure with all pages      │ Complete sitemap or nav tree        │ Every page URL in the 0.7 docs                                    │
  ├─────┼────────────────────┼───────────────────────────────────────────────────┼──────────────────────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ 2   │ GitHub             │ dioxus docs directory structure                   │ repo:DioxusLabs/dioxus path:docs     │ Source files for the documentation     │ Doc source tree visible             │ Full list of doc pages with categorization                        │
  ├─────┼────────────────────┼───────────────────────────────────────────────────┼──────────────────────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ 3   │ crates.io, docs.rs │ dioxus                                            │ dioxus crate page                    │ Module list, feature flags, re-exports │ Module names and public API surface │ Complete list of modules: hooks, router, fullstack, signals, etc. │
  ├─────┼────────────────────┼───────────────────────────────────────────────────┼──────────────────────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ 4   │ Google             │ dioxus 0.7 "what's new" OR changelog OR migration │ site:dioxuslabs.com after:2025-01-01 │ Release notes showing all features     │ Feature list for 0.7                │ Comprehensive changelog with every new API                        │
  └─────┴────────────────────┴───────────────────────────────────────────────────┴──────────────────────────────────────┴────────────────────────────────────────┴─────────────────────────────────────┴───────────────────────────────────────────────────────────────────┘

  Tier 2: Broadened

  ┌─────┬───────────┬────────────────────────────────┬──────────────────────────────────────┬────────────────────────────────────────┬─────────────────────────────────┬─────────────────────────────────────────────────────────┐
  │  #  │ Engine(s) │             Query              │              Operators               │            Expected Results            │           Acceptance            │                         Success                         │
  ├─────┼───────────┼────────────────────────────────┼──────────────────────────────────────┼────────────────────────────────────────┼─────────────────────────────────┼─────────────────────────────────────────────────────────┤
  │ 1   │ Google    │ dioxus guide tutorial cookbook │ site:dioxuslabs.com                  │ Guide/tutorial content beyond API docs │ Additional topic areas          │ Cookbook recipes revealing non-obvious topics           │
  ├─────┼───────────┼────────────────────────────────┼──────────────────────────────────────┼────────────────────────────────────────┼─────────────────────────────────┼─────────────────────────────────────────────────────────┤
  │ 2   │ GitHub    │ dioxus examples                │ repo:DioxusLabs/dioxus path:examples │ Example apps showing feature usage     │ Example names indicating topics │ Example list with categories (routing, auth, ssr, etc.) │
  └─────┴───────────┴────────────────────────────────┴──────────────────────────────────────┴────────────────────────────────────────┴─────────────────────────────────┴─────────────────────────────────────────────────────────┘

  Tier 3: Alternative sources

  ┌─────┬───────────┬───────────────────┬────────────┬────────────────────────────────┬───────────────────────┬────────────────────────────────────────────────────┐
  │  #  │ Engine(s) │       Query       │ Operators  │        Expected Results        │      Acceptance       │                      Success                       │
  ├─────┼───────────┼───────────────────┼────────────┼────────────────────────────────┼───────────────────────┼────────────────────────────────────────────────────┤
  │ 1   │ DeepWiki  │ DioxusLabs/dioxus │ direct URL │ AI-generated architecture docs │ Module dependency map │ Internal architecture showing component boundaries │
  └─────┴───────────┴───────────────────┴────────────┴────────────────────────────────┴───────────────────────┴────────────────────────────────────────────────────┘

  Grading Summary

  - Tier 1 acceptance: >= 20 distinct topic areas identified
  - Tier 1 success: Complete topic taxonomy matching or exceeding the user's rough outline
  - Overall success: Every Dioxus 0.7 feature area mapped to a potential reference file

  ---
  Matrix 2: Dioxus Practitioner Pain Points

  Context

  - Goal: Find what Dioxus users struggle with to prioritize which topics deserve detailed reference files
  - Type: survey
  - Domain: tech

  Tier 1: Primary (high-precision)

  ┌─────┬───────────────┬────────────────────────────────────────────────┬──────────────────────────────────────────────────────┬────────────────────────────────┬────────────────────────────┬─────────────────────────────────────────────────────────┐
  │  #  │   Engine(s)   │                     Query                      │                      Operators                       │        Expected Results        │         Acceptance         │                         Success                         │
  ├─────┼───────────────┼────────────────────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────────────┼────────────────────────────┼─────────────────────────────────────────────────────────┤
  │ 1   │ GitHub        │ dioxus issues                                  │ repo:DioxusLabs/dioxus is:issue label:bug is:open    │ Open bugs showing pain points  │ >= 10 categorizable issues │ Issue categories revealing which features cause trouble │
  ├─────┼───────────────┼────────────────────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────────────┼────────────────────────────┼─────────────────────────────────────────────────────────┤
  │ 2   │ StackOverflow │ dioxus                                         │ [dioxus] score:1                                     │ Practitioner questions         │ >= 5 answered questions    │ Question topics showing common confusion areas          │
  ├─────┼───────────────┼────────────────────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────────────┼────────────────────────────┼─────────────────────────────────────────────────────────┤
  │ 3   │ Google        │ dioxus "how to" OR "struggling with" OR "help" │ site:discord.com OR site:reddit.com after:2024-01-01 │ Community help requests        │ >= 5 distinct help topics  │ Pattern of repeated questions indicating doc gaps       │
  ├─────┼───────────────┼────────────────────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────────────┼────────────────────────────┼─────────────────────────────────────────────────────────┤
  │ 4   │ GitHub        │ dioxus discussions                             │ repo:DioxusLabs/dioxus is:discussion                 │ Community discussions on usage │ >= 5 discussions           │ Topics where people need guidance beyond docs           │
  └─────┴───────────────┴────────────────────────────────────────────────┴──────────────────────────────────────────────────────┴────────────────────────────────┴────────────────────────────┴─────────────────────────────────────────────────────────┘

  Tier 2: Broadened

  ┌─────┬───────────┬────────────────────────────────────────────────────────┬──────────────────┬─────────────────────────────────┬───────────────┬──────────────────────────────────────────────────┐
  │  #  │ Engine(s) │                         Query                          │    Operators     │        Expected Results         │  Acceptance   │                     Success                      │
  ├─────┼───────────┼────────────────────────────────────────────────────────┼──────────────────┼─────────────────────────────────┼───────────────┼──────────────────────────────────────────────────┤
  │ 1   │ Google    │ dioxus "known issues" OR limitations OR "doesn't work" │ after:2024-01-01 │ Blog posts about limitations    │ >= 2 articles │ Specific limitations documented by practitioners │
  ├─────┼───────────┼────────────────────────────────────────────────────────┼──────────────────┼─────────────────────────────────┼───────────────┼──────────────────────────────────────────────────┤
  │ 2   │ Google    │ site:reddit.com/r/rust dioxus                          │ site:reddit.com  │ r/rust discussions about Dioxus │ >= 3 threads  │ Threads revealing real-world usage challenges    │
  └─────┴───────────┴────────────────────────────────────────────────────────┴──────────────────┴─────────────────────────────────┴───────────────┴──────────────────────────────────────────────────┘

  Tier 3: Alternative sources

  ┌─────┬───────────┬──────────────────────────────────┬───────────────────────────┬───────────────────────────────────────────┬────────────────────────┬───────────────────────────────────────────────────┐
  │  #  │ Engine(s) │              Query               │         Operators         │             Expected Results              │       Acceptance       │                      Success                      │
  ├─────┼───────────┼──────────────────────────────────┼───────────────────────────┼───────────────────────────────────────────┼────────────────────────┼───────────────────────────────────────────────────┤
  │ 1   │ Google    │ site:news.ycombinator.com dioxus │ site:news.ycombinator.com │ HN discussions with practitioner opinions │ Any substantive thread │ Thread with practitioners sharing real experience │
  └─────┴───────────┴──────────────────────────────────┴───────────────────────────┴───────────────────────────────────────────┴────────────────────────┴───────────────────────────────────────────────────┘

  Grading Summary

  - Tier 1 acceptance: >= 8 distinct pain point categories identified
  - Tier 1 success: Prioritized list of topics by frequency/severity of questions
  - Overall success: Clear signal on which reference files need the most depth

  ---
  Matrix 3: Framework Skill Organization Patterns

  Context

  - Goal: Study how other framework docs/skills organize content to inform our directory structure
  - Type: comparison
  - Domain: tech

  Tier 1: Primary (high-precision)

  ┌─────┬───────────┬─────────────────────────────────────────────────────┬─────────────────────────────────────────────┬────────────────────────────────────────────┬───────────────────────────┬────────────────────────────────────────────┐
  │  #  │ Engine(s) │                        Query                        │                  Operators                  │              Expected Results              │        Acceptance         │                  Success                   │
  ├─────┼───────────┼─────────────────────────────────────────────────────┼─────────────────────────────────────────────┼────────────────────────────────────────────┼───────────────────────────┼────────────────────────────────────────────┤
  │ 1   │ Google    │ leptos rust framework documentation structure       │ site:leptos.dev OR site:leptos-rs.github.io │ Leptos docs nav (closest Rust competitor)  │ Doc section list          │ Full nav tree showing topic organization   │
  ├─────┼───────────┼─────────────────────────────────────────────────────┼─────────────────────────────────────────────┼────────────────────────────────────────────┼───────────────────────────┼────────────────────────────────────────────┤
  │ 2   │ Google    │ react documentation table of contents               │ site:react.dev                              │ React docs nav (gold standard)             │ Section list              │ How React organizes: learn, reference, API │
  ├─────┼───────────┼─────────────────────────────────────────────────────┼─────────────────────────────────────────────┼────────────────────────────────────────────┼───────────────────────────┼────────────────────────────────────────────┤
  │ 3   │ Google    │ svelte documentation structure                      │ site:svelte.dev                             │ Svelte docs nav                            │ Section list              │ Svelte's tutorial vs API reference split   │
  ├─────┼───────────┼─────────────────────────────────────────────────────┼─────────────────────────────────────────────┼────────────────────────────────────────────┼───────────────────────────┼────────────────────────────────────────────┤
  │ 4   │ GitHub    │ content/skills/lang-rust-dev OR lang-typescript-dev │ repo:arustydev/agents path:content/skills   │ Existing skills in this repo for reference │ Skill directory structure │ How our existing lang skills organize      │
  └─────┴───────────┴─────────────────────────────────────────────────────┴─────────────────────────────────────────────┴────────────────────────────────────────────┴───────────────────────────┴────────────────────────────────────────────┘

  Tier 2: Broadened

  ┌─────┬───────────┬───────────────────────────────────────────────────────────────────────────┬───────────────────────┬─────────────────────────────────────────────────────┬────────────────────┬─────────────────────────────────────────────────────────┐
  │  #  │ Engine(s) │                                   Query                                   │       Operators       │                  Expected Results                   │     Acceptance     │                         Success                         │
  ├─────┼───────────┼───────────────────────────────────────────────────────────────────────────┼───────────────────────┼─────────────────────────────────────────────────────┼────────────────────┼─────────────────────────────────────────────────────────┤
  │ 1   │ Google    │ flutter documentation categories                                          │ site:docs.flutter.dev │ Flutter docs structure (cross-platform like Dioxus) │ Section categories │ Multi-platform doc organization pattern                 │
  ├─────┼───────────┼───────────────────────────────────────────────────────────────────────────┼───────────────────────┼─────────────────────────────────────────────────────┼────────────────────┼─────────────────────────────────────────────────────────┤
  │ 2   │ Google    │ "documentation architecture" OR "docs structure" framework best practices │ after:2023-01-01      │ Meta-advice on organizing framework docs            │ >= 1 guide         │ Principles for progressive disclosure in framework docs │
  └─────┴───────────┴───────────────────────────────────────────────────────────────────────────┴───────────────────────┴─────────────────────────────────────────────────────┴────────────────────┴─────────────────────────────────────────────────────────┘

  Grading Summary

  - Tier 1 acceptance: >= 3 framework doc structures analyzed
  - Tier 1 success: Common organizational patterns identified across frameworks
  - Overall success: Recommended directory structure informed by proven patterns

  ---
  Runtime Recovery

  - Decompose: Split "topic surface" into sub-areas (UI, state, fullstack, platform) if the full survey is too broad
  - Pivot terminology: "dioxus reference", "dioxus API", "dioxus cookbook", "dioxus recipes"
  - Escalate: Ask the user which of their rough outline directories are firm vs negotiable

  Now let me execute these matrices. I'll dispatch 3 research agents in parallel.

⏺ 3 agents launched (ctrl+o to expand)
   ├─ Matrix 1: Dioxus topic surface
   │  ⎿  Running in the background
   ├─ Matrix 2: Dioxus pain points
   │  ⎿  Running in the background
   └─ Matrix 3: Framework doc patterns
      ⎿  Running in the background

⏺ 3 research matrices executing in parallel:

  1. Topic Surface — mapping every Dioxus 0.7 feature area from docs, repo, and crate modules
  2. Pain Points — GitHub issues, StackOverflow, Reddit, HN for practitioner struggles
  3. Doc Patterns — Leptos, React, Svelte, Flutter organizational structures + existing skill patterns

  I'll synthesize the findings into a recommended directory tree once all three complete. Waiting for results.
  
  Matrix 1 is still running — actively searching Dioxus 0.7 doc pages. While it finishes, I already have enough from matrices 2 and 3 to synthesize the recommended directory tree. Let me draft it now, and refine when matrix 1 completes.

  Synthesized Directory Tree

  Merging Matrix 2 (pain points → prioritization) with Matrix 3 (framework patterns → structure) and your rough outline:

  content/skills/lang/rust/dioxus/
  ├── SKILL.md                              # Overview, quick start, dx CLI, core concepts
  │
  ├── references/
  │   ├── cli.md                            # dx new, serve, build, bundle, doctor, hotpatch
  │   │
  │   ├── ui/
  │   │   ├── rsx-syntax.md                 # RSX macro, elements, attributes, text interpolation
  │   │   ├── conditional-rendering.md      # if/else, match, Option in RSX
  │   │   ├── lists-and-keys.md             # for loops, key attribute, reconciliation
  │   │   ├── components.md                 # Defining, props (#[component] vs Props struct), children
  │   │   ├── event-handlers.md             # onclick, oninput, async handlers, form patterns
  │   │   ├── styling.md                    # CSS, Tailwind, scoped styles, FOUC workaround
  │   │   ├── assets.md                     # asset!() macro, images, optimization, fonts
  │   │   └── hot-reload.md                 # Hot-reload vs hot-patch, limitations, workspace issues
  │   │
  │   ├── state/
  │   │   ├── signals.md                    # use_signal, read/write, Signal/ReadSignal, GlobalSignal
  │   │   ├── hooks.md                      # use_hook, rules of hooks, custom hooks
  │   │   ├── derived-state.md              # use_memo, computed values
  │   │   ├── effects.md                    # use_effect, side effects
  │   │   ├── async.md                      # use_resource, use_action, use_future, spawn, cancel safety
  │   │   ├── context.md                    # use_context, use_context_provider, TypeId gotcha
  │   │   ├── error-handling.md             # Error boundaries, Result propagation
  │   │   └── troubleshooting.md            # "Already Borrowed" panics, hook rule violations, signal gotchas
  │   │
  │   ├── routing/
  │   │   ├── definition.md                 # Routable derive, route types, segments, query/hash
  │   │   ├── navigation.md                 # Link component, programmatic nav
  │   │   └── layouts.md                    # Nested routes, #[nest], #[layout], #[end_nest]
  │   │
  │   ├── fullstack/                        # P0 priority — deepest depth
  │   │   ├── setup.md                      # Cargo.toml features, Dioxus.toml, client/server split
  │   │   ├── server-functions.md           # #[get], #[post], #[server], extractors, path/query params
  │   │   ├── ssr.md                        # Server-side rendering, hydration, hydration gotchas
  │   │   ├── streaming.md                  # HTML streaming, commit_initial_chunk, suspense
  │   │   ├── websockets.md                 # WebSocket endpoints, typed sockets, use_websocket, encodings
  │   │   ├── streams-and-sse.md            # TextStream, ByteStream, Streaming<T>, FileStream
  │   │   ├── authentication.md             # Middleware pattern, session extractors, auth libraries
  │   │   ├── native-clients.md             # Desktop/mobile as fullstack clients, set_server_url, API versioning
  │   │   ├── middleware.md                 # Axum middleware, custom router, layer pattern
  │   │   ├── forms.md                      # Form handling, multipart, onsubmit
  │   │   └── axum-integration.md           # Integrating into existing Axum apps (the #1 pain point)
  │   │
  │   ├── platforms/
  │   │   ├── web.md                        # WASM target, web-sys, JS interop, eval
  │   │   ├── desktop.md                    # Window management, native menus, Wayland/X11, WebView2
  │   │   ├── mobile.md                     # iOS/Android setup, permissions gap, known issues
  │   │   └── static-generation.md          # SSG configuration and deployment
  │   │
  │   ├── testing/
  │   │   ├── component-testing.md          # Testing components, simulating events
  │   │   └── server-testing.md             # Testing server functions
  │   │
  │   ├── deployment/
  │   │   ├── bundling.md                   # dx bundle, AppImage, MSI, DMG, WASM optimization
  │   │   └── hosting.md                    # Fly.io, Vercel, static hosting
  │   │
  │   ├── debugging.md                      # DevTools, tracing, common error messages
  │   ├── js-interop.md                     # eval, web-sys, dynamically loaded scripts
  │   ├── migration.md                      # 0.5→0.6→0.7 migration guide
  │   │
  │   └── patterns/
  │       ├── anti-patterns.md              # Common mistakes and how to fix them
  │       └── recipes.md                    # Practical patterns: auth flow, data loading, optimistic UI

  Key decisions informed by the research:

  ┌───────────────────────────────────────────────┬───────────────────────────────────────────────────────────────────┐
  │                   Decision                    │                              Source                               │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ fullstack/ gets 11 files (deepest)            │ Matrix 2: P0 pain point, 25+ open issues, docs actively fail here │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ state/troubleshooting.md exists               │ Matrix 2: "Already Borrowed" panics are the most visceral bug     │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ fullstack/axum-integration.md is separate     │ Matrix 2: #1 user complaint per #4011 "Death by 1000 Papercuts"   │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ ui/hot-reload.md exists                       │ Matrix 2: ~75 CLI issues, hot-reload limitations are major        │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ platforms/mobile.md notes the permissions gap │ Matrix 2: #3870 is the most-discussed mobile issue                │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ Flat core concepts, nested thematic clusters  │ Matrix 3: React/Flutter/Svelte all use this pattern               │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ No separate tutorial directory                │ Matrix 3: SKILL.md serves as entry point per repo convention      │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ patterns/ instead of cookbook/                │ Matrix 3: Aligns with our existing Dolt/git-cliff skill patterns  │
  ├───────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────┤
  │ testing/ and deployment/ are small            │ Matrix 2: P2 priority, not major pain points yet                  │
  └───────────────────────────────────────────────┴───────────────────────────────────────────────────────────────────┘

  Phase plan:

  ┌───────┬──────────────────────────────────────────────────────┬───────────┬────────────────────────────────┐
  │ Phase │                     Directories                      │   Files   │         Priority basis         │
  ├───────┼──────────────────────────────────────────────────────┼───────────┼────────────────────────────────┤
  │ 1     │ SKILL.md + ui/ + state/ + fullstack/                 │ ~30 files │ Core dev loop + P0 pain points │
  ├───────┼──────────────────────────────────────────────────────┼───────────┼────────────────────────────────┤
  │ 2     │ routing/ + platforms/ + debugging.md + js-interop.md │ ~8 files  │ P1 pain points                 │
  ├───────┼──────────────────────────────────────────────────────┼───────────┼────────────────────────────────┤
  │ 3     │ testing/ + deployment/ + migration.md + patterns/    │ ~6 files  │ P2/P3, cross-cutting           │
  └───────┴──────────────────────────────────────────────────────┴───────────┴────────────────────────────────┘

  Does this structure work? Any directories to add, remove, or reorganize before I start Phase 1?
  
  Revised Directory Tree

  Matrix 1 surfaced ~108 topics but a 130+ file skill is impractical. The key insight from Matrix 3 is: merge related topics into cohesive reference files rather than 1:1 file-per-topic. Matrix 2 tells us where to go deep vs shallow.

  Here's the refined tree, incorporating all three matrices:

  content/skills/lang/rust/dioxus/
  ├── SKILL.md                                    # Quick start, dx CLI basics, core concepts overview
  │
  ├── references/
  │   ├── cli.md                                  # dx new/serve/build/bundle/doctor, Dioxus.toml config
  │   │
  │   ├── ui/
  │   │   ├── rsx-syntax.md                       # RSX macro, elements, attributes, text interpolation,
  │   │   │                                       #   spread props, shorthand, SVG, escaping RSX
  │   │   ├── components.md                       # Defining, #[component], Props struct, children,
  │   │   │                                       #   memoization, generic components, lifecycle
  │   │   ├── rendering.md                        # Conditional rendering, lists/keys, dynamic rendering,
  │   │   │                                       #   reconciliation model
  │   │   ├── event-handlers.md                   # Mouse/keyboard/form events, async handlers,
  │   │   │                                       #   EventHandler<T>, form input patterns
  │   │   ├── styling.md                          # CSS, Tailwind (auto-detected), scoped styles,
  │   │   │                                       #   FOUC workaround, document::Stylesheet
  │   │   ├── assets.md                           # asset!() macro, manganis, images, fonts,
  │   │   │                                       #   optimization (avif), document API (head/meta/title)
  │   │   └── hot-reload.md                       # RSX hot-reload, Rust hot-patching (subsecond),
  │   │                                           #   limitations, workspace issues, dx fmt
  │   │
  │   ├── state/
  │   │   ├── signals.md                          # use_signal, read/write, Signal/ReadSignal/WriteSignal,
  │   │   │                                       #   GlobalSignal, reactivity model, stores/collections
  │   │   ├── hooks.md                            # use_hook, rules of hooks, custom hooks, use_memo,
  │   │   │                                       #   use_effect, built-in hook reference
  │   │   ├── async.md                            # use_resource (cancel safety!), use_action, use_future,
  │   │   │                                       #   use_loader, spawn, coroutines, suspense
  │   │   ├── context.md                          # use_context, use_context_provider, TypeId gotcha,
  │   │   │                                       #   error handling, error boundaries
  │   │   └── troubleshooting.md                  # P0: "Already Borrowed" panics, hook rule violations,
  │   │                                           #   signals in closures, autofill not triggering,
  │   │                                           #   writing during render warnings
  │   │
  │   ├── routing/
  │   │   ├── definition.md                       # Routable derive, segment types (static/dynamic/
  │   │   │                                       #   catch-all/query/hash), #[nest]/#[end_nest]
  │   │   ├── navigation.md                       # Link component, programmatic nav, scroll restoration
  │   │   └── layouts.md                          # Nested routes, #[layout], route-based code splitting
  │   │
  │   ├── fullstack/                              # P0 — deepest section per pain point analysis
  │   │   ├── setup.md                            # Cargo.toml features, Dioxus.toml, client/server split,
  │   │   │                                       #   #[cfg(feature)], workspace gotchas
  │   │   ├── server-functions.md                 # #[get]/#[post]/#[server], path/query extractors,
  │   │   │                                       #   server-only extractors, encoding formats, calling
  │   │   │                                       #   from client
  │   │   ├── ssr-and-hydration.md                # SSR, hydration, hydration gotchas (P0 pain point),
  │   │   │                                       #   streaming (commit_initial_chunk), suspense
  │   │   ├── websockets-and-streams.md           # WebSocket endpoints, typed sockets, use_websocket,
  │   │   │                                       #   TextStream, ByteStream, Streaming<T>, SSE, FileStream
  │   │   ├── authentication.md                   # Middleware pattern, session extractors, recommended
  │   │   │                                       #   libraries, auth flow recipe
  │   │   ├── forms.md                            # Form<T>, multipart upload, onsubmit, streaming upload
  │   │   ├── axum-integration.md                 # P0: integrating into existing Axum apps, custom router,
  │   │   │                                       #   middleware layers, server state
  │   │   └── native-clients.md                   # Desktop/mobile as fullstack clients, set_server_url,
  │   │                                           #   API versioning, disabled features
  │   │
  │   ├── platforms/
  │   │   ├── web.md                              # WASM target, web-sys, JS interop, eval, PWA
  │   │   ├── desktop.md                          # Window mgmt, menus, tray icons, shortcuts, drag-drop,
  │   │   │                                       #   Wayland/X11 issues, WebView2
  │   │   └── mobile.md                           # iOS/Android setup, permissions gap (P1 pain point),
  │   │                                           #   known issues, APK signing, simulator setup
  │   │
  │   ├── testing.md                              # Component testing, Playwright web testing, server testing
  │   ├── deployment.md                           # Bundling (AppImage/MSI/DMG), wasm-opt, Fly.io, Docker,
  │   │                                           #   GitHub Pages, static site generation
  │   ├── js-interop.md                           # eval, web-sys, dynamic scripts, web components
  │   ├── debugging.md                            # DevTools, tracing, common error messages, dx doctor
  │   ├── migration.md                            # 0.5→0.6→0.7 breaking changes, new APIs, upgrade path
  │   │
  │   └── patterns/
  │       ├── anti-patterns.md                    # From Matrix 2: hook rule violations, branch-per-render,
  │       │                                       #   server deps in WASM, async handler foot-guns
  │       └── recipes.md                          # Auth flow, optimistic UI, data loading, global state,
  │                                               #   performance optimization, bundle size reduction

  File count by phase:

  ┌───────┬────────────────────────────────────────────────────────┬───────┬───────┐
  │ Phase │                        Section                         │ Files │ Total │
  ├───────┼────────────────────────────────────────────────────────┼───────┼───────┤
  │ 1     │ SKILL.md + ui/ (7) + state/ (5) + fullstack/ (8)       │ 21    │ 21    │
  ├───────┼────────────────────────────────────────────────────────┼───────┼───────┤
  │ 2     │ routing/ (3) + platforms/ (3) + debugging + js-interop │ 8     │ 29    │
  ├───────┼────────────────────────────────────────────────────────┼───────┼───────┤
  │ 3     │ testing + deployment + migration + patterns/ (2)       │ 5     │ 34    │
  └───────┴────────────────────────────────────────────────────────┴───────┴───────┘

  What got merged (108 topics → 34 files):

  - RSX elements, attributes, spread, shorthand, SVG → single rsx-syntax.md
  - Signals, reactivity, stores/collections → single signals.md
  - use_resource, use_action, use_future, coroutines, suspense → single async.md
  - SSR + hydration + streaming → single ssr-and-hydration.md
  - WebSockets + streams + SSE → single websockets-and-streams.md
  - Window mgmt, menus, tray, shortcuts, drag-drop → single desktop.md
  - All testing → single testing.md
  - All deployment → single deployment.md

  Does this structure work? Want to adjust anything before I start Phase 1?
