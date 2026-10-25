// Plain JavaScript on purpose: `svelte.config.js` imports this file for the CSP `connect-src`
// directive, and the config loader cannot import TypeScript. `src/lib/config.ts` is the typed
// face the app code uses. Keep the two in step by keeping the defaults only here.

/** The account service (BetterAuth behind Auth0). The session cookie is scoped to `.resu.mx`. */
export const DEFAULT_ID_URL = 'https://id.resu.mx'

/** The snapshot service for a signed-in person's data. */
export const DEFAULT_DATA_URL = 'https://data.resu.mx'

/**
 * Normalise a configured service URL to its origin (no path, no trailing slash). An empty or
 * malformed value, or a scheme other than http(s), gives `fallback`: a typo in a build variable
 * then lands on the production host instead of breaking the app.
 *
 * @param {string | undefined} value
 * @param {string} fallback
 * @returns {string}
 */
export function resolveOrigin(value, fallback) {
  const raw = (value ?? '').trim()
  if (!raw) return fallback
  try {
    const url = new URL(raw)
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.origin : fallback
  } catch {
    return fallback
  }
}
