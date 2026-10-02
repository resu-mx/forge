# Phase 46: LaTeX/XeTeX Compatibility Documentation

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-latex-xetex-compat.md](../refs/specs/2026-04-03-latex-xetex-compat.md)
**Depends on:** None
**Blocks:** None currently identified
**Parallelizable with:** Phase 44 (IR Data Quality), Phase 45 (Editor Restructuring), Phase 47 (Clearance Structured Data) -- documentation-only, no code changes

## Goal

Create a comprehensive reference document at `docs/latex-xetex-compatibility.md` covering the differences between pdflatex and XeTeX/tectonic, a step-by-step checklist for importing pdflatex templates into Forge, documentation of the sb2nov template adaptations already made, the v1 pipeline reference, and a troubleshooting section for common tectonic compilation errors. This serves template contributors, users importing their own templates, and maintainers debugging PDF compilation failures.

## Non-Goals

- Code changes to the template or compiler
- Adding new LaTeX templates
- Automating the pdflatex-to-XeTeX conversion
- Documenting non-resume-related LaTeX differences (e.g., bibliography engines, TikZ)
- Docker-based pdflatex fallback compilation path
- Font installation or fontspec configuration for custom fonts

## Context

Forge uses [tectonic](https://tectonic-typesetting.github.io/) (a XeTeX-based engine) to compile LaTeX to PDF. The v1 pipeline used pdflatex inside a Docker container. Many LaTeX resume templates (including the sb2nov template this project uses) are written for pdflatex, and several commands are incompatible with XeTeX/tectonic. The two known incompatibilities already encountered are `\input{glyphtounicode}` and `\pdfgentounicode=1`, which were removed from the sb2nov template during the v1-to-v2 migration.

No documentation currently exists covering these differences, making it difficult for contributors to add new templates or for users to debug compilation failures.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| Introduction | Yes |
| Engine Comparison Table | Yes |
| Known Incompatibilities | Yes |
| Template Import Checklist | Yes |
| sb2nov Template Adaptations | Yes |
| v1 Pipeline Reference | Yes |
| Troubleshooting | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `docs/latex-xetex-compatibility.md` | Comprehensive LaTeX/XeTeX compatibility reference document |

## Files to Modify

None. This is a documentation-only phase.

## Fallback Strategies

- **Document accuracy:** If any documented incompatibility is later found to be inaccurate (e.g., a tectonic update adds support for `\pdfgentounicode`), the document can be updated in a future phase. The checklist is designed to be conservative -- removing lines that are unnecessary is always safe.
- **Missing package info:** The package compatibility table covers the most common resume-related packages. If a user encounters an unlisted package incompatibility, they can check `tectonic --help` or the tectonic issue tracker, and the troubleshooting section guides them.

---

## Tasks

### T46.1: Write `docs/latex-xetex-compatibility.md` [CRITICAL]

**File:** `docs/latex-xetex-compatibility.md`

Create `docs/` directory if it does not exist (`mkdir -p docs`).

Create the full compatibility reference document.

```markdown
# LaTeX / XeTeX Compatibility Reference

Forge uses [tectonic](https://tectonic-typesetting.github.io/) to compile LaTeX resumes to PDF. Tectonic bundles a XeTeX engine with automatic package downloading -- no Docker container or full TeX Live installation is required. The trade-off: some pdflatex-specific commands are unavailable.

This document covers the differences between pdflatex and XeTeX/tectonic, provides a checklist for importing pdflatex templates, documents the adaptations made to the sb2nov template, and includes troubleshooting for common compilation errors.

## Engine Comparison

| Feature | pdflatex | XeTeX (tectonic) |
|---------|----------|-------------------|
| Font system | TFM/Type1 | System fonts (OpenType/TrueType) via fontspec |
| Unicode input | Limited (`\usepackage[utf8]{inputenc}`) | Native UTF-8 |
| `\input{glyphtounicode}` | Supported | Not available (XeTeX handles Unicode natively) |
| `\pdfgentounicode` | Supported (integer register) | Not available |
| Font selection | `\usepackage{times}` etc. | `\setmainfont{...}`, `\setsansfont{...}` |
| Microtype | Full support | Partial (protrusion only, no expansion) |
| Graphics driver | `pdftex.def` (auto) | `xetex.def` (auto) |
| Output format | PDF directly | PDF via xdvipdfmx |
| Package auto-download | No (manual `tlmgr`) | Yes (tectonic bundles) |

## Known Incompatibilities

### `\input{glyphtounicode}`

- **pdflatex:** Loads glyph-to-Unicode mapping for PDF text extraction and copy-paste.
- **XeTeX:** Not available. XeTeX produces Unicode-aware PDFs natively because it processes UTF-8 input directly.
- **Fix:** Remove the line. No replacement needed.
- **Error message:** `I can't find file 'glyphtounicode'`

### `\pdfgentounicode=1`

- **pdflatex:** Enables the glyph-to-Unicode mapping loaded above.
- **XeTeX:** Not a valid register. Causes a compilation error.
- **Fix:** Remove the line. XeTeX PDFs are already Unicode-searchable.
- **Error message:** `Undefined control sequence \pdfgentounicode`

### Font handling

- **pdflatex:** Uses `\usepackage[T1]{fontenc}` with T1 encoding and virtual font packages like `\usepackage{palatino}`, `\usepackage{lmodern}`.
- **XeTeX:** Uses `\usepackage{fontspec}` with system-installed OpenType/TrueType fonts: `\setmainfont{TeX Gyre Pagella}`.
- **For resume templates:** Most templates use Computer Modern (the default), which works in both engines. Templates specifying a font package need conversion to `fontspec`.
- **Tectonic note:** Tectonic bundles standard TeX fonts but cannot access arbitrary system fonts unless configured. The default Computer Modern and Latin Modern fonts work out of the box.

### Unicode handling

- **pdflatex:** Requires `\usepackage[utf8]{inputenc}` for UTF-8 input. Special characters may need escape sequences.
- **XeTeX:** UTF-8 is native. `\usepackage{inputenc}` is unnecessary and may cause warnings.
- **For resume content:** Characters like em-dashes, curly quotes, and accented characters (common in names and company names) work directly in XeTeX without escape sequences.

### Conditional engine detection

Templates may use `\ifpdf` (from `ifpdf` package) or `\ifxetex` (from `ifxetex`). Under XeTeX:

- `\ifpdf` returns **false**
- `\ifxetex` returns **true**

Templates using these conditionals may already be partially XeTeX-aware -- check before making manual changes.

### Package compatibility

| Package | pdflatex | XeTeX (tectonic) | Notes |
|---------|----------|-------------------|-------|
| `enumitem` | Yes | Yes | No changes needed |
| `hyperref` | Yes | Yes | XeTeX may need `\usepackage[xetex]{hyperref}` but tectonic auto-detects |
| `color` / `xcolor` | Yes | Yes | `dvipsnames` option works with XeTeX |
| `titlesec` | Yes | Yes | No changes needed |
| `fancyhdr` | Yes | Yes | No changes needed |
| `geometry` / `fullpage` | Yes | Yes | No changes needed |
| `tabularx` | Yes | Yes | No changes needed |
| `microtype` | Full | Partial | Protrusion only, no expansion. May cause errors -- wrap in conditional or remove |
| `babel` | Yes | Yes | XeTeX projects often prefer `polyglossia` instead |
| `graphicx` | Yes | Yes | Remove explicit `[pdftex]` option if present |

## Template Import Checklist

Step-by-step for converting a pdflatex template to work with tectonic:

1. **Remove** `\input{glyphtounicode}` -- XeTeX handles Unicode natively.
2. **Remove** `\pdfgentounicode=1` -- not a valid XeTeX register.
3. **Remove** `\usepackage[utf8]{inputenc}` if present -- XeTeX is natively UTF-8.
4. **Remove** `\usepackage[T1]{fontenc}` if present -- XeTeX uses fontspec for fonts. Note: remove `inputenc` and `fontenc` together -- these packages are typically paired. Removing only one while keeping the other can cause unexpected behavior.
5. **Font packages:** If the template uses a specific font package (e.g., `\usepackage{palatino}`), either:
   - Remove it (fall back to Computer Modern, which works everywhere), or
   - Replace with `\usepackage{fontspec}` + `\setmainfont{TeX Gyre Pagella}` (tectonic-available equivalent)
6. **Microtype:** If `\usepackage{microtype}` is present, test compilation. Remove if it causes errors.
7. **Graphics:** Replace any `\usepackage[pdftex]{graphicx}` with `\usepackage{graphicx}` (let the engine auto-detect the driver).
8. **Test compile:** Run `tectonic resume.tex` and check for errors.
9. **Verify text selection:** Open the PDF in a reader and try Ctrl+A, Ctrl+C -- verify text is selectable and searchable.
10. **Verify special characters:** Check that em-dashes, accented names, and other Unicode characters render correctly.

## sb2nov Template Adaptations

The following changes were made from the original sb2nov pdflatex template (reference: `zettelkasten/proj/job-hunting/applications/marriott/director-ml-devops/resume.tex`) for the Forge tectonic version (`packages/core/src/templates/sb2nov.ts`):

| Original (pdflatex) | Forge version (tectonic/XeTeX) | Reason |
|---------------------|-------------------------------|--------|
| `\input{glyphtounicode}` | Removed (comment: "not needed with tectonic") | XeTeX handles Unicode natively |
| `\pdfgentounicode=1` | Removed (comment: "not needed with tectonic") | Not a valid XeTeX register |
| All other preamble commands | Unchanged | Compatible with both engines |

The sb2nov template is minimal and avoids font-specific packages, making it highly portable. The only two lines that needed removal were the glyphtounicode commands.

## v1 Pipeline Reference

The v1 resume build pipeline used a different toolchain:

```
resume.md --> mdq --> md2json.py --> mustache --> resume.tex --> pdflatex (Docker)
```

| Stage | Tool | Purpose |
|-------|------|---------|
| Source | `resume.md` | Hand-written Markdown resume source |
| Extract | `mdq` | Markdown query tool for extracting sections |
| Transform | `md2json.py` | Python script converting parsed Markdown to JSON for template rendering |
| Template | `mustache` | Logic-less template engine producing LaTeX from JSON |
| Compile | `pdflatex` | Standard TeX Live pdflatex inside a Docker container (`texlive/texlive:latest`) |

The v2 (Forge) pipeline replaces this with:

```
SQLite DB --> compileResumeIR() --> sb2nov.ts template --> resume.tex --> tectonic
```

Key differences:

- **Source of truth** moved from Markdown files to a SQLite database.
- **Template engine** moved from Mustache to TypeScript functions (type-safe, testable).
- **Compilation engine** moved from pdflatex (Docker) to tectonic (native binary, no Docker).
- The Markdown intermediate format is still available as an output but is no longer the source of truth.

## Troubleshooting

### "Undefined control sequence \pdfgentounicode"

**Cause:** The template contains `\pdfgentounicode=1`, which is a pdflatex-only register.

**Fix:** Remove the line. XeTeX PDFs are already Unicode-searchable without this command.

### "I can't find file 'glyphtounicode'"

**Cause:** The template contains `\input{glyphtounicode}`. This file is available in TeX Live for pdflatex but is not bundled in tectonic.

**Fix:** Remove `\input{glyphtounicode}`. XeTeX handles Unicode natively.

### "LaTeX Error: File 'palatino.sty' not found"

**Cause:** The template uses `\usepackage{palatino}` or another font package not available in tectonic's bundle.

**Fix:** Either remove the line (uses default Computer Modern) or replace with:

```latex
\usepackage{fontspec}
\setmainfont{TeX Gyre Pagella}
```

### "Missing $ inserted"

**Cause:** An unescaped special character in resume content. LaTeX treats `&`, `%`, `$`, `#`, `_`, `{`, `}`, `~`, `^` as special characters.

**Fix:** Ensure the LaTeX compiler's escaping function handles all special characters. In Forge, the `escLatex()` function in `packages/core/src/templates/sb2nov.ts` handles this. If raw text is inserted without escaping, check the template rendering code.

### Blank PDF output

**Cause:** Tectonic may silently produce empty output on certain preamble errors, especially when `\begin{document}` or `\end{document}` is missing.

**Fix:** Verify the template includes both `\begin{document}` and `\end{document}`. Check tectonic's stderr output for warnings.

### "Package microtype Error"

**Cause:** The `microtype` package has limited XeTeX support (protrusion only, no expansion).

**Fix:** Either remove `\usepackage{microtype}` or wrap it in a conditional:

```latex
\usepackage{ifxetex}
\ifxetex\else
  \usepackage{microtype}
\fi
```
```

**Key points:**
- The document is self-contained: no external links that could break, no references to code that might move.
- The Template Import Checklist is ordered for safety: remove incompatible commands first, test last.
- The troubleshooting section uses the exact error messages users will see, making it grep-friendly.
- The v1 pipeline reference is included for historical context since some users may need to understand why certain design decisions were made.

**Acceptance criteria:**
- `docs/latex-xetex-compatibility.md` exists and covers all seven sections from the spec outline.
- The Template Import Checklist is actionable: following it step-by-step converts a pdflatex template to tectonic-compatible.
- The sb2nov adaptations section accurately documents what was changed and why.
- The v1 pipeline reference accurately describes the previous toolchain.
- The troubleshooting section covers the most common tectonic compilation errors with exact error messages.
- The document does not contain code changes, migration scripts, or implementation tasks.

**Failure criteria:**
- Missing sections from the spec outline.
- Inaccurate information about XeTeX behavior.
- Checklist that would break a working template if followed.

---

## Testing Support

### Documentation Review Tests

| Test | What to verify |
|------|---------------|
| Checklist walkthrough | Follow the Template Import Checklist on a third-party pdflatex resume template -- verify it compiles with tectonic after applying all steps |
| Link accuracy | Verify all file path references in the document exist in the repo |
| Engine comparison accuracy | Test each claim in the comparison table against pdflatex and tectonic |

### Compilation Verification Tests

| Test | What to verify |
|------|---------------|
| Reference resume with pdflatex | `zettelkasten/.../resume.tex` compiles with pdflatex and produces valid PDF |
| Forge template with tectonic | sb2nov template output compiles with tectonic and produces valid PDF |
| Checklist on reference resume | Apply checklist transformations to reference resume, compile with tectonic -- verify it works |

### Content Validation Tests

| Test | What to verify |
|------|---------------|
| `\input{glyphtounicode}` causes tectonic error | Confirms the documented incompatibility |
| `\pdfgentounicode=1` causes tectonic error | Confirms the documented incompatibility |
| Removing both lines allows tectonic compilation | Confirms the fix works |

### Doc Tests

The documentation itself serves as a test artifact. Each troubleshooting entry includes the exact error message, which can be validated by intentionally triggering the error with tectonic.

---

## Documentation Requirements

- This phase IS the documentation requirement. The output is `docs/latex-xetex-compatibility.md`.
- No inline code documentation changes needed (no code is changed).
- The spec file serves as the design document for what content to include.

---

## Parallelization Notes

**Within this phase:**
- T46.1 is the only task. No parallelization within the phase.

**Cross-phase:**
- This phase has zero code changes and zero overlap with any other phase.
- Can be executed at any time, in parallel with any other phase.
- If Phase 44 (IR Data Quality) or Phase 45 (Editor Restructuring) change the template or editor behavior, this document does not need updating -- it covers engine-level compatibility, not application-level behavior.
