# Research: Rust LaTeX/Typesetting Library Evaluation

**Date**: 2026-04-23
**Status**: Complete
**Context**: forge-9fwl — IR Compiler, zero-subprocess constraint

## TL;DR

Of 12 crates evaluated, **only 2 can compile documents to PDF**: tectonic and typst. The other 10 are either subprocess wrappers, format converters, LaTeX generators, or incomplete/nonexistent.

## Crate Categories

### Full Document Compilers (the only viable options)

| Crate | Can compile LaTeX→PDF? | Pure Rust? | Binary size | Compile speed |
|-------|----------------------|-----------|-------------|---------------|
| **tectonic** | Yes (full XeTeX) | No (C engine) | ~50-100MB | 1-5s |
| **typst** | No (own markup only) | Yes | ~15-25MB | ~100ms |

### LaTeX Generators (produce .tex strings, not PDFs)

| Crate | What it does | Relevant? |
|-------|-------------|-----------|
| **latex-rs** | Builder DSL for LaTeX source | No — Forge already has its own LaTeX generation |
| **pgfplots-rs** | TikZ chart code generation | No — resumes don't have charts |

### Format Converters (single-purpose parsers)

| Crate | What it does | Relevant? |
|-------|-------------|-----------|
| **biblatex** | .bib file parser (by Typst team) | No — resumes don't have bibliographies |
| **pulldown-latex** | LaTeX math → MathML | No — resumes don't have equations |
| **mitex** | LaTeX math → Typst math | No — only useful for Typst migration of math |

### Non-starters

| Crate | Why disqualified |
|-------|-----------------|
| **latexcompile** | Subprocess wrapper by design. Stale (2020). |
| **listings-rust** | LaTeX style file, not a Rust crate. For code listings. |
| **oak-tex** | Not found on crates.io. Likely nonexistent. |
| **rustex** | Experimental TeX reimplementation. Cannot handle real packages. |
| **texcore** | Unknown/very early. Not a usable engine. |

## Deep Comparison: tectonic vs typst

### tectonic

- **Pros**: Full XeTeX compatibility, existing templates work unchanged, proven in production (Overleaf), handles all LaTeX packages (titlesec, enumitem, hyperref, geometry, fontspec)
- **Cons**: Large binary (~50-100MB from C engine + ICU + HarfBuzz), complex sub-crate structure (10+ interdependent crates), poorly documented library API, C compilation adds build complexity, needs TeX package bundle (network or pre-cached)
- **Library crates**: `tectonic_engine_xetex`, `tectonic_bridge_core`, `tectonic_io_base`, `tectonic_docmodel`

### typst

- **Pros**: Pure Rust (no C), 10-50x faster compilation, 60-75% smaller binary, clean API, actively funded (Typst GmbH), 30k+ GitHub stars
- **Cons**: **Cannot process LaTeX** — uses its own markup language, all templates must be rewritten, library API not yet stable (changes between versions)

## Recommendation

### Phase 1 (R0-R1): tectonic compiled-in

Preserves existing LaTeX templates. The `tectonic_engine_xetex` crate provides the compilation engine. Integration challenges:
- Pre-bundle TeX support files (fonts, .sty packages) for offline operation
- Handle the multi-crate dependency graph
- Accept the binary size increase

### Phase 2 (future): typst migration

Once the Rust port is stable, rewrite templates from LaTeX → Typst:
- Dramatic performance improvement
- Smaller binary
- No C compilation dependency
- But: template rewrite is a project in itself

### The fundamental trade-off

No crate exists that compiles LaTeX with packages as pure Rust. LaTeX compatibility requires the C-based TeX engine (tectonic). Pure Rust means Typst, which means non-LaTeX templates. Pick one axis.

## Feature Gap Summary

| Need | tectonic | typst | Others |
|------|---------|-------|--------|
| Compile LaTeX → PDF | ✓ | ✗ | ✗ |
| Compile any markup → PDF | ✓ (LaTeX) | ✓ (Typst) | ✗ |
| Zero subprocess | ✓ (compiled-in) | ✓ (native) | latexcompile: ✗ |
| Existing template compat | ✓ | ✗ (rewrite) | ✗ |
| Pure Rust | ✗ (C engine) | ✓ | N/A |
| Production users | Moderate | Large | Minimal |
