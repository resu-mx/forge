# LaTeX / XeTeX Compatibility Documentation

**Date:** 2026-04-03
**Spec:** F3 (LaTeX / XeTeX Compat Docs)
**Phase:** TBD
**Dependencies:** None
**Type:** Documentation only (no code changes)

## Overview

The Forge resume builder uses [tectonic](https://tectonic-typesetting.github.io/) (a XeTeX-based engine) to compile LaTeX to PDF. The v1 pipeline used pdflatex inside a Docker container. Many LaTeX resume templates (including the sb2nov template this project is based on) are written for pdflatex, and several commands are incompatible with XeTeX/tectonic.

This spec produces a reference document covering the differences between pdflatex and XeTeX/tectonic, a checklist for importing pdflatex templates, and documentation of the adaptations already made to the sb2nov template.

## Scope

Write a single documentation file: `docs/latex-xetex-compatibility.md`

This document serves three audiences:
1. **Template contributors** who want to add new LaTeX templates to Forge
2. **Users** who want to import their existing pdflatex resume templates
3. **Maintainers** debugging PDF compilation failures

## Technical Approach

### 1. Document structure

```
docs/latex-xetex-compatibility.md
├── Introduction (why tectonic, why this matters)
├── Engine Comparison Table
├── Known Incompatibilities
│   ├── \input{glyphtounicode}
│   ├── \pdfgentounicode
│   ├── Font handling
│   ├── Unicode handling
│   └── Package compatibility
├── Template Import Checklist
├── sb2nov Template Adaptations
├── v1 Pipeline Reference
└── Troubleshooting
```

### 2. Content outline

#### 2.1 Introduction

- Forge uses tectonic for PDF compilation
- Tectonic bundles a XeTeX engine with automatic package downloading
- No Docker or full TeX Live installation required
- Trade-off: some pdflatex-specific commands are unavailable

#### 2.2 Engine Comparison Table

| Feature | pdflatex | XeTeX (tectonic) |
|---------|----------|-------------------|
| Font system | TFM/Type1 | System fonts (OpenType/TrueType) via fontspec |
| Unicode input | Limited (inputenc) | Native UTF-8 |
| \input{glyphtounicode} | Supported | Not available (XeTeX handles Unicode natively) |
| \pdfgentounicode | Supported (integer register) | Not available |
| Font selection | \usepackage{times} etc. | \setmainfont{...}, \setsansfont{...} |
| Microtype | Full support | Partial (protrusion only, no expansion) |
| Graphics driver | pdftex.def (auto) | xetex.def (auto) |
| Output format | PDF directly | PDF via xdvipdfmx |
| Package auto-download | No (manual tlmgr) | Yes (tectonic bundles) |

#### 2.3 Known Incompatibilities

**`\input{glyphtounicode}`**
- pdflatex: Loads glyph-to-Unicode mapping for PDF text extraction/copy-paste
- XeTeX: Not available. XeTeX produces Unicode-aware PDFs natively because it processes UTF-8 input directly
- Fix: Remove the line. No replacement needed.

**`\pdfgentounicode=1`**
- pdflatex: Enables the glyph-to-Unicode mapping loaded above
- XeTeX: Not a valid register. Will cause a compilation error.
- Fix: Remove the line. XeTeX PDFs are already Unicode-searchable.

**Font handling**
- pdflatex: Uses `\usepackage{fontenc}` with T1 encoding and virtual font packages
- XeTeX: Uses `\usepackage{fontspec}` with system-installed OpenType/TrueType fonts
- For resume templates: Most templates use Computer Modern (default) which works in both engines. Templates specifying `\usepackage{lmodern}` or `\usepackage{palatino}` need conversion to `\setmainfont{...}` with XeTeX.
- Tectonic note: tectonic bundles standard TeX fonts but cannot access arbitrary system fonts unless configured. The default Computer Modern / Latin Modern fonts work out of the box.

**Unicode handling**
- pdflatex: Requires `\usepackage[utf8]{inputenc}` for UTF-8 input. Special characters may need escape sequences.
- XeTeX: UTF-8 is native. `\usepackage{inputenc}` is unnecessary and may cause warnings.
- For resume content: Characters like em-dashes, curly quotes, and accented characters (common in names/companies) work directly in XeTeX without escape sequences.

**Conditional Engine Detection:** Templates may use `\ifpdf` (from `ifpdf` package) or `\ifxetex` (from `ifxetex`). Under XeTeX: `\ifpdf` returns false, `\ifxetex` returns true. Templates using these conditionals may already be partially XeTeX-aware — check before making manual changes.

**Package compatibility notes**
- `enumitem`: Works in both engines. No changes needed.
- `hyperref`: Works in both. XeTeX may need `\usepackage[xetex]{hyperref}` but tectonic auto-detects.
- `color` / `xcolor`: Work in both. The `dvipsnames` option works with XeTeX.
- `titlesec`: Works in both.
- `fancyhdr`: Works in both.
- `geometry` / `fullpage`: Work in both.
- `tabularx`: Works in both.
- `microtype`: Partially supported in XeTeX (protrusion only). The `\usepackage{microtype}` line should be wrapped in a conditional or removed if it causes errors.
- `babel`: Works in both, though XeTeX projects often prefer `polyglossia`.

#### 2.4 Template Import Checklist

Step-by-step for converting a pdflatex template to work with tectonic:

1. Remove `\input{glyphtounicode}` (XeTeX handles Unicode natively)
2. Remove `\pdfgentounicode=1` (not a valid XeTeX register)
3. Remove `\usepackage[utf8]{inputenc}` if present (XeTeX is natively UTF-8)
4. Remove `\usepackage[T1]{fontenc}` if present (XeTeX uses fontspec for fonts). Note: Remove `\usepackage[utf8]{inputenc}` AND `\usepackage[T1]{fontenc}` together — these packages are typically paired. Removing only one while keeping the other can cause unexpected behavior.
5. If the template uses a specific font package (e.g., `\usepackage{palatino}`), either:
   - Remove it (fall back to Computer Modern, which works everywhere), or
   - Replace with `\usepackage{fontspec}` + `\setmainfont{TeX Gyre Pagella}` (tectonic-available equivalent)
6. If `\usepackage{microtype}` is present, test compilation. Remove if it causes errors.
7. Replace any `\usepackage[pdftex]{graphicx}` with `\usepackage{graphicx}` (let the engine auto-detect)
8. Test compile with `tectonic resume.tex` and check for errors
9. Verify PDF text is selectable/searchable (open in a reader and try Ctrl+A, Ctrl+C)
10. Verify special characters render correctly (em-dashes, accented names, etc.)

#### 2.5 sb2nov Template Adaptations

Document what was changed from the original sb2nov pdflatex template (`zettelkasten/proj/job-hunting/applications/marriott/director-ml-devops/resume.tex`) for the Forge tectonic version (`packages/core/src/templates/sb2nov.ts`):

| Original (pdflatex) | Forge version (tectonic/XeTeX) | Reason |
|---------------------|-------------------------------|--------|
| `\input{glyphtounicode}` | Removed (comment: "not needed with tectonic") | XeTeX handles Unicode natively |
| `\pdfgentounicode=1` | Removed (comment: "not needed with tectonic") | Not a valid XeTeX register |
| All other preamble commands | Unchanged | Compatible with both engines |

The sb2nov template is minimal and avoids font-specific packages, making it highly portable. The only two lines that needed removal were the glyphtounicode commands.

#### 2.6 v1 Pipeline Reference

The v1 resume build pipeline used a different toolchain:

```
resume.md --> mdq --> md2json.py --> mustache --> resume.tex --> pdflatex (Docker)
```

- `resume.md`: Hand-written Markdown resume source
- `mdq`: Markdown query tool for extracting sections
- `md2json.py`: Python script converting parsed Markdown to JSON for template rendering
- `mustache`: Logic-less template engine producing LaTeX from JSON
- `pdflatex`: Standard TeX Live pdflatex inside a Docker container (`texlive/texlive:latest`)

The v2 (Forge) pipeline replaces this with:

```
SQLite DB --> compileResumeIR() --> sb2nov.ts template --> resume.tex --> tectonic
```

Key differences:
- Source of truth moved from Markdown files to a SQLite database
- Template engine moved from Mustache to TypeScript functions (type-safe, testable)
- Compilation engine moved from pdflatex (Docker) to tectonic (native binary, no Docker)
- The Markdown intermediate format is still available as an output but is no longer the source of truth

#### 2.7 Troubleshooting

Common tectonic compilation errors and their fixes:

- **"Undefined control sequence \pdfgentounicode"**: Remove the line. XeTeX does not have this register.
- **"I can't find file `glyphtounicode'"**: Remove `\input{glyphtounicode}`. Not available in tectonic's bundle.
- **"LaTeX Error: File `palatino.sty' not found"**: Replace with `\usepackage{fontspec}\setmainfont{TeX Gyre Pagella}` or remove to use default fonts.
- **"Missing $ inserted"**: Usually an unescaped special character in resume content. Ensure the LaTeX compiler's escaping function handles `&`, `%`, `$`, `#`, `_`, `{`, `}`, `~`, `^`.
- **Blank PDF output**: Check that `\begin{document}` and `\end{document}` are present. Tectonic may silently produce empty output on certain preamble errors.

## Files to Create

| File | Purpose |
|------|---------|
| `docs/latex-xetex-compatibility.md` | Comprehensive compatibility reference document |

## Files to Modify

None. This is a documentation-only spec.

## Testing Approach

### Documentation review

1. Have a contributor read the Template Import Checklist and successfully convert a third-party pdflatex resume template to work with tectonic
2. Verify all links and file path references in the document are correct
3. Verify the engine comparison table is accurate by testing the listed commands in both pdflatex and tectonic

### Compilation verification

1. Compile the reference resume (`zettelkasten/.../resume.tex`) with pdflatex -- verify it produces valid PDF
2. Compile the Forge sb2nov template output with tectonic -- verify it produces valid PDF
3. Take the reference resume, apply the import checklist transformations, compile with tectonic -- verify it works

### Content validation

1. Confirm `\input{glyphtounicode}` causes a tectonic error (validates the documented incompatibility)
2. Confirm `\pdfgentounicode=1` causes a tectonic error
3. Confirm removing both lines allows successful tectonic compilation

## Acceptance Criteria

1. `docs/latex-xetex-compatibility.md` exists and covers all sections from the outline
2. The Template Import Checklist is actionable: following it step-by-step converts a pdflatex template to tectonic-compatible
3. The sb2nov adaptations section accurately documents what was changed and why
4. The v1 pipeline reference accurately describes the previous toolchain
5. The troubleshooting section covers the most common tectonic compilation errors
6. The document does not contain code changes, migration scripts, or implementation tasks

## Non-Goals

- Code changes to the template or compiler
- Adding new LaTeX templates
- Automating the pdflatex-to-XeTeX conversion
- Documenting non-resume-related LaTeX differences (e.g., bibliography engines, TikZ)
- Docker-based pdflatex fallback compilation path
- Font installation or fontspec configuration for custom fonts
