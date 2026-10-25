import { DEFAULT_DATA_URL, DEFAULT_ID_URL, resolveOrigin } from './config-defaults.js'

/**
 * Where the account and snapshot services live. Set `VITE_ID_URL` / `VITE_DATA_URL` at build
 * time to point a build at other hosts (a local Worker, say); the defaults are the production
 * hosts. Each value is an origin. `svelte.config.js` allows the same two origins in the CSP.
 */
export const idUrl: string = resolveOrigin(import.meta.env.VITE_ID_URL, DEFAULT_ID_URL)
export const dataUrl: string = resolveOrigin(import.meta.env.VITE_DATA_URL, DEFAULT_DATA_URL)

/**
 * Hosts that can carry the `.resu.mx` session cookie to the account service. A `*.pages.dev`
 * preview or any other origin cannot, so signing in there could never work. `localhost` is for
 * local development against a local Worker.
 */
const SESSION_HOSTS: readonly string[] = ['app.resu.mx', 'localhost', '127.0.0.1']

function currentHostname(): string | undefined {
  return typeof location === 'undefined' ? undefined : location.hostname
}

/** Whether a page served from `hostname` can hold a session with the account service. */
export function hostCanCarrySession(hostname: string | undefined): boolean {
  return hostname !== undefined && SESSION_HOSTS.includes(hostname.toLowerCase())
}

/**
 * Whether the sign-in UI and the `/api/me` check exist at all. When false the app is local-only
 * and makes no request to the account service.
 */
export function signInAvailable(hostname: string | undefined = currentHostname()): boolean {
  return hostCanCarrySession(hostname)
}

/**
 * Whether this build ships cloud sync switched on (`VITE_CLOUD_SYNC=on`). It is off by default, so a
 * build can carry the feature before the snapshot service is deployed: nothing is shown or requested
 * until the variable says `on`, and turning it back to anything else removes the feature again.
 */
export const cloudSyncOn: boolean = import.meta.env.VITE_CLOUD_SYNC === 'on'

/** Whether cloud sync can run here: it needs the same cookie, so the same hosts. */
export function syncAvailable(hostname: string | undefined = currentHostname()): boolean {
  return hostCanCarrySession(hostname)
}
