<script lang="ts">
  import '$lib/styles/tokens.css'
  import '$lib/styles/base.css'
  import '$lib/stores/theme.svelte.ts'  // initializes theme on import
  import { onMount } from 'svelte'
  import { page } from '$app/state'
  import { ToastContainer } from '$lib/components'
  import { JDOverlayHost } from '$lib/components/overlays'
  import ProfileMenu from '$lib/components/ProfileMenu.svelte'
  import StorageBanner from '$lib/components/StorageBanner.svelte'
  import ChainViewModal from '$lib/components/ChainViewModal.svelte'
  import { chainViewState, closeChainView } from '$lib/stores/chain-view.svelte'
  import { navigation, isNavGroup } from '$lib/nav'
  import { forge } from '$lib/sdk'

  let { children } = $props()

  // Profile data for sidebar button and menu
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

  // Accordion expanded state: keyed by group prefix.
  // NOTE: Must use Record<string, boolean> with $state, NOT a Set.
  // Set.add() / Set.delete() are method calls that mutate without assignment,
  // so $effect and {#if} blocks won't re-trigger. Record property writes
  // (expanded[key] = value) ARE tracked by Svelte 5 runes.
  let expanded = $state<Record<string, boolean>>({})

  // Auto-expand group matching current route on navigation
  $effect(() => {
    for (const entry of navigation) {
      if (isNavGroup(entry) && page.url.pathname.startsWith(entry.prefix)) {
        expanded[entry.prefix] = true
      }
    }
  })

  function toggleGroup(prefix: string) {
    expanded[prefix] = !expanded[prefix]
  }

  function isActive(href: string): boolean {
    if (href === '/' || href === '/resumes') return page.url.pathname === href
    return page.url.pathname.startsWith(href)
  }

  function isGroupActive(group: { prefix: string }): boolean {
    return page.url.pathname.startsWith(group.prefix)
  }
</script>

<div class="app">
  <nav class="sidebar">
    <div class="logo">
      <h2>Forge</h2>
    </div>
    <ul class="nav-list">
      {#each navigation as entry}
        {#if isNavGroup(entry)}
          <li class="nav-group">
            <button
              class="group-label"
              class:group-active={isGroupActive(entry)}
              onclick={() => toggleGroup(entry.prefix)}
            >
              <span>{entry.label}</span>
              <span class="chevron" class:open={expanded[entry.prefix]}>
                &#9656;
              </span>
            </button>
            {#if expanded[entry.prefix]}
              <ul class="group-children">
                {#each entry.children as child}
                  <li>
                    <a
                      href={child.href}
                      class:active={isActive(child.href)}
                    >
                      {child.label}
                    </a>
                  </li>
                {/each}
              </ul>
            {/if}
          </li>
        {:else}
          <li>
            <a
              href={entry.href}
              class="top-link"
              class:active={isActive(entry.href)}
            >
              {entry.label}
            </a>
          </li>
        {/if}
      {/each}
    </ul>
    <div class="profile-button-area">
      <button class="profile-button" onclick={toggleProfileMenu} bind:this={profileButtonEl}>
        <span class="profile-avatar">{initials}</span>
        <span class="profile-name">{profileName}</span>
        <span class="profile-gear">&#9881;</span>
      </button>
    </div>
  </nav>
  <main class="content">
    <StorageBanner />
    {@render children()}
  </main>
</div>

<!-- Profile menu: mounted outside .app so position:fixed escapes all stacking contexts -->
<ProfileMenu
  profile={profileData}
  isOpen={menuOpen}
  onclose={() => menuOpen = false}
  buttonEl={profileButtonEl}
/>

<ToastContainer />

<!-- JD overlay: singleton mount. Host handles its own open/close state via
     the jdOverlayState store. Any consumer opens it via openJDOverlay(id). -->
<JDOverlayHost />

<!-- Chain view modal: mounted outside .app so it renders on top of everything.
     Conditional mount means zero overhead (no Sigma/WebGL) when closed. -->
{#if chainViewState.isOpen}
  <ChainViewModal
    highlightNode={chainViewState.highlightNode}
    isModal={true}
    onClose={closeChainView}
  />
{/if}

<style>
  .app {
    display: flex;
    min-height: 100vh;
  }

  .sidebar {
    width: var(--sidebar-width);
    background: var(--color-sidebar-bg);
    color: var(--color-sidebar-text);
    padding: 1.5rem 0 0;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    position: sticky;
    top: 0;
    height: 100vh;
    overflow-y: auto;
  }

  .logo {
    padding: 0 1.5rem 1.5rem;
    border-bottom: 1px solid var(--color-sidebar-border);
    margin-bottom: 1rem;
  }

  .logo h2 {
    font-size: var(--text-xl);
    font-weight: var(--font-semibold);
    letter-spacing: 0.05em;
    color: var(--color-sidebar-text-active);
  }

  .nav-list {
    list-style: none;
    flex: 1;
    overflow-y: auto;
  }

  /* Top-level standalone links (Dashboard, Resumes) */
  .top-link {
    display: block;
    padding: 0.625rem 1.5rem;
    color: var(--color-sidebar-text);
    text-decoration: none;
    font-size: var(--text-base);
    transition: background 0.15s, color 0.15s;
  }

  .top-link:hover {
    background: var(--color-sidebar-hover-bg);
    color: var(--color-sidebar-text-hover);
  }

  .top-link.active {
    background: var(--color-sidebar-active-bg);
    color: var(--color-sidebar-text-active);
    border-left: 3px solid var(--color-sidebar-accent);
    padding-left: calc(1.5rem - 3px);
  }

  /* Group label (accordion toggle) */
  .nav-group {
    border-top: 1px solid var(--color-sidebar-border);
  }

  .nav-group:first-child {
    border-top: none;
  }

  .group-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.625rem 1.5rem;
    background: none;
    border: none;
    color: var(--color-sidebar-text);
    font-size: var(--text-base);
    font-weight: var(--font-medium);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    text-align: left;
  }

  .group-label:hover {
    background: var(--color-sidebar-hover-bg);
    color: var(--color-sidebar-text-hover);
  }

  .group-label.group-active {
    color: var(--color-sidebar-text-active);
    font-weight: var(--font-semibold);
  }

  .chevron {
    font-size: var(--text-xs);
    transition: transform 0.15s;
    display: inline-block;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  /* Child links within a group */
  .group-children {
    list-style: none;
  }

  .group-children li a {
    display: block;
    padding: 0.45rem 1.5rem 0.45rem 2.5rem;
    color: var(--color-sidebar-text);
    text-decoration: none;
    font-size: var(--text-sm);
    transition: background 0.15s, color 0.15s;
  }

  .group-children li a:hover {
    background: var(--color-sidebar-hover-bg);
    color: var(--color-sidebar-text-hover);
  }

  .group-children li a.active {
    background: var(--color-sidebar-active-bg);
    color: var(--color-sidebar-text-active);
    border-left: 3px solid var(--color-sidebar-accent);
    padding-left: calc(2.5rem - 3px);
  }

  .content {
    flex: 1;
    padding: var(--content-padding);
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
    background: var(--color-sidebar-hover-bg);
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
    color: var(--color-sidebar-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .profile-gear {
    font-size: var(--text-base);
    color: var(--color-sidebar-text);
  }
</style>
