# Certification ordering: support custom order for both categories and certs within categories
**Type**: bug + feature-request
**Component**: resume-compiler, webui
**Filed**: 2026-04-08
**Status**: open

## Description

Certifications render in a fixed alphabetical order for both:
1. **Category rows** (e.g., AWS, GIAC, GAQM) — the issuer groupings
2. **Certs within a category** (e.g., SAA, MLE, AIP, DEV within AWS)

There is no way to control ordering. The user may want to lead with the most relevant cert (e.g., MLE before SAA for an ML role) or the most relevant issuer group (e.g., GIAC before GAQM).

## Expected Behavior

- **Default**: alphabetical ordering (current behavior — keep as fallback)
- **Override**: user can reorder via drag-and-drop in the web UI, persisted as explicit position values
- This applies to **both dimensions**:
  - **Row ordering**: which issuer category appears first (AWS vs GIAC vs GAQM)
  - **Column ordering within a row**: which cert appears first within an issuer group (MLE, SAA, DEV, AIP vs alphabetical AIP, DEV, MLE, SAA)

## Current Behavior

`resume_certifications` has a `position` field, but:
- It appears to be a global position across all certs, not scoped per category
- The renderer ignores it and sorts alphabetically by issuer name, then cert short_name

## Fix

### Data model
Add ordering fields to `resume_certifications`:
- `category_position INTEGER` — ordering of the issuer group within the section (row order)
- `cert_position INTEGER` — ordering of the cert within its issuer group (column order)

Or alternatively, use the existing `position` field but interpret it as a composite: group by issuer, sort groups by min(position), sort certs within group by position.

### Resume compiler
- Group certs by issuer
- Sort groups by `category_position` (or min position in group), falling back to alphabetical
- Sort certs within group by `cert_position`, falling back to alphabetical

### Web UI
- Cert section should render as a grid: rows = issuer categories, columns = certs
- Support DnD reordering for both rows (category order) and items within rows (cert order)
- On reorder, persist new position values to `resume_certifications`
