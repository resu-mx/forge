# M4 — Workday Dropdown Filling

**Version**: 0.1.4  
**Depends on**: M1 (cross-browser), P6/P7 (autofill)  
**Blocks**: M6 (overlay + answer bank + app questions)  
**Bead**: 3bp.25

## Goal

Extend autofill to non-text-input controls on Workday forms: custom dropdown widgets, radio buttons, and native `<select>` elements. Also harden field detection to use semantic/structural selectors instead of fragile CSS class names (addresses job-hunting-1bl5 concern).

## Scope

### In scope
1. **Custom Workday dropdown filling** — country, state, phoneType
2. **Radio button filling** — candidateIsPreviousWorker (yes/no)
3. **Robust field detection** — replace `css-77hcv` exclusion with structural selectors
4. **DetectedField type extension** — add `fieldType` discriminator
5. **Async fillField** — dropdown interaction requires click-wait-click
6. **Profile mapping** — address.country → country dropdown, address.state → state dropdown
7. **FIELD_NAME_MAP extension** — phoneType, candidateIsPreviousWorker

### Out of scope
- Multiselect widgets (source, countryPhoneCode) — M5 or later
- EEO/work-auth fields — M6 answer bank
- Checkbox filling (preferredCheck) — M6
- Fixture capture tooling — M7 (bead 1bl5)

## Design

### 1. Robust Field Detection (replaces CSS class filtering)

**Current fragile filter** (workday.ts):
```typescript
// BAD: css-77hcv is a generated class, differs across deployments
if (input.className === 'css-77hcv') continue
```

**New semantic detection** — detect by what elements ARE, not what they look like:

```
formField-* wrapper
├── button[aria-haspopup="listbox"]  → fieldType: 'custom-dropdown'
│   └── sibling input[type="text"]   → backing input (SKIP, not a field)
├── input[type="radio"]              → fieldType: 'radio'  
├── input[type="checkbox"]           → fieldType: 'checkbox' (detect but skip fill in M4)
├── select                           → fieldType: 'select'
└── input[type="text"|"email"|"tel"] → fieldType: 'text' (existing)
```

**Backing input exclusion** — structural, no CSS classes:
```
A text input whose parent also contains a button[aria-haspopup="listbox"]
is a backing input → exclude from detection.
```

**Multiselect exclusion** — Workday data attribute:
```
input[data-uxi-widget-type="selectinput"] → exclude (M5 scope)
```

### 2. DetectedField Type Extension

```typescript
// Add to types.ts
export type FieldType = 'text' | 'select' | 'custom-dropdown' | 'radio' | 'checkbox'

export interface DetectedField {
  element: Element        // the interactive element (button for dropdown, input for radio/text)
  label_text: string | null
  field_kind: FieldKind
  field_type: FieldType   // NEW — discriminator for fill strategy
  required: boolean
}
```

For custom dropdowns, `element` is the `<button>`, not the backing input. The button is what gets clicked to open the listbox.

For radios, `element` is the wrapper div containing all radio inputs for the group (the `div[name="..."][aria-required]`). Individual radio inputs are found during fill.

### 3. Fill Strategy Per Field Type

**fillField becomes async** — signature change:
```typescript
// Before
fillField(element: Element, value: string): boolean
// After
fillField(field: DetectedField, value: string): Promise<boolean>
```

Passing the full `DetectedField` (not just element) so the fill function can branch on `field_type`.

#### 3a. fillTextInput (existing, extracted)
No change to logic. Extract current `fillField` body into `fillTextInput`.

#### 3b. fillCustomDropdown (new)
Workday custom dropdown interaction sequence:

1. **Click the button** to open the popup
2. **Wait for listbox** — poll for `[role="listbox"]` appearing in the DOM (Workday lazy-renders it). Max 2s timeout.
3. **Find matching option** — scan `[role="option"]` children for text match. Use fuzzy matching: normalize whitespace, case-insensitive startsWith for country/state names (e.g., value "Colorado" matches "Colorado (CO)").
4. **Click the option** — dispatches Workday's internal selection handler
5. **Wait for popup close** — the listbox should disappear. Max 500ms.
6. **Verify** — check button's textContent updated to reflect selection

The listbox appears as a sibling/portal in the DOM (not inside the button's parent). Locate it via the button's `aria-controls` or `aria-owns` attribute, or fall back to `document.querySelector('[role="listbox"]:not([hidden])')`.

**Event dispatch**: The button click and option click use real mouse events (click), not synthetic value setting. Workday's React/UXI framework handles the rest.

#### 3c. fillRadio (new)
1. **Find radio group** — element is the group wrapper with `name` attribute
2. **Match value** — find `input[type="radio"]` whose:
   - `value` attribute matches (e.g., "true"/"false" for yes/no), OR
   - sibling `<label>` text matches the fill value (case-insensitive)
3. **Click the radio** — use `.click()` on the input element
4. **Dispatch change** — `new Event('change', { bubbles: true })`

#### 3d. fillNativeSelect (new, for completeness)
1. Set `.value` on the `<select>` element
2. Use native value setter (same React pattern as text inputs)
3. Dispatch `change` event with `{ bubbles: true }`

Note: Workday rarely uses native `<select>`. This is for edge cases and future ATS reuse.

### 4. FIELD_NAME_MAP Extension

```typescript
const FIELD_NAME_MAP: Record<string, FieldKind> = {
  // existing
  'legalName--firstName': 'name.first',
  'legalName--lastName': 'name.last',
  'email': 'email',
  'city': 'address.city',
  'countryRegion': 'address.state',
  'country': 'address.country',
  'phoneNumber': 'phone',
  // M4 additions
  'phoneType': 'phone.type',         // new FieldKind
  'candidateIsPreviousWorker': 'unknown',  // no profile mapping yet (M6)
}
```

`phone.type` is a new FieldKind. Profile mapping will default to "Mobile" for phone type since the profile doesn't store this.

### 5. Profile Mapping Extension

```typescript
// profile-map.ts additions
// phone.type → default "Mobile" (most common for personal phone)
if (profile.phone) map['phone.type'] = 'Mobile'
```

Country and state mappings already exist (`address.country`, `address.state`). The fill value for country needs to be the display name (e.g., "United States of America") not the code ("US"). Add a small lookup map for common country codes → display names. State values pass through as-is (profile stores state name).

```typescript
const COUNTRY_CODE_TO_NAME: Record<string, string> = {
  'US': 'United States of America',
  'CA': 'Canada',
  'GB': 'United Kingdom',
  'AU': 'Australia',
  // extend as needed
}
```

### 6. Content Script Changes

The content script fill loop becomes async:
```typescript
// content/workday.ts — profileFill handler
for (const field of fields) {
  const value = values[field.field_kind]
  if (!value || field.field_kind === 'unknown') { skipped++; continue }
  const ok = await plugin.capabilities.fillField!(field, value)
  // ...
}
```

The `fillField` capability signature in `types.ts` changes to return `Promise<boolean>`.

### 7. Detection Algorithm

```
For each div[data-automation-id^="formField-"]:
  fieldName = automationId.replace('formField-', '')
  fieldKind = FIELD_NAME_MAP[fieldName] ?? 'unknown'

  // 1. Custom dropdown?
  button = wrapper.querySelector('button[aria-haspopup="listbox"]')
  if (button):
    yield { element: button, fieldType: 'custom-dropdown', fieldKind, ... }
    continue

  // 2. Radio group?
  radioGroup = wrapper.querySelector('div[name], fieldset')
  radios = wrapper.querySelectorAll('input[type="radio"]')
  if (radios.length > 0):
    yield { element: radioGroup ?? radios[0].parentElement, fieldType: 'radio', fieldKind, ... }
    continue

  // 3. Native select?
  select = wrapper.querySelector('select')
  if (select):
    yield { element: select, fieldType: 'select', fieldKind, ... }
    continue

  // 4. Text input (existing logic, minus CSS class filter)
  for input in wrapper.querySelectorAll('input'):
    if input.type in ['hidden', 'submit', 'button']: skip
    if input matches [data-uxi-widget-type="selectinput"]: skip  // multiselect
    if input.parentElement.querySelector('button[aria-haspopup="listbox"]'): skip  // backing input
    yield { element: input, fieldType: 'text', fieldKind, ... }
```

## Testing

### Unit tests (workday.test.ts)
1. Detects custom dropdown fields (country, state, phoneType) with fieldType 'custom-dropdown'
2. Detects radio group (candidateIsPreviousWorker) with fieldType 'radio'
3. Existing text input tests still pass with fieldType 'text'
4. Backing input excluded WITHOUT css-77hcv class check
5. Multiselect inputs excluded via data-uxi-widget-type
6. Total detected field count updated (7 text + 3 dropdown + 1 radio = 11)

### Fill tests (new, against fixture)
7. fillTextInput — existing behavior preserved
8. fillCustomDropdown — clicks button, finds option, returns true (needs mock popup DOM)
9. fillRadio — clicks correct radio for value match
10. fillNativeSelect — sets value + dispatches change
11. fillField dispatches to correct handler based on field_type

### Profile mapping tests (profile-map.test.ts)
12. phone.type defaults to "Mobile" when phone exists
13. Country code "US" maps to "United States of America"
14. Country code without mapping passes through as-is

### Integration
15. Content script profileFill handles async fill loop
16. Mixed field types (text + dropdown + radio) all fill in one pass

## Phases

### Phase 1: Robust detection + types (no fill changes yet)
- Add `FieldType` to types
- Add `field_type` to `DetectedField`
- Rewrite detection algorithm with semantic selectors
- Remove `css-77hcv` filter
- Update existing tests + add new detection tests
- Extend FIELD_NAME_MAP

### Phase 2: Fill infrastructure
- Make `fillField` async, accept `DetectedField`
- Extract `fillTextInput` from existing `fillField`
- Add `fillNativeSelect` (simple)
- Add `fillRadio`
- Update content script for async fill loop
- Tests for each fill function

### Phase 3: Custom dropdown fill
- Implement `fillCustomDropdown` (click-wait-click)
- Add country code → name mapping to profile-map
- Add phone.type default mapping
- Mock popup DOM in tests
- Integration test: mixed field fill

### Phase 4: Version bump + validation
- Bump manifests to 0.1.4
- Build both browsers
- Run full test suite
- Update parity tests if needed

## FieldKind additions

```typescript
// Add to types.ts FieldKind union
| 'phone.type'
```
