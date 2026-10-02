# @forge/extension

Browser extension for Forge. Capture job descriptions and autofill applications
into the Forge resume builder.

## Status

**Prototype P1** — pure extraction proof. Works on LinkedIn job pages, displays
extracted data in an in-page debug modal. No backend integration yet (added in P4).

See `.agents/plans/forge-resume-browser-extension/SPEC.md` for the full design.

## Development

From the monorepo root:

```bash
bun install
cd packages/extension
bun test           # run plugin unit tests
bun run build      # build to dist/
```

Then load `packages/extension/dist/` as an unpacked extension in
`chrome://extensions` (enable Developer mode first).

## Architecture (P1)

- **Popup** (`src/popup/Popup.svelte`) — single "Extract Job" button
- **Content script** (`src/content/linkedin.ts`) — injected on demand via
  `chrome.scripting.executeScript` when the popup button is clicked. No
  auto-injection, zero cost on LinkedIn page loads.
- **Plugin** (`src/plugin/plugins/linkedin.ts`) — pure extraction logic that
  reads DOM and returns a structured `ExtractedJob` object. Tested against
  real LinkedIn fixtures.
- **Debug modal** (`src/content/shared/modal.ts`) — displays extracted fields
  in-page. **Removed in P4** when capture wires through to Forge.

## Supported sites

| Site | Extract JD | Extract Org | Autofill |
|------|-----------|-------------|----------|
| LinkedIn | Partial (P1) | — | — |

**Known limitations**: LinkedIn plugin currently extracts title, company, and
description reliably but struggles with location and salary across some SDUI
chip layouts. Tracked in bead `job-hunting-3bp.12`; fix before P4.

## Tests

```bash
bun test tests/plugins/
```

9 tests covering plugin registry (5) and LinkedIn plugin (4: extraction from
real fixture, empty document, search listing, metadata).
