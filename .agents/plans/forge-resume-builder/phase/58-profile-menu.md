# Phase 58: Profile Button & User Menu (Spec C)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-profile-menu.md](../refs/specs/2026-04-03-profile-menu.md)
**Depends on:** Phase 42 (Design System — tokens for theming, base classes for buttons/modals)
**Blocks:** None
**Parallelizable with:** Phase 57 (UI Consistency — different files except `+layout.svelte`, see notes), Phase 59 (ECharts Infrastructure — no overlapping files)

## Goal

Replace the sidebar's Config nav group with a profile button anchored to the bottom of the sidebar, modeled after Slack and Discord's user area. Clicking the button opens a flyout/popover menu with quick-view profile info, links to config pages, and a theme toggle. This consolidates all config/settings access into a single, always-visible entry point and removes the Config group from the main navigation.

## Non-Goals

- Authentication / real login (Login/Logout is a stub placeholder only)
- Avatar image upload (profile button uses initials, not an image)
- Input masking for phone (the profile edit page does not enforce phone format on input; formatting is display-only)
- Plugin system ("Plugins" menu item is a stub)
- External API integrations ("External APIs" menu item is a stub)
- Privacy features ("Data Privacy Visualization" and "Privacy Settings" are stubs)
- Documentation site ("Documentation" menu item is a stub)
- Multi-user / account switching (single-user app)
- Responsive / mobile sidebar

## Context

The sidebar currently displays a "Config" nav group containing Profile, Export, and Debug links. This phase moves all config access into a profile menu popover, freeing sidebar real estate for primary navigation. The profile menu also introduces the theme toggle UI that Phase 42's `themeStore` was built to support.

The existing `+layout.svelte` imports `navigation` and `isNavGroup` from `$lib/nav.ts` and renders the sidebar. The sidebar is a simple flex-column layout. The `nav.test.ts` currently asserts 5 top-level entries including Config with 4 children (the test says 4 but the actual nav.ts shows 3 children — this is an existing test discrepancy that needs resolution).

The existing `config/debug/+page.svelte` is a single-page "Prompt Logs" stub. This phase expands it into an index/overview page with four sub-page routes.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Profile Button (Sidebar Bottom) | Yes |
| 2. Profile Menu (Flyout/Popover) | Yes |
| 3. Phone Number Formatting | Yes |
| 4. Theme Toggle | Yes |
| 5. Debug Sub-Pages | Yes |
| 6. Stub Pages | Yes |
| 7. Remove Config Nav Group | Yes |
| 8. Files Summary | Yes |
| 9. Testing | Yes |
| 10. Acceptance Criteria | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/ProfileMenu.svelte` | Profile flyout menu component |
| `packages/webui/src/lib/format.ts` | Phone formatting utility |
| `packages/webui/src/lib/format.test.ts` | Unit tests for `formatPhone()` |
| `packages/webui/src/routes/config/debug/+layout.svelte` | Debug sub-navigation layout |
| `packages/webui/src/routes/config/debug/prompts/+page.svelte` | Prompt Logs stub |
| `packages/webui/src/routes/config/debug/api/+page.svelte` | Forge API Logs stub |
| `packages/webui/src/routes/config/debug/events/+page.svelte` | Event Logs stub |
| `packages/webui/src/routes/config/debug/ui/+page.svelte` | UI/UX Logs stub |

## Files to Modify

| File | Changes |
|------|---------|
| `packages/webui/src/routes/+layout.svelte` | Add profile button to sidebar bottom, import ProfileMenu, fetch profile on mount, restructure sidebar flex layout |
| `packages/webui/src/lib/nav.ts` | Remove Config nav group |
| `packages/webui/src/lib/nav.test.ts` | Update assertions for removed Config group |
| `packages/webui/src/routes/config/debug/+page.svelte` | Convert from single stub to debug index/overview page |
| `packages/webui/src/lib/components/index.ts` | Export ProfileMenu |

## Fallback Strategies

- **Phase 42 `themeStore` not yet merged:** If `$lib/stores/theme.svelte.ts` does not exist at implementation time, the theme toggle section of ProfileMenu creates a minimal inline store: `let currentTheme = $state<'light' | 'dark' | 'system'>('system')` with `localStorage` read/write and `document.documentElement.setAttribute('data-theme', ...)`. When Phase 42 lands, this inline store is replaced with the shared `themeStore` import.
- **Profile API (`forge.profile.get()`) not available:** If the SDK does not yet expose a profile endpoint, the profile button shows placeholder data: `initials = '?'`, `name = 'User'`. The menu shows "Set up your profile" link to `/config/profile` instead of profile details. The profile data loading code is wrapped in a try/catch that falls back to the placeholder.
- **Click-outside detection fails on touch devices:** The `pointerdown` listener for click-outside uses `event.target` containment check. On touch devices, `pointerdown` may fire differently. Fallback: add a `touchstart` listener alongside `pointerdown` with the same logic. Alternatively, use a transparent overlay `<div>` behind the menu that captures clicks.
- **Popover positioning overflows viewport:** The menu expands upward from the sidebar bottom. If the sidebar is very short (small viewport), the menu may extend beyond the top of the viewport. Fallback: add `max-height: calc(100vh - 80px)` and `overflow-y: auto` to the menu (already specified in the spec's styling section).
- **Debug sub-page routing conflict:** The existing `config/debug/+page.svelte` renders at `/config/debug`. Adding a `+layout.svelte` in the same directory wraps both the index page and sub-pages. If the layout's `{@render children()}` conflicts with the existing page, ensure the layout is minimal (just nav + slot) with no duplicate content.

---

## Tasks

### T58.1: Create Phone Formatting Utility

**File:** `packages/webui/src/lib/format.ts`

[IMPORTANT] This utility is used by ProfileMenu to display the phone number. It must handle all documented cases without crashing on unexpected input.

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
  if (raw.startsWith('+') && digits.length > 10) {
    // Determine country code length (1-3 digits).
    // Heuristic: if total digits > 12, cc is 3; > 11, cc is 2; else 1.
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

**Acceptance criteria:**
- `formatPhone('+15551234567')` returns `'+1 (555) 123-4567'`
- `formatPhone('+442071234567')` returns `'+44 207 123 4567'`
- `formatPhone('5551234567')` returns `'(555) 123-4567'`
- `formatPhone('')` returns `''`
- `formatPhone(null)` returns `''`
- `formatPhone(undefined)` returns `''`
- Non-digit characters in input are stripped before processing.

**Failure criteria:**
- Function throws on null/undefined input.
- US number with country code omits the `+` prefix.
- International grouping produces a trailing lone digit.

---

### T58.2: Write `formatPhone` Unit Tests

**File:** `packages/webui/src/lib/format.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { formatPhone } from './format'

describe('formatPhone', () => {
  it('formats US number with country code', () => {
    expect(formatPhone('+15551234567')).toBe('+1 (555) 123-4567')
  })

  it('formats US number with country code (digits only, no +)', () => {
    expect(formatPhone('15551234567')).toBe('+1 (555) 123-4567')
  })

  it('formats US number without country code', () => {
    expect(formatPhone('5551234567')).toBe('(555) 123-4567')
  })

  it('formats international number (UK)', () => {
    expect(formatPhone('+442071234567')).toBe('+44 207 123 4567')
  })

  it('returns empty string for empty input', () => {
    expect(formatPhone('')).toBe('')
  })

  it('returns empty string for null', () => {
    expect(formatPhone(null)).toBe('')
  })

  it('returns empty string for undefined', () => {
    expect(formatPhone(undefined)).toBe('')
  })

  it('strips non-digit characters before processing', () => {
    expect(formatPhone('+1 (555) 123-4567')).toBe('+1 (555) 123-4567')
  })

  it('returns raw input for unrecognized formats', () => {
    expect(formatPhone('12345')).toBe('12345')
  })

  it('handles number with dashes and spaces', () => {
    expect(formatPhone('555-123-4567')).toBe('(555) 123-4567')
  })

  it('handles number with parentheses', () => {
    expect(formatPhone('(555) 123-4567')).toBe('(555) 123-4567')
  })
})
```

**Acceptance criteria:**
- All 11 test cases pass.
- Edge cases (null, undefined, empty, non-digit characters, unrecognized formats) are covered.

**Failure criteria:**
- Any test fails, indicating a formatting bug.

---

### T58.3: Create ProfileMenu Component

**File:** `packages/webui/src/lib/components/ProfileMenu.svelte`

[CRITICAL] The menu must close on click-outside AND on Escape. The click-outside check must exclude both the menu element and the profile button element to prevent the menu from closing and immediately reopening when the button is clicked.

[IMPORTANT] "Coming Soon" items are rendered as disabled menu rows directly in the component — no routes, no links. Only items with actual destinations are rendered as `<a>` tags.

[IMPORTANT] The theme toggle reads from and writes to `themeStore` (Phase 42). If the store does not exist yet, a local fallback is used.

```svelte
<script lang="ts">
  import { page } from '$app/state'
  import { goto } from '$app/navigation'
  import { formatPhone } from '$lib/format'

  let {
    profile,
    isOpen = false,
    onclose,
    buttonEl,
  }: {
    profile: {
      name?: string
      email?: string
      phone?: string
      city?: string
      state?: string
      github_url?: string
      linkedin_url?: string
      portfolio_url?: string
      blog_url?: string
    } | null
    isOpen: boolean
    onclose: () => void
    buttonEl?: HTMLElement
  } = $props()

  let menuEl: HTMLDivElement

  // Theme state — uses themeStore if available, local fallback otherwise
  let currentTheme = $state<'light' | 'dark' | 'system'>('system')

  // Read initial theme from localStorage on mount
  $effect(() => {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem('forge-theme')
      if (stored === 'light' || stored === 'dark' || stored === 'system') {
        currentTheme = stored
      }
    }
  })

  function setTheme(theme: 'light' | 'dark' | 'system') {
    currentTheme = theme
    localStorage.setItem('forge-theme', theme)
    if (theme === 'system') {
      document.documentElement.removeAttribute('data-theme')
    } else {
      document.documentElement.setAttribute('data-theme', theme)
    }
  }

  // Close on Escape
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isOpen) {
      onclose()
    }
  }

  // Close on click outside.
  // [FIX] The profile button element reference (`buttonEl` prop) must be
  // passed to ProfileMenu and excluded in handlePointerDown. Without this
  // exclusion, clicking the profile button fires both the button's onclick
  // (toggles isOpen=true) and handlePointerDown (sets isOpen=false), causing
  // the menu to never open.
  function handlePointerDown(event: PointerEvent) {
    if (!isOpen || !menuEl) return
    const target = event.target as Node
    if (!menuEl.contains(target) && !buttonEl?.contains(target)) onclose()
  }

  // Register global listeners when open
  $effect(() => {
    if (!isOpen) return
    window.addEventListener('keydown', handleKeydown)
    window.addEventListener('pointerdown', handlePointerDown)
    return () => {
      window.removeEventListener('keydown', handleKeydown)
      window.removeEventListener('pointerdown', handlePointerDown)
    }
  })

  // Navigate and close menu
  function navigateTo(href: string) {
    goto(href)
    onclose()
  }

  // Derived display values
  let displayPhone = $derived(formatPhone(profile?.phone))
  let displayLocation = $derived.by(() => {
    const parts = [profile?.city, profile?.state].filter(Boolean)
    return parts.join(', ')
  })
  let initials = $derived.by(() => {
    if (!profile?.name) return '?'
    return profile.name
      .split(' ')
      .map(w => w[0])
      .join('')
      .slice(0, 2)
      .toUpperCase()
  })

  // Social links
  let socialLinks = $derived.by(() => {
    const links: { label: string; url: string; icon: string }[] = []
    if (profile?.github_url) links.push({ label: 'GitHub', url: profile.github_url, icon: 'GH' })
    if (profile?.linkedin_url) links.push({ label: 'LinkedIn', url: profile.linkedin_url, icon: 'LI' })
    if (profile?.portfolio_url) links.push({ label: 'Portfolio', url: profile.portfolio_url, icon: 'PF' })
    if (profile?.blog_url) links.push({ label: 'Blog', url: profile.blog_url, icon: 'BG' })
    return links
  })
</script>

{#if isOpen}
  <div class="profile-menu" bind:this={menuEl}>
    <!-- Profile section -->
    <div class="menu-section profile-section">
      <div class="profile-header">
        <span class="menu-avatar">{initials}</span>
        <div class="profile-info">
          <span class="profile-name-display">{profile?.name ?? 'User'}</span>
        </div>
      </div>
      {#if profile?.email}
        <span class="profile-detail">{profile.email}</span>
      {/if}
      {#if displayPhone}
        <span class="profile-detail">{displayPhone}</span>
      {/if}
      {#if displayLocation}
        <span class="profile-detail">{displayLocation}</span>
      {/if}
      {#if socialLinks.length > 0}
        <div class="social-links">
          {#each socialLinks as link}
            <a href={link.url} target="_blank" rel="noopener noreferrer" class="social-link" title={link.label}>
              {link.icon}
            </a>
          {/each}
        </div>
      {/if}
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/profile')}>
        Edit Profile &rarr;
      </button>
    </div>

    <!-- Account section -->
    <div class="menu-section">
      <span class="section-header">Account</span>
      <span class="menu-item disabled">Login / Logout <span class="coming-soon">Coming Soon</span></span>
    </div>

    <!-- Config section -->
    <div class="menu-section">
      <span class="section-header">Config</span>
      <span class="menu-item disabled">Plugins <span class="coming-soon">Coming Soon</span></span>
      <span class="menu-item disabled">External APIs <span class="coming-soon">Coming Soon</span></span>
    </div>

    <!-- Settings section -->
    <div class="menu-section">
      <span class="section-header">Settings</span>
      <span class="menu-item disabled">Data Privacy Viz <span class="coming-soon">Coming Soon</span></span>
      <span class="menu-item disabled">Privacy Settings <span class="coming-soon">Coming Soon</span></span>
    </div>

    <!-- Export section -->
    <div class="menu-section">
      <span class="section-header">Export</span>
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/export')}>
        Export Data
      </button>
    </div>

    <!-- Debug section -->
    <div class="menu-section">
      <span class="section-header">Debug</span>
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/debug')}>
        Overview
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/debug/prompts')}>
        Prompt Logs
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/debug/api')}>
        Forge API Logs
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/debug/events')}>
        Event Logs
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/debug/ui')}>
        UI/UX Logs
      </button>
    </div>

    <!-- About section -->
    <div class="menu-section">
      <span class="section-header">About</span>
      <span class="menu-item disabled">Privacy Policy <span class="coming-soon">Coming Soon</span></span>
      <span class="menu-item disabled">About Forge <span class="coming-soon">Coming Soon</span></span>
      <span class="menu-item disabled">Documentation <span class="coming-soon">Coming Soon</span></span>
    </div>

    <!-- Theme toggle -->
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
  </div>
{/if}

<style>
  .profile-menu {
    position: absolute;
    bottom: 100%;
    left: 0;
    width: 280px;
    max-height: calc(100vh - 80px);
    overflow-y: auto;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    z-index: 1000;
    margin-bottom: var(--space-2);
  }

  .menu-section {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
  }

  .menu-section:last-of-type {
    border-bottom: none;
  }

  .profile-section {
    padding: var(--space-3) var(--space-4);
  }

  .profile-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-2);
  }

  .menu-avatar {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-full);
    background: var(--color-primary);
    color: var(--text-inverse);
    font-size: var(--text-sm);
    font-weight: var(--font-semibold);
    flex-shrink: 0;
  }

  .profile-name-display {
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
  }

  .profile-detail {
    display: block;
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-1);
  }

  .social-links {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .social-link {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-weight: var(--font-semibold);
    text-decoration: none;
    transition: background 0.15s ease;
  }

  .social-link:hover {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
  }

  .section-header {
    display: block;
    font-size: var(--text-xs);
    font-weight: var(--font-semibold);
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: var(--space-1);
  }

  .menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-1) var(--space-2);
    border: none;
    background: none;
    font-size: var(--text-sm);
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    cursor: default;
    text-align: left;
  }

  .menu-link {
    cursor: pointer;
  }

  .menu-link:hover {
    background: var(--color-surface-raised);
    color: var(--text-primary);
  }

  .menu-item.disabled {
    color: var(--text-faint);
    cursor: default;
  }

  .coming-soon {
    font-size: var(--text-xs);
    color: var(--text-faint);
    font-style: italic;
  }

  .theme-toggle {
    padding: var(--space-3) var(--space-4);
    border-top: 1px solid var(--color-border);
  }

  .theme-label {
    display: block;
    font-size: var(--text-xs);
    font-weight: var(--font-semibold);
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: var(--space-2);
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
    transition: background 0.15s ease, color 0.15s ease;
  }

  .theme-btn:not(:last-child) {
    border-right: 1px solid var(--color-border);
  }

  .theme-btn:hover {
    background: var(--color-surface-raised);
  }

  .theme-btn.active {
    background: var(--color-primary);
    color: var(--text-inverse);
  }
</style>
```

**Acceptance criteria:**
- Menu renders when `isOpen` is true.
- Profile section shows name, email, formatted phone, location, social links.
- "Edit Profile" navigates to `/config/profile` and closes menu.
- "Coming Soon" items are visible but not clickable.
- Debug links navigate to correct routes and close menu.
- Theme toggle switches between light/dark/system.
- Theme persists across page reloads via localStorage.
- `data-theme` attribute updates on `<html>` element.
- Click-outside closes the menu.
- Escape key closes the menu.

**Failure criteria:**
- Menu does not close on click-outside (listener not registered or containment check wrong).
- Theme toggle does not persist (localStorage key mismatch).
- Navigation does not close the menu (missing `onclose()` call after `goto()`).
- Menu overflows viewport with no scroll (missing `max-height` or `overflow-y: auto`).

---

### T58.4: Add Profile Button to Layout Sidebar

**File:** `packages/webui/src/routes/+layout.svelte`

[CRITICAL] This spec owns the sidebar flex-column restructuring. Change `.sidebar` to ensure `.nav-list` grows to fill available space and the profile button is pinned to the bottom.

[IMPORTANT] On mount, `+layout.svelte` fetches the user profile via `forge.profile.get()` to populate the profile button (name, initials) and the profile menu (email, phone, location, links). This is a single API call, cached in a layout-level `$state`.

Add to `<script>`:
```typescript
import ProfileMenu from '$lib/components/ProfileMenu.svelte'
import { forge } from '$lib/sdk'

let profileData = $state<any>(null)
let menuOpen = $state(false)
let profileButtonEl: HTMLButtonElement

let initials = $derived.by(() => {
  if (!profileData?.name) return '?'
  return profileData.name
    .split(' ')
    .map((w: string) => w[0])
    .join('')
    .slice(0, 2)
    .toUpperCase()
})

let profileName = $derived(profileData?.name ?? 'User')

function toggleProfileMenu() {
  menuOpen = !menuOpen
}

// Fetch profile on mount — use onMount, NOT $effect.
// $effect would re-run when profileData is written, causing an infinite loop.
import { onMount } from 'svelte'

onMount(() => {
  async function loadProfile() {
    try {
      const result = await forge.profile.get()
      if (result.ok) {
        profileData = result.data
      }
    } catch {
      // Profile not available — use placeholder
    }
  }
  loadProfile()
})
```

Add to template, inside `.sidebar` after the `.nav-list` `</ul>`:
```svelte
<div class="profile-button-area">
  <button class="profile-button" onclick={toggleProfileMenu} bind:this={profileButtonEl}>
    <span class="profile-avatar">{initials}</span>
    <span class="profile-name">{profileName}</span>
    <span class="profile-gear">&#9881;</span>
  </button>
  <ProfileMenu
    profile={profileData}
    isOpen={menuOpen}
    onclose={() => menuOpen = false}
    buttonEl={profileButtonEl}
  />
</div>
```

Add/modify CSS:
```css
.sidebar {
  display: flex;
  flex-direction: column;
  /* existing sidebar styles preserved */
}

.nav-list {
  flex: 1;
  overflow-y: auto;
}

.profile-button-area {
  position: relative;
  border-top: 1px solid var(--color-sidebar-border, var(--color-border));
  padding: var(--space-3) var(--space-4);
  flex-shrink: 0;
}

.profile-button {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-2);
  border: none;
  background: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  color: var(--text-secondary);
  transition: background 0.15s ease;
}

.profile-button:hover {
  background: var(--color-surface-raised);
}

.profile-avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-full);
  background: var(--color-primary);
  color: var(--text-inverse);
  font-size: var(--text-xs);
  font-weight: var(--font-semibold);
  flex-shrink: 0;
}

.profile-name {
  flex: 1;
  text-align: left;
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.profile-gear {
  font-size: var(--text-base);
  color: var(--text-muted);
}
```

**Acceptance criteria:**
- Profile button appears at the bottom of the sidebar on every page.
- Button shows user initials in a circle, name, and gear icon.
- Clicking the button opens the profile menu popover.
- Clicking again closes it.
- Profile data loads on mount (or placeholder shown if API unavailable).
- `.nav-list` scrolls independently when content exceeds sidebar height.
- Profile button area does not scroll with the nav list.

**Failure criteria:**
- Profile button overlaps nav list items (flex layout not correct).
- Profile menu opens behind sidebar content (z-index issue).
- Profile data fetch blocks sidebar rendering (should be async, non-blocking).
- Menu position is wrong (should anchor upward from profile button area).

---

### T58.5: Remove Config Nav Group

**File:** `packages/webui/src/lib/nav.ts`

[IMPORTANT] Remove the entire Config `NavGroup` entry from the `navigation` array. All config access is now through the profile menu. The routes still exist (`/config/profile`, `/config/export`, `/config/debug`) — only the sidebar navigation entry is removed.

Before:
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
  {
    label: 'Config',
    prefix: '/config',
    children: [
      { href: '/config/profile', label: 'Profile' },
      { href: '/config/export', label: 'Export' },
      { href: '/config/debug', label: 'Debug (Logs)' },
    ],
  },
]
```

After:
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
  // Config nav group removed — all config access through profile menu
]
```

**Acceptance criteria:**
- `navigation` array has 5 entries (Dashboard + 4 groups).
- No entry with `label: 'Config'` exists.
- Config routes still exist and are accessible via direct URL.

**Failure criteria:**
- Config routes are deleted (only the nav entry should be removed, not the routes).

---

### T58.6: Update Nav Tests

**File:** `packages/webui/src/lib/nav.test.ts`

[IMPORTANT] The existing test asserts "5 top-level entries" and "Config group with 4 children." Both assertions must be updated to reflect the removal.

[INCONSISTENCY] The existing test says Config has "4 children" but the actual `nav.ts` only has 3 children (Profile, Export, Debug). The test is already out of sync. After removing Config, this discrepancy resolves naturally because the Config test is removed entirely.

Updated test file:
```typescript
import { describe, it, expect } from 'vitest'
import { navigation, isNavGroup } from './nav'
import type { NavEntry, NavGroup, NavItem } from './nav'

describe('nav', () => {
  describe('isNavGroup', () => {
    it('returns true for entries with children', () => {
      const group: NavGroup = { label: 'Data', prefix: '/data', children: [{ href: '/data/x', label: 'X' }] }
      expect(isNavGroup(group)).toBe(true)
    })

    it('returns false for plain NavItem', () => {
      const item: NavItem = { href: '/', label: 'Dashboard' }
      expect(isNavGroup(item)).toBe(false)
    })
  })

  describe('navigation', () => {
    it('has 5 top-level entries', () => {
      expect(navigation).toHaveLength(5)
    })

    it('starts with Dashboard as a plain link', () => {
      expect(isNavGroup(navigation[0])).toBe(false)
      expect((navigation[0] as NavItem).href).toBe('/')
      expect((navigation[0] as NavItem).label).toBe('Dashboard')
    })

    it('has Experience group with 5 children', () => {
      const exp = navigation[1]
      expect(isNavGroup(exp)).toBe(true)
      expect((exp as NavGroup).label).toBe('Experience')
      expect((exp as NavGroup).children).toHaveLength(5)
    })

    it('has Data group with 5 children', () => {
      const data = navigation[2]
      expect(isNavGroup(data)).toBe(true)
      expect((data as NavGroup).label).toBe('Data')
      expect((data as NavGroup).children).toHaveLength(5)
    })

    it('has Opportunities group with 2 children', () => {
      const opp = navigation[3]
      expect(isNavGroup(opp)).toBe(true)
      expect((opp as NavGroup).label).toBe('Opportunities')
      expect((opp as NavGroup).children).toHaveLength(2)
    })

    it('has Resumes group with 3 children', () => {
      const res = navigation[4]
      expect(isNavGroup(res)).toBe(true)
      expect((res as NavGroup).label).toBe('Resumes')
      expect((res as NavGroup).children).toHaveLength(3)
    })

    it('does not contain Config group', () => {
      const labels = navigation
        .filter(isNavGroup)
        .map((g: NavGroup) => g.label)
      expect(labels).not.toContain('Config')
    })

    it('all hrefs start with /', () => {
      for (const entry of navigation) {
        if (isNavGroup(entry)) {
          for (const child of entry.children) {
            expect(child.href).toMatch(/^\//)
          }
        } else {
          expect(entry.href).toMatch(/^\//)
        }
      }
    })

    it('does not contain Chain View', () => {
      const labels: string[] = []
      for (const entry of navigation) {
        if (isNavGroup(entry)) {
          for (const child of entry.children) labels.push(child.label)
        } else {
          labels.push(entry.label)
        }
      }
      expect(labels).not.toContain('Chain View')
    })
  })
})
```

**Acceptance criteria:**
- All tests pass with the updated assertions.
- Config group absence is explicitly tested.
- Experience, Data, Opportunities, Resumes groups are correctly asserted.

**Failure criteria:**
- Test still references Config group.
- Test count assertions are wrong (cause false positives/negatives).

---

### T58.7: Create Debug Index Page

**File:** `packages/webui/src/routes/config/debug/+page.svelte` (modify existing)

[IMPORTANT] The existing page is a "Prompt Logs" stub. It becomes the debug index/overview page with card-style links to each sub-page.

```svelte
<script lang="ts">
  const debugPages = [
    {
      href: '/config/debug/prompts',
      title: 'Prompt Logs',
      description: 'AI derivation audit trail — every prompt sent, every response received.',
    },
    {
      href: '/config/debug/api',
      title: 'Forge API Logs',
      description: 'HTTP request/response log for the Forge backend API.',
    },
    {
      href: '/config/debug/events',
      title: 'Event Logs',
      description: 'Application event stream — entity creates, updates, deletes.',
    },
    {
      href: '/config/debug/ui',
      title: 'UI/UX Logs',
      description: 'Client-side errors, performance metrics, and interaction traces.',
    },
  ]
</script>

<div class="debug-page">
  <h1 class="page-title">Debug</h1>
  <p class="subtitle">Logs, diagnostics, and audit trails</p>

  <div class="debug-cards">
    {#each debugPages as page}
      <a href={page.href} class="debug-card">
        <h3>{page.title}</h3>
        <p>{page.description}</p>
      </a>
    {/each}
  </div>
</div>

<style>
  .debug-page {
    max-width: 800px;
  }

  .page-title {
    font-size: var(--text-2xl);
    font-weight: var(--font-bold);
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin-bottom: var(--space-6);
  }

  .debug-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: var(--space-4);
  }

  .debug-card {
    display: block;
    padding: var(--space-4);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    text-decoration: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .debug-card:hover {
    border-color: var(--color-primary);
    box-shadow: var(--shadow-md);
  }

  .debug-card h3 {
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    margin-bottom: var(--space-2);
  }

  .debug-card p {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }
</style>
```

**Acceptance criteria:**
- `/config/debug` shows overview with four card links.
- Each card links to the correct sub-page route.
- Cards have hover state (border color change, shadow).
- No hardcoded hex colors or font sizes.

**Failure criteria:**
- Old "Prompt Logs" stub content still visible.
- Card links are broken (wrong `href`).

---

### T58.8: Create Debug Layout and Sub-Page Stubs

**Files to create:**
- `packages/webui/src/routes/config/debug/+layout.svelte`
- `packages/webui/src/routes/config/debug/prompts/+page.svelte`
- `packages/webui/src/routes/config/debug/api/+page.svelte`
- `packages/webui/src/routes/config/debug/events/+page.svelte`
- `packages/webui/src/routes/config/debug/ui/+page.svelte`

**Debug layout:**
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
      <a
        href={link.href}
        class="debug-nav-link"
        class:active={page.url.pathname === link.href}
      >{link.label}</a>
    {/each}
  </nav>
  {@render children()}
</div>

<style>
  .debug-layout {
    padding: var(--space-4);
  }

  .debug-nav {
    display: flex;
    gap: var(--space-1);
    margin-bottom: var(--space-4);
    border-bottom: 1px solid var(--color-border);
    padding-bottom: var(--space-2);
  }

  .debug-nav-link {
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    text-decoration: none;
    border-radius: var(--radius-md);
    transition: background 0.15s ease, color 0.15s ease;
  }

  .debug-nav-link:hover {
    background: var(--color-surface-raised);
    color: var(--text-primary);
  }

  .debug-nav-link.active {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
    font-weight: var(--font-semibold);
  }
</style>
```

**Prompt Logs stub** (`prompts/+page.svelte`):
```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="debug-subpage">
  <h1 class="page-title">Prompt Logs</h1>
  <p class="subtitle">AI derivation audit trail</p>

  <EmptyState
    title="Coming soon"
    description="Prompt logs will appear here after AI derivation runs."
  />
</div>

<style>
  .page-title {
    font-size: var(--text-2xl);
    font-weight: var(--font-bold);
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin-bottom: var(--space-6);
  }
</style>
```

**Forge API Logs stub** (`api/+page.svelte`):
```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="debug-subpage">
  <h1 class="page-title">Forge API Logs</h1>
  <p class="subtitle">Backend HTTP request log</p>

  <EmptyState
    title="Coming soon"
    description="API request and response logs will appear here when the logging endpoint is implemented."
  />
</div>

<style>
  .page-title {
    font-size: var(--text-2xl);
    font-weight: var(--font-bold);
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin-bottom: var(--space-6);
  }
</style>
```

**Event Logs stub** (`events/+page.svelte`):
```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="debug-subpage">
  <h1 class="page-title">Event Logs</h1>
  <p class="subtitle">Application event stream</p>

  <EmptyState
    title="Coming soon"
    description="Entity lifecycle events (create, update, delete) will appear here."
  />
</div>

<style>
  .page-title {
    font-size: var(--text-2xl);
    font-weight: var(--font-bold);
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin-bottom: var(--space-6);
  }
</style>
```

**UI/UX Logs stub** (`ui/+page.svelte`):
```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="debug-subpage">
  <h1 class="page-title">UI/UX Logs</h1>
  <p class="subtitle">Client-side diagnostics</p>

  <EmptyState
    title="Coming soon"
    description="Client-side errors, performance metrics, and interaction traces will appear here."
  />
</div>

<style>
  .page-title {
    font-size: var(--text-2xl);
    font-weight: var(--font-bold);
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin-bottom: var(--space-6);
  }
</style>
```

**Acceptance criteria:**
- `/config/debug` shows overview with sub-navigation tabs.
- `/config/debug/prompts` shows "Prompt Logs" with EmptyState.
- `/config/debug/api` shows "Forge API Logs" with EmptyState.
- `/config/debug/events` shows "Event Logs" with EmptyState.
- `/config/debug/ui` shows "UI/UX Logs" with EmptyState.
- Sub-navigation tabs highlight the active page.
- All pages use design tokens (no hardcoded colors/fonts).

**Failure criteria:**
- Sub-page routes return 404.
- Layout wraps incorrectly (double content rendering).
- Sub-navigation active state does not match current route.

---

### T58.9: Export ProfileMenu from Component Index

**File:** `packages/webui/src/lib/components/index.ts`

Add export:
```typescript
export { default as ProfileMenu } from './ProfileMenu.svelte'
```

[MINOR] This export enables other consumers to import ProfileMenu, but the primary consumer (`+layout.svelte`) may import it directly. The export is for consistency with the existing pattern.

**Acceptance criteria:**
- `import { ProfileMenu } from '$lib/components'` works.

**Failure criteria:**
- Import fails because the file path is wrong.

---

## Testing Support

### Unit Tests

**File:** `packages/webui/src/lib/format.test.ts` (T58.2)

| Test | Assertion |
|------|-----------|
| US number with country code | `'+15551234567'` -> `'+1 (555) 123-4567'` |
| US number with country code (digits only) | `'15551234567'` -> `'+1 (555) 123-4567'` |
| US number without country code | `'5551234567'` -> `'(555) 123-4567'` |
| International number (UK) | `'+442071234567'` -> `'+44 207 123 4567'` |
| Empty string | `''` -> `''` |
| Null | `null` -> `''` |
| Undefined | `undefined` -> `''` |
| Strips non-digit characters | `'+1 (555) 123-4567'` -> `'+1 (555) 123-4567'` |
| Unrecognized format passthrough | `'12345'` -> `'12345'` |
| Dashes and spaces | `'555-123-4567'` -> `'(555) 123-4567'` |
| Parentheses | `'(555) 123-4567'` -> `'(555) 123-4567'` |

**File:** `packages/webui/src/lib/nav.test.ts` (T58.6)

| Test | Assertion |
|------|-----------|
| `isNavGroup` true for group | Entry with `children` returns `true` |
| `isNavGroup` false for item | Entry with `href` returns `false` |
| 5 top-level entries | `navigation.length === 5` |
| Dashboard is first, plain link | `href === '/'`, `label === 'Dashboard'` |
| Experience group with 5 children | `label === 'Experience'`, `children.length === 5` |
| Data group with 5 children | `label === 'Data'`, `children.length === 5` |
| Opportunities group with 2 children | `label === 'Opportunities'`, `children.length === 2` |
| Resumes group with 3 children | `label === 'Resumes'`, `children.length === 3` |
| No Config group | Group labels do not contain `'Config'` |
| All hrefs start with `/` | Regex `/^\//` matches all hrefs |
| No Chain View | Label list does not contain `'Chain View'` |

### Visual / Manual Tests

| Test | What to verify |
|------|---------------|
| Profile button visibility | Appears at sidebar bottom on every page. Shows initials and name. |
| Menu open/close | Click button to open. Click outside to close. Escape to close. Click button again to close. |
| Profile info | Shows name, email, formatted phone, location, link icons. "Edit Profile" navigates to `/config/profile`. |
| Theme toggle | Click Light/Dark/System. UI theme changes. Reload — theme persists. |
| Navigation links | "Export Data" navigates to `/config/export`. Debug links navigate correctly. Menu closes on navigation. |
| Coming Soon items | Account, Config, Settings, About items visible but disabled. Not clickable. |
| Debug sub-pages | `/config/debug` shows overview cards. Each sub-page shows correct stub. Sub-nav tabs work. |
| Config nav removed | Config group no longer in sidebar. Only Experience, Data, Opportunities, Resumes visible. |
| Direct URL access | `/config/profile`, `/config/export`, `/config/debug` still work via direct URL. |

### Regression Gate

Before merging:
1. `bun run check` passes (TypeScript compilation).
2. `bun run build` succeeds.
3. `bun test` passes — specifically `nav.test.ts` and `format.test.ts`.
4. Existing tests not modified by this phase continue to pass.

---

## Documentation Requirements

- No new documentation files.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline TSDoc comments on:
  - `formatPhone()`: parameter types, return value, edge cases.
  - `ProfileMenu.svelte` props: `profile`, `isOpen`, `onclose`.
- Inline code comments for:
  - Click-outside detection logic (containment check rationale).
  - Theme toggle localStorage key (`'forge-theme'`).
  - Profile API fallback behavior.
  - Debug layout `{@render children()}` pattern.

---

## Parallelization Notes

**Within this phase:**
- T58.1 (format utility) and T58.2 (format tests) can be developed first (no dependencies).
- T58.3 (ProfileMenu) depends on T58.1 (imports `formatPhone`).
- T58.4 (layout changes) depends on T58.3 (imports ProfileMenu).
- T58.5 (nav removal) and T58.6 (nav tests) are independent of T58.3/T58.4.
- T58.7 (debug index) and T58.8 (debug sub-pages) are independent of all others.
- T58.9 (component export) depends on T58.3.

**Recommended execution order:**
1. T58.1 + T58.2 (format utility + tests — foundational, no dependencies)
2. T58.5 + T58.6 (nav removal + tests — independent, can parallel with step 1)
3. T58.7 + T58.8 (debug pages — independent, can parallel with steps 1-2)
4. T58.3 (ProfileMenu — depends on T58.1)
5. T58.4 + T58.9 (layout + export — depends on T58.3)

**Cross-phase:**
- This phase depends on Phase 42 (tokens and `themeStore`).
- Phase 57 (UI Consistency) modifies route-level CSS but does not touch `+layout.svelte` sidebar structure or `nav.ts` — no conflict.
- Phase 59 (ECharts Infrastructure) creates new files only — no conflict.
- All three phases (57, 58, 59) can run in parallel.
- Exception: Phase 57's sweep (T57.5) spot-checks `+layout.svelte`. If it modifies CSS there, coordinate with T58.4. The spec says `+layout.svelte` CSS is "Already done in Spec A" so no conflict expected, but the profile button area CSS is new and should not overlap.
