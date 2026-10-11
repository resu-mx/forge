/**
 * Content conventions for generated data. The datasets are public, so nothing in them may
 * point at a real person, inbox, phone line, website or organization.
 *
 * - Emails end in `@example.com`.
 * - URL and domain hosts are `example.com`/`.org`/`.net`, a subdomain of one, or `*.test`.
 * - Phone numbers are `(NNN) 555-01NN`, the range reserved for fiction.
 * - Organization, issuer, agency and university names are invented; the denylist catches
 *   the obvious slips. Real cities and states are fine (the dashboard map needs them).
 *
 * `scanText` is applied to the corpus and to every TEXT column of a generated database.
 */

const EMAIL = /[A-Za-z0-9._%+-]+@([A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+)/g
const URL = /\b[a-z][a-z0-9+.-]*:\/\/([^/\s"'<>)\]]+)/gi
// Bare domains: lower-case TLDs only, so "ASP.NET" or "Node.js" are not mistaken for hosts.
const BARE_DOMAIN = /(?<![@\w.-])((?:[a-z0-9-]+\.)+(?:com|org|net|io|edu|gov|dev|ai|co|us|app|info|biz|mil))\b/g
const PHONE_LIKE = /(?:\+?1[\s.-]?)?(?:\(\d{3}\)|\b\d{3})[\s.-]?\d{3}[\s.-]\d{4}\b/g
const FICTIONAL_PHONE = /^\(\d{3}\) 555-01\d\d$/

/** Hosts allowed in URLs and bare domains. */
export function isAllowedHost(host: string): boolean {
  const h = host.toLowerCase().replace(/:\d+$/, '')
  return /^(?:[a-z0-9-]+\.)*example\.(?:com|org|net)$/.test(h) || /^(?:[a-z0-9-]+\.)+test$/.test(h)
}

/** Well-known real organizations that must never appear in generated content. */
export const DENYLIST: readonly string[] = [
  // Companies
  'Google', 'Alphabet', 'Microsoft', 'Amazon', 'Apple Inc', 'Meta Platforms', 'Facebook', 'Netflix',
  'IBM', 'Oracle', 'Intel', 'Nvidia', 'Salesforce', 'Adobe', 'Cisco', 'Uber', 'Airbnb', 'Stripe',
  'Tesla', 'SpaceX', 'OpenAI', 'Anthropic', 'Deloitte', 'Accenture', 'Booz Allen', 'Lockheed',
  'Raytheon', 'Northrop', 'Boeing', 'JPMorgan', 'Goldman Sachs', 'Walmart', 'Target Corporation',
  'CompTIA', 'ISC2', 'Coursera', 'Udemy',
  // Universities
  'Harvard', 'Stanford', 'Massachusetts Institute of Technology', 'MIT', 'Berkeley', 'Carnegie Mellon',
  'Caltech', 'Princeton', 'Yale', 'Columbia University', 'Cornell', 'Georgia Tech', 'Purdue', 'UCLA',
  'Ohio State', 'University of Washington', 'University of Michigan', 'University of Texas', 'Oxford',
  'Cambridge University', 'University of Cincinnati', 'University of Dayton', 'Arizona State',
  // Agencies and services
  'NASA', 'NSA', 'CIA', 'FBI', 'DARPA', 'Department of Defense', 'Department of Homeland Security',
  'US Army', 'U.S. Army', 'US Navy', 'U.S. Navy', 'Air Force', 'Marine Corps', 'Pentagon',
]

const DENY_PATTERNS = DENYLIST.map((name) => ({
  name,
  // Short all-caps acronyms match case-sensitively; names match case-insensitively.
  re: /^[A-Z0-9]{2,5}$/.test(name)
    ? new RegExp(`(?<![A-Za-z0-9])${escapeRe(name)}(?![A-Za-z0-9])`)
    : new RegExp(`(?<![A-Za-z0-9])${escapeRe(name)}(?![A-Za-z0-9])`, 'i'),
}))

function escapeRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export interface ScanOptions {
  /** Hosts allowed in this text on top of the example/test ones (app-seeded config). */
  extraHosts?: readonly string[]
}

/** Every convention violation in `text`, as human-readable strings. */
export function scanText(text: string, opts: ScanOptions = {}): string[] {
  const problems: string[] = []
  const hostOk = (host: string) =>
    isAllowedHost(host) || (opts.extraHosts ?? []).includes(host.toLowerCase().replace(/:\d+$/, ''))

  for (const m of text.matchAll(EMAIL)) {
    if (m[1]?.toLowerCase() !== 'example.com') problems.push(`email not @example.com: ${m[0]}`)
  }
  for (const m of text.matchAll(URL)) {
    const host = (m[1] ?? '').replace(/^[^@]*@/, '')
    if (!hostOk(host)) problems.push(`URL host not allowed: ${m[0]}`)
  }
  const withoutUrlsAndEmails = text.replace(URL, ' ').replace(EMAIL, ' ')
  for (const m of withoutUrlsAndEmails.matchAll(BARE_DOMAIN)) {
    if (!hostOk(m[1] ?? '')) problems.push(`domain not allowed: ${m[1]}`)
  }
  for (const m of text.matchAll(PHONE_LIKE)) {
    if (!FICTIONAL_PHONE.test(m[0].trim())) problems.push(`phone not (NNN) 555-01NN: ${m[0]}`)
  }
  for (const { name, re } of DENY_PATTERNS) {
    if (re.test(text)) problems.push(`denylisted name: ${name}`)
  }
  return problems
}

/** Walk any JSON-like value and scan every string in it. */
export function scanValue(value: unknown, path = '$', opts: ScanOptions = {}): string[] {
  if (typeof value === 'string') return scanText(value, opts).map((p) => `${path}: ${p}`)
  if (Array.isArray(value)) return value.flatMap((v, i) => scanValue(v, `${path}[${i}]`, opts))
  if (value && typeof value === 'object') {
    return Object.entries(value).flatMap(([k, v]) => scanValue(v, `${path}.${k}`, opts))
  }
  return []
}
