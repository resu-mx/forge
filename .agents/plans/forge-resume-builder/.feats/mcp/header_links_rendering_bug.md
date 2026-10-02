# Header contact links render as empty/broken links
**Type**: bug
**Component**: resume-compiler
**Filed**: 2026-04-08
**Status**: open

## Description

Two issues with the contact info line in resume header rendering:

### 1. Missing protocol prefix breaks links
URLs in `user_profile` are stored without `https://` prefix (e.g., `linkedin.com/in/smithadamc`). The resume compiler outputs them as-is in markdown links:
```markdown
[LinkedIn](linkedin.com/in/smithadamc)
```
Most renderers (LaTeX `\href{}`, HTML `<a href="">`, PDF viewers) treat protocol-less URLs as relative paths, resulting in empty or broken links.

**Fix**: Compiler should prepend `https://` if the URL doesn't already start with `http://` or `https://`.

### 2. "Website" label should be "Blog" (or configurable)
The compiler hardcodes `Website` as the label for the `website` field. When the URL is `blog.arusty.dev`, the label should be "Blog" to match.

**Fix options**:
1. Add a `website_label` field to `user_profile` (most flexible)
2. Auto-detect from URL: if hostname starts with `blog.`, use "Blog"; otherwise "Website"
3. Allow per-resume header override for link labels
