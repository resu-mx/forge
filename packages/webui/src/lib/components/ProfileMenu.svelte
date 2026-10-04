<script lang="ts">
  import { goto } from '$app/navigation'
  import { formatPhone } from '$lib/format'
  import { extensionPagesAvailable } from '$lib/extension-gate'

  /**
   * Profile flyout menu component.
   *
   * Props:
   * - profile: User profile data (UserProfile shape from API) or null
   * - isOpen: Whether the menu is currently visible
   * - onclose: Callback to close the menu
   * - buttonEl: Reference to the profile button element (for click-outside exclusion and positioning)
   */
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
      location?: string
      github?: string
      linkedin?: string
      website?: string
    } | null
    isOpen: boolean
    onclose: () => void
    buttonEl?: HTMLElement
  } = $props()

  let menuEl: HTMLDivElement

  // Fixed position for the menu (computed from buttonEl bounding rect)
  let menuStyle = $state('')

  // Theme state — uses localStorage key 'forge-theme'
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

  // Compute fixed position when the menu opens, using the profile button's bounding rect
  // so the menu escapes the sidebar's stacking context and renders above all content.
  $effect(() => {
    if (isOpen && buttonEl) {
      const rect = buttonEl.getBoundingClientRect()
      const menuWidth = 280
      menuStyle = `left: ${rect.left}px; bottom: ${window.innerHeight - rect.top + 8}px; width: ${menuWidth}px;`
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
  // The profile button element reference (`buttonEl` prop) must be excluded
  // in handlePointerDown. Without this exclusion, clicking the profile button
  // fires both the button's onclick (toggles isOpen=true) and handlePointerDown
  // (sets isOpen=false), causing the menu to never open.
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
  let displayLocation = $derived(profile?.location ?? '')
  let initials = $derived.by(() => {
    if (!profile?.name) return '?'
    return profile.name
      .split(' ')
      .map(w => w[0])
      .join('')
      .slice(0, 2)
      .toUpperCase()
  })

  // Social links — match UserProfile field names from API
  let socialLinks = $derived.by(() => {
    const links: { label: string; url: string; icon: string }[] = []
    if (profile?.github) links.push({ label: 'GitHub', url: profile.github, icon: 'GH' })
    if (profile?.linkedin) links.push({ label: 'LinkedIn', url: profile.linkedin, icon: 'LI' })
    if (profile?.website) links.push({ label: 'Website', url: profile.website, icon: 'WB' })
    return links
  })
</script>

{#if isOpen}
  <div class="profile-menu" bind:this={menuEl} style={menuStyle}>
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
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/account')}>
        Login / Logout
      </button>
    </div>

    <!-- Config section -->
    <div class="menu-section">
      <span class="section-header">Config</span>
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/plugins')}>
        Plugins
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/integrations')}>
        Integrations
      </button>
    </div>

    <!-- Settings section -->
    <div class="menu-section">
      <span class="section-header">Settings</span>
      <button class="menu-item menu-link" onclick={() => navigateTo('/settings/work-auth')}>
        Work Authorization
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/settings/eeo')}>
        EEO Disclosures
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/settings/storage')}>
        Storage
      </button>
      {#if extensionPagesAvailable}
        <button class="menu-item menu-link" onclick={() => navigateTo('/settings/extension')}>
          Extension Config
        </button>
        <button class="menu-item menu-link" onclick={() => navigateTo('/settings/extension-logs')}>
          Extension Logs
        </button>
      {/if}
      <button class="menu-item menu-link" onclick={() => navigateTo('/config/privacy')}>
        Privacy
      </button>
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
      <button class="menu-item menu-link" onclick={() => navigateTo('/about')}>
        About Forge
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/docs')}>
        Documentation
      </button>
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
    position: fixed;
    max-height: calc(100vh - 80px);
    overflow-y: auto;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    z-index: var(--z-modal);
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
