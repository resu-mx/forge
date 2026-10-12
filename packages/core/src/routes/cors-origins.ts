/**
 * `FORGE_CORS_ORIGINS` (resu-mx/forge#126): which origins may call the API from a browser.
 *
 * The Rust server reads the same variable with the same rules (`CorsOrigins::parse` in
 * `crates/forge-server/src/lib.rs`), and `cors.test.ts` holds both runtimes to them.
 */

/** Any origin (`'*'`), or exactly these origins. */
export type CorsOrigins = '*' | string[]

/**
 * Parse a `FORGE_CORS_ORIGINS` value. `null` means "not configured", for an unset or blank value,
 * so the built-in policy applies (which keeps the defaults for anyone who never sets it).
 *
 * A comma-separated list. Each entry is trimmed and loses trailing `/`s, since an `Origin` header
 * never has one, so `http://localhost:4173/` still works. Empty entries are dropped. A `*` entry
 * means any origin. Entries are otherwise compared exactly (case-sensitive) with the request's
 * `Origin`.
 */
export function parseCorsOrigins(raw: string | undefined): CorsOrigins | null {
  if (raw === undefined) return null
  const origins: string[] = []
  for (const part of raw.split(',')) {
    const entry = part.trim().replace(/\/+$/, '')
    if (entry === '') continue
    if (entry === '*') return '*'
    origins.push(entry)
  }
  return origins.length > 0 ? origins : null
}
