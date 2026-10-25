import { idUrl, signInAvailable } from '$lib/config'
import { createSession, initialSnapshot, type SessionSnapshot } from '$lib/session-core'
import { addToast } from '$lib/stores/toast.svelte'

// Who is signed in to the account service (id.resu.mx), if anyone. The logic lives in
// `$lib/session-core` so it can be tested without Svelte; this file only holds it in `$state`.
// Where the origin cannot carry the session cookie (previews, other hosts) the snapshot is
// `{ available: false, state: 'out' }` and nothing here makes a request.
const available = typeof window !== 'undefined' && signInAvailable()

let snapshot = $state<SessionSnapshot>(initialSnapshot(available))

const session = createSession({
  idUrl,
  available,
  // Looked up per call, so a test double installed on `window` is honoured.
  fetch: (input, init) => window.fetch(input, init),
  getHref: () => window.location.href,
  navigate: (url) => window.location.assign(url),
  // Keep SvelteKit's own history state; only the address changes.
  replaceUrl: (url) => window.history.replaceState(window.history.state, '', url),
  notify: addToast,
  onChange: (next) => {
    snapshot = next
  },
})

/** The current session: `state` is `unknown` until the first answer, then `in` or `out`. */
export function getSession(): SessionSnapshot {
  return snapshot
}

/** Ask the account service who is signed in (`GET /api/me`). Call once on load. */
export function refreshSession(): Promise<void> {
  return session.refresh()
}

/** Start the Auth0 sign-in: leaves the page for the identity provider and returns here. */
export function signIn(): Promise<void> {
  return session.signIn()
}

/** End the session, then re-check it. */
export function signOut(): Promise<void> {
  return session.signOut()
}
