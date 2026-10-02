# Bundled fonts

New Computer Modern 10 (regular, bold, italic, bold italic), the closest match to the
Computer Modern look of the `sb2nov` LaTeX template the resume used before.

Copied unmodified from the `typst-assets` crate (0.15.1). Licences: `NewCM10-Regular` is under the SIL
Open Font License; the other three faces are under the GUST Font License. Both allow
redistribution with the licence text, which is in `NOTICE` (the full notice shipped with
`typst-assets`; it also covers fonts we do not bundle).

Only these four are bundled on purpose: the whole `typst-assets` font set is about 11 MB, and a
resume needs one text family. Add a face by copying it here and listing it in `src/lib.rs`.
