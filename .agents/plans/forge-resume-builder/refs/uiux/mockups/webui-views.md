# WebUI View Descriptions

Detailed wireframe descriptions for each MVP view. Visual mockups to be created during Phase 7 implementation.

## View 1: Sources List/Editor

**Layout:** Two-panel — list on left, editor on right.

**Left panel (list):**
- Filter bar: employer dropdown, project dropdown, status tabs (all | draft | approved)
- Source cards showing: title, first line of description, bullet count badge, last_derived_at
- "New Source" button at top
- Click a card to open in editor

**Right panel (editor):**
- Title input field
- Description textarea (multi-line, resizable)
- Employer/project selectors
- Start/end date pickers
- Status badge (read-only)
- "Derive Bullets" button (disabled if not approved, shows spinner if deriving)
- Bullet count with link to derivation view
- Save/cancel buttons

## View 2: Derivation View

**Layout:** Three-column — source | bullets | perspectives

**Column 1 (source):**
- Read-only source content
- Employer, dates, status

**Column 2 (bullets):**
- List of bullets derived from this source
- Each bullet card shows: content, technologies, status badge, snapshot match indicator
- Approve/reject buttons inline on each card
- Rejection shows reason input modal
- "Derive Perspectives" button on each approved bullet (opens archetype/domain/framing selector)

**Column 3 (perspectives):**
- List of perspectives derived from selected bullet
- Each perspective card shows: content, archetype, domain, framing, status badge, snapshot match
- Approve/reject buttons inline

**Derivation in progress:** Overlay spinner on the relevant column with "Generating... (up to 60s)" message.

## View 3: Chain View

**Layout:** Tree visualization (top-to-bottom or left-to-right)

**Root:** Source node (title, description preview)
**Level 1:** Bullet nodes branching from source
**Level 2:** Perspective nodes branching from bullets

**Node rendering:**
- Collapsed: title/preview + status badge + snapshot indicator
- Expanded (on click): full content, content snapshot comparison, edit button
- Green border: snapshot matches current content
- Yellow border + warning icon: snapshot diverges from current content

**Interaction:**
- Click node to expand/collapse
- "Edit" opens inline editor
- "View diff" shows side-by-side snapshot vs current

## View 4: Resume Builder

**Layout:** Two-panel — section organizer on left, gap analysis on right.

**Left panel (organizer):**
- Resume metadata header: name, target_role, target_employer, archetype
- Section groups: Summary, Work History, Projects, Education, Skills, Awards
- Each section contains draggable perspective cards
- Drag to reorder within section or move between sections
- "Add Perspective" button per section — opens a picker showing approved perspectives for this archetype
- "Remove" button on each perspective card

**Right panel (gap analysis):**
- Auto-updates when perspectives are added/removed
- Sections:
  - Coverage summary: perspectives included, skills covered, domains represented
  - Gaps: missing domain coverage, thin coverage, unused bullets
  - Each gap has an action button: "Derive Perspective" or "View Bullet"
- "Export" button — shows 501 message with `just export-resume` instructions for MVP

## View 5: Review Queue (Dashboard)

**Layout:** Single-page dashboard, landing page of the app.

**Content:**
- Summary cards: "N bullets pending review", "N perspectives pending review"
- Click a card to navigate to the relevant derivation view
- Recent activity: last 10 approved/rejected items with timestamps
- Quick stats: total sources, total approved bullets, total approved perspectives
