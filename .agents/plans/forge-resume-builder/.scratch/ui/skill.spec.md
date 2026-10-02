# UI Skill

## Skill Layout
```
├── references
│   ├── concepts.md
│   ├── authentication.md
│   ├── hooks.md
│   ├── styling.md
│   ├── i18n.md
│   ├── debugging.md
│   ├── ui/
│   │   ├── styling.md
│   │   ├── assets.md
│   │   ├── custom-renderer.md
│   │   ├── introducing-rsx.md
│   │   ├── elements-and-text.md
│   │   ├── dynamic-attributes.md
│   │   ├── conditional-rendering.md
│   │   ├── rendering-lists.md
│   │   ├── components.md
│   │   ├── reconciliation.md
│   │   ├── assets.md
│   │   ├── styling.md
│   │   ├── hot-reload.md
│   │   └── escape-hatches.md
│   ├── state/
│   │   ├── project-setup.md
│   │   ├── intro-to-reactivity.md
│   │   ├── storing-state-in-hooks.md
│   │   ├── reactive-signals.md
│   │   ├── user-input.md
│   │   ├── async-and-futures.md
│   │   ├── data-fetching.md
│   │   ├── effects-and-memos.md
│   │   ├── hoisting-state.md
│   │   ├── global-context.md
│   │   ├── stores-and-collections.md
│   │   ├── error-handling.md
│   │   └── suspense.md
│   ├── routing/
│   │   ├── navigation.md
│   │   ├── layouts.md
│   │   └── defintion.md
│   ├── components/
│   │   └── lifecycle.md
│   ├── fullstack/
│   │   ├── project-setup.md
│   │   ├── ssr.md
│   │   ├── server-functions.md
│   │   ├── custom-error-pages.md
│   │   ├── router-and-state.md
│   │   ├── middleware.md
│   │   ├── websockets.md
│   │   ├── streams.md
│   │   ├── sse.md
│   │   ├── forms-and-multipart.md
│   │   ├── authentication.md
│   │   ├── native-clients.md
│   │   ├── html-streaming.md
│   │   └── static-site-generation.md
│   ├── platforms/
│   ├── publishing/
│   ├── testing/
│   └── o11y/
│       ├── logging.md
│       ├── tracing.md
│       └── metrics.md
└── SKILL.md
```
```
/
├── .config/
│   └── brewfile
├── .docs/
│   ├── ast-grep/
│   │   └── foo.rule.yaml
│   ├── src/
│   │   └── SUMMARY.md
│   └── book.toml
├── src/
│   ├── ast-grep/
│   │   └── foo.rule.yaml
│   ├── registry/
│   │   ├── anti-patterns.test.ts
│   ├── __tests__/
│   │   ├── anti-patterns.test.ts
│   └── 
├── references
│   ├── examples/
│   │   ├── css/
│   │   └── markup.html
│   ├── component/
│   │   ├── api/
│   │   │   ├── props/
│   │   │   ├── slots/
│   │   │   └── events/
│   └── 
├── sgconfig.yml
├── package.json
├── tsconfig.json
├── bunfig.toml
├── bun.lock
├── justfile
└── SKILL.md
```


## Considerations

- component API
    - Props
    - Slots
    - Events
    - implementer matches existing patterns.
- Behavior
- Design-tokens
    - CSS tokens (e.g., var(--space-3), var(--color-primary))
    - Implementer reads tokens.css for values.
    - token consumption/propogation strategy and reusability patterns
- Concrete CSS
    - pixel-level mockups
    - actual CSS properties with token references
    - implementer can copy-paste the styles and just wire up the markup.
- structural components
- visual/styling sections (visual pattern)
- variant/divergence control
- how to support branding
- progressive delivery patterns
- explicitly call out what is and is NOT allowed in the SPEC (goals, non-goals, acceptance/failure criteria)
- Migration/Adoption Patterns
    - Progressive adoption — Component is created, then each page is migrated one at a time in separate tasks. Old per-page CSS coexists temporarily. Slower but each page can be verified independently.
    - Create the component with tests, migrate one "reference page" (the simplest user of the pattern) to prove it works, then migrate the rest page-by-page. The spec should define the reference page for each component and the migration order.
- mechanism to catch regressions 
    - Ex: where someone adds a new page with the old inline pattern instead of using the shared component
    - Lint rule (grep-based) — A test that greps for anti-patterns
        - like height: calc(100vh - 4rem) or .btn-new { in page files. If found outside the shared component, the test fails. Simple, no tooling dependency.
    - Svelte compiler check — A custom preprocessor or ESLint plugin that flags inline CSS matching known anti-patterns. More robust but heavier setup.
    - AST-Grep
    

## Template Layout

1. Overview (what, why, which pages)
2. Component API (props, slots/snippets, events, types)
3. Styling (concrete CSS with token references + branding strategy)
4. Behavior (state management, accessibility, edge cases)
5. Adoption Strategy (reference page, migration order, coexistence rules)
6. Adoption Enforcement (grep tests + CLAUDE.md rules)
7. Examples (explicit "do this" + implicit "this is the pattern" + "DON'T do this")
8. Goals / Non-Goals
9. Acceptance Criteria / Failure Criteria
10. Testing (unit, component, visual, adoption enforcement)

Key principles embedded in every spec:
- Allowed/Not Allowed section explicitly lists what inline CSS or markup is forbidden once the component exists
- Migration checklist per page with before/after
- Reference implementation — first page migrated is called out, serves as the pattern for the rest
- Branding — all colors, spacing, and typography come from tokens, never hardcoded values


## Components

- PageWrapper: fixes the sticky kanban bug across all pages
- SplitPanel: standardizes 7 independent layout implementations
- ListPanelHeader: fixes the info vs primary button color inconsistency
- Global buttons: eliminates ~200 lines of duplicated styles across 10 files
- PageHeader
- TabBar
- EmptyPanel
- ListSearchInput
