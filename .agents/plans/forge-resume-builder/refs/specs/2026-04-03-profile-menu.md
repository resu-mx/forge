# Profile Button & User Menu

**Date:** 2026-04-03
**Spec:** C (Profile Menu)
**Depends on:** Spec A (Design System & CSS Variables) -- tokens for theming, base classes for buttons/modals
**Phase:** TBD (after Spec A)

## Overview

Replace the sidebar's Config nav group with a profile button anchored to the bottom of the sidebar, modeled after Slack and Discord's user area. Clicking the button opens a flyout/popover menu with quick-view profile info, links to config pages, and a theme toggle. This consolidates all config/settings access into a single, always-visible entry point and removes the Config group from the main navigation.

The profile menu also introduces the theme toggle UI that Spec A's `themeStore` was built to support (Spec A created the store, this spec creates the button that calls it).

---

## 1. Profile Button (Sidebar Bottom)

**Location:** Bottom of `.sidebar` in `packages/webui/src/routes/+layout.svelte`

### Design

- Fixed to the bottom of the sidebar, separated from the nav list by a border-top.
- Contains: user avatar (placeholder circle with initials), name, and a gear/settings icon.
- `initials` is derived from the profile name: `name.split(' ').map(w => w[0]).join('').slice(0, 2).toUpperCase()`. If name is empty, show `'?'`.
- Clicking the button toggles the profile menu popover.

```svelte
<div class="profile-button-area">
  <button class="profile-button" onclick={toggleProfileMenu}>
    <span class="profile-avatar">{initials}</span>
    <span class="profile-name">{profileName}</span>
    <span class="profile-gear">&#9881;</span>
  </button>
</div>
```

### Layout changes to sidebar

This spec owns the sidebar flex-column restructuring. Change `.sidebar` to `display: flex; flex-direction: column` and `.nav-list` to `flex: 1; overflow-y: auto`. Spec A only tokenizes colors/sizes on the existing layout.

The sidebar currently has a simple flex-column layout. To pin the profile button to the bottom:

```css
.sidebar {
  display: flex;
  flex-direction: column;
}

.nav-list {
  flex: 1;
  overflow-y: auto;
}

.profile-button-area {
  border-top: 1px solid var(--color-sidebar-border);
  padding: var(--space-3) var(--space-4);
  flex-shrink: 0;
}
```

### Data loading

On mount, `+layout.svelte` fetches the user profile via `forge.profile.get()` to populate the profile button (name, initials) and the profile menu (email, phone, location, links). This is a single API call, cached in a layout-level `$state`.

---

## 2. Profile Menu (Flyout/Popover)

**File to create:** `packages/webui/src/lib/components/ProfileMenu.svelte`

### Behavior

- Opens upward from the profile button (popover anchored to bottom-left of sidebar, expanding up and to the right).
- Clicking outside the menu closes it. Use a Svelte action or `window` `pointerdown` listener that checks `!menuEl.contains(event.target) && !buttonEl.contains(event.target)` to close the menu on outside click.
- Pressing Escape closes it.
- Menu sections are visually separated by dividers (1px border).

### Menu Structure

```
+------------------------------------------+
|  [Avatar]  Name                          |
|  email@example.com                       |
|  +1 (555) 123-4567                       |
|  Location, ST                            |
|  [GitHub] [LinkedIn] [Portfolio] [Blog]  |
|  [Edit Profile ->]                       |
+------------------------------------------+
|  Account                                 |
|    Login / Logout        (Coming Soon)   |
+------------------------------------------+
|  Config                                  |
|    Plugins               (Coming Soon)   |
|    External APIs         (Coming Soon)   |
+------------------------------------------+
|  Settings                                |
|    Data Privacy Viz      (Coming Soon)   |
|    Privacy Settings      (Coming Soon)   |
+------------------------------------------+
|  Export                                  |
|    Export Data            -> /config/export|
+------------------------------------------+
|  Debug                                   |
|    Overview              -> /config/debug |
|    Prompt Logs           -> /config/debug/prompts |
|    Forge API Logs        -> /config/debug/api     |
|    Event Logs            -> /config/debug/events  |
|    UI/UX Logs            -> /config/debug/ui      |
+------------------------------------------+
|  About                                   |
|    Privacy Policy        (Coming Soon)   |
|    About Forge           (Coming Soon)   |
|    Documentation         (Coming Soon)   |
+------------------------------------------+
|  Theme: [Light] [Dark] [System]          |
+------------------------------------------+
```

### Component API

```svelte
<ProfileMenu
  profile={profileData}
  isOpen={menuOpen}
  onclose={() => menuOpen = false}
/>
```

Props:
- `profile: UserProfile | null` -- the loaded profile data
- `isOpen: boolean` -- controls visibility
- `onclose: () => void` -- called when the menu should close (click outside, Escape, or navigation)

### Styling

- Background: `var(--color-surface)`
- Border: `1px solid var(--color-border)`
- Border radius: `var(--radius-lg)`
- Shadow: `var(--shadow-lg)`
- Width: `280px`
- Max height: `calc(100vh - 80px)` with `overflow-y: auto`
- Z-index: `1000` (above kanban cards at `var(--z-dropdown): 100` but below modals at `var(--z-modal): 10000`. The profile menu needs a higher z-index than standard dropdowns because it must overlay sidebar nav items and kanban content.)
- Position: absolute, anchored to bottom-left of sidebar, expanding upward

Section headers use `var(--text-xs)`, `var(--font-semibold)`, `var(--text-faint)`, uppercase, letter-spacing `0.05em`.

Menu items use `var(--text-sm)`, `var(--text-secondary)`, with hover background `var(--color-surface-raised)`.

"Coming Soon" labels use `var(--text-xs)`, `var(--text-faint)`, italic.

---

## 3. Phone Number Formatting

### Storage format

Raw digits with country code prefix, stored in `user_profile.phone`:
```
+15551234567
```

No changes to the database schema or API -- phone is already a free-text `TEXT` field. The convention is enforced at the UI level only.

### Display format

Formatted on display as:
```
+1 (555) 123-4567
```

### Implementation

Create a utility function in `packages/webui/src/lib/format.ts`:

```typescript
/**
 * Format a raw phone string (e.g. "+15551234567") into display format.
 * Supports US numbers (+1) and international numbers.
 *
 * Examples:
 *   "+15551234567"  -> "+1 (555) 123-4567"
 *   "+442071234567" -> "+44 207 123 4567"
 *   "5551234567"    -> "(555) 123-4567"    (assumes US, no country code)
 *   ""              -> ""                   (empty passthrough)
 */
export function formatPhone(raw: string | null | undefined): string {
  if (!raw) return ''
  const digits = raw.replace(/\D/g, '')

  // US number with country code: 11 digits starting with 1
  if (digits.length === 11 && digits.startsWith('1')) {
    const cc = digits[0]
    const area = digits.slice(1, 4)
    const prefix = digits.slice(4, 7)
    const line = digits.slice(7, 11)
    return `+${cc} (${area}) ${prefix}-${line}`
  }

  // US number without country code: 10 digits
  if (digits.length === 10) {
    const area = digits.slice(0, 3)
    const prefix = digits.slice(3, 6)
    const line = digits.slice(6, 10)
    return `(${area}) ${prefix}-${line}`
  }

  // International: insert spaces after country code using basic grouping
  // For international numbers (starts with `+` but not `+1`), group as:
  // +{cc} {rest in groups of 3-4}. Example: +442071234567 → +44 207 123 4567
  if (raw.startsWith('+') && digits.length > 10) {
    // Determine country code length (1-3 digits). Heuristic: if total digits > 12, cc is 3; > 11, cc is 2; else 1.
    const ccLen = digits.length > 12 ? 3 : digits.length > 11 ? 2 : 1
    const cc = digits.slice(0, ccLen)
    const rest = digits.slice(ccLen)
    // Group remaining digits in chunks of 3-4
    const groups: string[] = []
    let i = 0
    while (i < rest.length) {
      const remaining = rest.length - i
      // Use groups of 3, but if only 4 left, use 4 (avoid leaving a lone digit)
      const chunk = remaining > 4 ? 3 : remaining
      groups.push(rest.slice(i, i + chunk))
      i += chunk
    }
    return `+${cc} ${groups.join(' ')}`
  }

  // Fallback: return as-is
  return raw
}
```

The `ProfileMenu` component calls `formatPhone(profile.phone)` for display. The profile edit page (`/config/profile`) continues to store whatever the user types -- no input masking is introduced in this spec.

---

## 4. Theme Toggle

### Location

Bottom of the profile menu, full-width row with three segmented buttons: `Light`, `Dark`, `System`.

### Behavior

- Reads current theme from `themeStore` (created in Spec A: `$lib/stores/theme.svelte.ts`)
- On click, calls `themeStore.set('light' | 'dark' | 'system')`
- The store updates `localStorage('forge-theme')` and sets/removes `data-theme` attribute on `<html>`
- Active button is visually highlighted with `var(--color-primary)` background

### Implementation

```svelte
<div class="theme-toggle">
  <span class="theme-label">Theme</span>
  <div class="theme-buttons">
    <button
      class="theme-btn"
      class:active={currentTheme === 'light'}
      onclick={() => setTheme('light')}
    >Light</button>
    <button
      class="theme-btn"
      class:active={currentTheme === 'dark'}
      onclick={() => setTheme('dark')}
    >Dark</button>
    <button
      class="theme-btn"
      class:active={currentTheme === 'system'}
      onclick={() => setTheme('system')}
    >System</button>
  </div>
</div>
```

Styling:
```css
.theme-toggle {
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--color-border);
}

.theme-buttons {
  display: flex;
  gap: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.theme-btn {
  flex: 1;
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-xs);
  border: none;
  background: var(--color-surface);
  color: var(--text-secondary);
  cursor: pointer;
}

.theme-btn.active {
  background: var(--color-primary);
  color: var(--text-inverse);
}
```

---

## 5. Debug Sub-Pages

### Current state

`/config/debug` is a single page showing a "Coming soon" stub for prompt logs. This spec expands it into an index page with four sub-page routes.

### New route structure

```
packages/webui/src/routes/config/debug/
  +page.svelte              # Existing -- becomes index/overview
  +layout.svelte            # NEW -- optional sub-nav for debug pages
  prompts/
    +page.svelte            # NEW -- Prompt Logs stub
  api/
    +page.svelte            # NEW -- Forge API Logs stub
  events/
    +page.svelte            # NEW -- Event Logs stub
  ui/
    +page.svelte            # NEW -- UI/UX Logs stub
```

### Debug index page (`/config/debug`)

Modify the existing page to become an overview/index with links to each sub-page:

```svelte
<div class="debug-page">
  <h1 class="page-title">Debug</h1>
  <p class="subtitle">Logs, diagnostics, and audit trails</p>

  <div class="debug-cards">
    <a href="/config/debug/prompts" class="debug-card">
      <h3>Prompt Logs</h3>
      <p>AI derivation audit trail -- every prompt sent, every response received.</p>
    </a>
    <a href="/config/debug/api" class="debug-card">
      <h3>Forge API Logs</h3>
      <p>HTTP request/response log for the Forge backend API.</p>
    </a>
    <a href="/config/debug/events" class="debug-card">
      <h3>Event Logs</h3>
      <p>Application event stream -- entity creates, updates, deletes.</p>
    </a>
    <a href="/config/debug/ui" class="debug-card">
      <h3>UI/UX Logs</h3>
      <p>Client-side errors, performance metrics, and interaction traces.</p>
    </a>
  </div>
</div>
```

### Debug sub-page stubs

Each sub-page follows the same pattern:

```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="debug-subpage">
  <h1 class="page-title">{title}</h1>
  <p class="subtitle">{subtitle}</p>

  <EmptyState
    title="Coming soon"
    description="{description}"
  />
</div>
```

**Prompt Logs** (`/config/debug/prompts`):
- Title: "Prompt Logs"
- Subtitle: "AI derivation audit trail"
- Description: "Prompt logs will appear here after AI derivation runs."

**Forge API Logs** (`/config/debug/api`):
- Title: "Forge API Logs"
- Subtitle: "Backend HTTP request log"
- Description: "API request and response logs will appear here when the logging endpoint is implemented."

**Event Logs** (`/config/debug/events`):
- Title: "Event Logs"
- Subtitle: "Application event stream"
- Description: "Entity lifecycle events (create, update, delete) will appear here."

**UI/UX Logs** (`/config/debug/ui`):
- Title: "UI/UX Logs"
- Subtitle: "Client-side diagnostics"
- Description: "Client-side errors, performance metrics, and interaction traces will appear here."

### Debug layout (optional)

A minimal `+layout.svelte` in `config/debug/` can add breadcrumb-style sub-navigation:

```svelte
<script>
  import { page } from '$app/state'
  let { children } = $props()

  const debugLinks = [
    { href: '/config/debug', label: 'Overview' },
    { href: '/config/debug/prompts', label: 'Prompts' },
    { href: '/config/debug/api', label: 'API' },
    { href: '/config/debug/events', label: 'Events' },
    { href: '/config/debug/ui', label: 'UI/UX' },
  ]
</script>

<div class="debug-layout">
  <nav class="debug-nav">
    {#each debugLinks as link}
      <a href={link.href} class:active={page.url.pathname === link.href}>{link.label}</a>
    {/each}
  </nav>
  {@render children()}
</div>
```

---

## 6. Stub Pages

The following menu items link to pages that do not yet exist. Each gets a minimal stub page with `EmptyState`.

### Account stubs

No route needed -- the "Login / Logout" menu item shows an inline "Coming Soon" label directly in the profile menu. It is not a link.

### Config stubs

No route needed -- "Plugins" and "External APIs" show inline "Coming Soon" labels in the profile menu. They are not links.

### Settings stubs

No route needed -- "Data Privacy Visualization" and "Privacy Settings" show inline "Coming Soon" labels in the profile menu. They are not links.

### About stubs

No route needed -- "Privacy Policy", "About Forge", and "Documentation" show inline "Coming Soon" labels in the profile menu. They are not links.

### Summary of stub approach

Only items that link to actual routes need stub pages. Items marked "Coming Soon" are rendered as disabled menu rows directly in `ProfileMenu.svelte` -- no route, no page file. This keeps the file count minimal and avoids creating routes that serve no purpose.

Stub pages created (4 total, all under debug):
- `packages/webui/src/routes/config/debug/prompts/+page.svelte`
- `packages/webui/src/routes/config/debug/api/+page.svelte`
- `packages/webui/src/routes/config/debug/events/+page.svelte`
- `packages/webui/src/routes/config/debug/ui/+page.svelte`

---

## 7. Remove Config Nav Group

**File:** `packages/webui/src/lib/nav.ts`

### Current state

```typescript
{
  label: 'Config',
  prefix: '/config',
  children: [
    { href: '/config/profile', label: 'Profile' },
    { href: '/config/export', label: 'Export' },
    { href: '/config/debug', label: 'Debug (Logs)' },
  ],
},
```

### Change

Remove the entire Config `NavGroup` entry from the `navigation` array. All config access is now through the profile menu.

```typescript
export const navigation: NavEntry[] = [
  { href: '/', label: 'Dashboard' },
  {
    label: 'Experience',
    prefix: '/experience',
    children: [
      { href: '/experience/roles', label: 'Roles' },
      { href: '/experience/projects', label: 'Projects' },
      { href: '/experience/education', label: 'Education' },
      { href: '/experience/clearances', label: 'Clearances' },
      { href: '/experience/general', label: 'General' },
    ],
  },
  {
    label: 'Data',
    prefix: '/data',
    children: [
      { href: '/data/bullets', label: 'Bullets' },
      { href: '/data/skills', label: 'Skills' },
      { href: '/data/organizations', label: 'Organizations' },
      { href: '/data/domains', label: 'Domains' },
      { href: '/data/notes', label: 'Notes' },
    ],
  },
  {
    label: 'Opportunities',
    prefix: '/opportunities',
    children: [
      { href: '/opportunities/organizations', label: 'Organizations' },
      { href: '/opportunities/job-descriptions', label: 'Job Descriptions' },
    ],
  },
  {
    label: 'Resumes',
    prefix: '/resumes',
    children: [
      { href: '/resumes', label: 'Builder' },
      { href: '/resumes/summaries', label: 'Summaries' },
      { href: '/resumes/templates', label: 'Templates' },
    ],
  },
  // Config nav group removed -- all config access through profile menu
]
```

### Impact on nav.test.ts

The existing `nav.test.ts` file will need its assertions updated to reflect the removal of the Config group. Specifically, any test that counts navigation entries or asserts the presence of Config children must be updated.

---

## 8. Files Summary

### Files to create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/ProfileMenu.svelte` | Profile flyout menu component |
| `packages/webui/src/lib/format.ts` | Phone formatting utility |
| `packages/webui/src/routes/config/debug/+layout.svelte` | Debug sub-navigation layout |
| `packages/webui/src/routes/config/debug/prompts/+page.svelte` | Prompt Logs stub |
| `packages/webui/src/routes/config/debug/api/+page.svelte` | Forge API Logs stub |
| `packages/webui/src/routes/config/debug/events/+page.svelte` | Event Logs stub |
| `packages/webui/src/routes/config/debug/ui/+page.svelte` | UI/UX Logs stub |

### Files to modify

| File | Changes |
|------|---------|
| `packages/webui/src/routes/+layout.svelte` | Add profile button to sidebar bottom, import ProfileMenu, fetch profile on mount |
| `packages/webui/src/lib/nav.ts` | Remove Config nav group |
| `packages/webui/src/lib/nav.test.ts` | Update assertions for removed Config group |
| `packages/webui/src/routes/config/debug/+page.svelte` | Convert from stub to debug index/overview page |
| `packages/webui/src/lib/components/index.ts` | Export ProfileMenu |

---

## 9. Testing

### Unit tests

1. **`format.ts` -- `formatPhone()`**: Test all documented cases:
   - `"+15551234567"` -> `"+1 (555) 123-4567"`
   - `"+442071234567"` -> `"+44 207 123 4567"` (international)
   - `"5551234567"` -> `"(555) 123-4567"`
   - `""` -> `""`
   - `null` -> `""`
   - `undefined` -> `""`

2. **`nav.test.ts`**: Verify Config group is no longer in navigation array.

### Visual / manual tests

1. **Profile button visibility:** Profile button appears at the bottom of the sidebar on every page. It shows the user's initials and name.
2. **Menu open/close:** Click profile button to open menu. Click outside to close. Press Escape to close. Click profile button again to close.
3. **Profile info:** Menu shows name, email, formatted phone, location, and link icons. "Edit Profile" link navigates to `/config/profile`.
4. **Theme toggle:** Click Light/Dark/System buttons. Verify the UI theme changes immediately. Reload the page and verify the theme persists.
5. **Navigation links:** Click "Export Data" -- navigates to `/config/export`. Click debug sub-page links -- navigate correctly. Menu closes on navigation.
6. **Coming Soon items:** Account, Config (Plugins/External APIs), Settings, and About items are visible but disabled/grayed with "Coming Soon" text. They are not clickable links.
7. **Debug sub-pages:** Navigate to `/config/debug` -- shows overview with cards linking to sub-pages. Each sub-page shows the correct title and "Coming soon" empty state. Sub-navigation tabs work.
8. **Config nav removed:** The Config group no longer appears in the sidebar navigation. Expanding all groups shows only Experience, Data, Opportunities, and Resumes.
9. **Direct URL access:** Typing `/config/profile`, `/config/export`, `/config/debug` directly in the browser still works (routes still exist, just not in the sidebar nav).

---

## 10. Acceptance Criteria

1. **Profile button** renders at the bottom of the sidebar with user initials and name on every page.
2. **Profile menu** opens as a popover anchored to the sidebar bottom, expanding upward.
3. **Profile section** displays name, email, formatted phone (`+1 (555) 123-4567`), location, and social links. Links to `/config/profile` for editing.
4. **Theme toggle** in the menu switches between light/dark/system. Selection persists across page reloads via localStorage. `data-theme` attribute on `<html>` updates correctly.
5. **Export link** navigates to `/config/export`.
6. **Debug section** links to `/config/debug` (overview) and four sub-pages (`/config/debug/prompts`, `/config/debug/api`, `/config/debug/events`, `/config/debug/ui`).
7. **Debug index page** shows card-style links to each sub-page with descriptions.
8. **Debug sub-pages** render as stubs with `EmptyState` "Coming soon" messages.
9. **Debug layout** includes sub-navigation tabs across all debug pages.
10. **Coming Soon items** (Account, Config, Settings, About) are rendered as disabled/grayed menu rows, not links.
11. **Config nav group** is removed from `$lib/nav.ts` and no longer appears in the sidebar.
12. **`nav.test.ts`** passes with updated assertions.
13. **`formatPhone()` unit tests** pass for all documented cases.
14. **Click-outside and Escape** close the profile menu.
15. **No regressions** -- all existing pages remain accessible and functional.

---

## Non-Goals

- **Authentication / real login** -- Login/Logout is a stub placeholder only
- **Avatar image upload** -- profile button uses initials, not an image
- **Input masking for phone** -- the profile edit page does not enforce phone format on input; formatting is display-only
- **Plugin system** -- "Plugins" menu item is a stub
- **External API integrations** -- "External APIs" menu item is a stub
- **Privacy features** -- "Data Privacy Visualization" and "Privacy Settings" are stubs
- **Documentation site** -- "Documentation" menu item is a stub
- **Multi-user / account switching** -- single-user app
- **Responsive / mobile sidebar** -- not in scope for this spec
