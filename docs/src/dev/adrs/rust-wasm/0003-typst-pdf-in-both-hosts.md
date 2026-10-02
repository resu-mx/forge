# 0003. PDFs with Typst, in both hosts (roadmap M5)

- **Status**: Accepted (Chrome verified; Firefox and Safari not yet verified)
- **Date**: 2026-10-02
- **Builds on**: [0002](./0002-browser-runtime-ownership-and-transport.md)
- **Roadmap**: `.agents/plans/rust-mvp-roadmap.md`, milestone M5

## Context

The TypeScript server makes PDFs by shelling out to `tectonic`, which a browser cannot do.
M5 needs one PDF path that works natively and inside the browser runtime.

## Decisions

### 1. Typst, from one crate (`forge-typst`)

`forge-sdk` renders the resume IR as Typst (`render_typst`, a port of the sb2nov layout).
`forge-typst` compiles that source to PDF in-process, with no subprocess and no network. The
native server uses it directly; the browser loads it as a Typst WASM module.

### 2. User text is only ever a string literal

Every user-supplied value is emitted as an escaped Typst string literal passed to a helper
(`#sub("Acme", "", "Eng", "2020")`), never as markup or code. The compiler is given no file
resolver, so a document cannot read files.

### 3. A separate, lazily loaded module in the browser

The compiler is **25.4 MB raw, 9.5 MB gzipped**, against 2.9 MB for the data layer. It is its own
Worker and WASM module, fetched when a PDF is first asked for (about 240 ms for the first PDF
including that load, about 35 ms after, in Chrome). The data layer's size budget is unaffected.
The preview says that a download is happening.

`forge-api` has a `pdf` feature. The server turns it on; the browser build leaves it off, so the
router there answers `501` and points at `?format=typst`. The browser client intercepts the two
PDF routes, fetches the Typst source from the database Worker, and compiles it in the Typst Worker.

### 4. Fonts

New Computer Modern 10 (regular, bold, italic, bold italic), embedded with `include_bytes!`
(SIL OFL; notice in `crates/forge-typst/fonts/`). No system fonts are consulted, so output is
the same on every host.

### 5. `latex_override` is not compiled

LaTeX cannot run in a browser. A resume with a non-empty override still gets a PDF, generated
from its content, with an `X-Forge-Pdf-Notice` header that the preview shows as a banner. A
`typst_override` column is follow-up work.

### 6. Errors

A document that does not compile is `422 TYPST_COMPILE_ERROR` with the diagnostics in `details`.
A browser compiler that cannot run is `500 PDF_UNAVAILABLE`; a build with no compiler is
`501 NOT_IMPLEMENTED`.

## Consequences

- The TS server still uses tectonic; it is untouched and keeps working.
- `wasm-bindgen-cli` must match the version in `Cargo.lock` (0.2.129) for `just typst-bundle`.
- 25 MB is heavy on a slow connection; it is a one-time cost per browser, cached afterwards.
