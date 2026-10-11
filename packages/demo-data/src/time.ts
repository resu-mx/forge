/**
 * Time handling. Every timestamp in a dataset is computed from `--as-of`, never from the
 * wall clock, so two runs with the same inputs produce the same content.
 */

import { rng } from './prng'

/** Default `--as-of`: a fixed instant, so the default dataset is reproducible. */
export const DEFAULT_AS_OF = '2026-09-30T17:00:00Z'

const ISO_Z = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/
const SQL_TS = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/

/** Parse an `--as-of` value. Only `YYYY-MM-DDTHH:MM:SSZ` is accepted. */
export function parseAsOf(value: string): Date {
  if (!ISO_Z.test(value)) throw new Error(`--as-of must look like 2026-09-30T17:00:00Z, got "${value}"`)
  const d = new Date(value)
  if (Number.isNaN(d.getTime())) throw new Error(`--as-of is not a valid instant: "${value}"`)
  return d
}

/** `YYYY-MM-DDTHH:MM:SSZ`, the format of the app's own `strftime(...)` defaults. */
export function isoZ(d: Date): string {
  return d.toISOString().replace(/\.\d{3}Z$/, 'Z')
}

/** `YYYY-MM-DD HH:MM:SS`, the format of SQLite's `datetime('now')`. */
export function sqlTimestamp(d: Date): string {
  return isoZ(d).replace('T', ' ').replace('Z', '')
}

/** Which of the two timestamp formats a stored value uses, or null if neither. */
export function timestampFormat(value: string): 'iso' | 'sql' | null {
  if (ISO_Z.test(value)) return 'iso'
  if (SQL_TS.test(value)) return 'sql'
  return null
}

/** Parse either timestamp format as UTC. */
export function parseTimestamp(value: string): Date {
  const fmt = timestampFormat(value)
  if (fmt === 'iso') return new Date(value)
  if (fmt === 'sql') return new Date(`${value.replace(' ', 'T')}Z`)
  throw new Error(`not a timestamp: "${value}"`)
}

/** Re-render an ISO instant in the format an existing stored value uses. */
export function inFormatOf(existing: string, iso: string): string {
  const fmt = timestampFormat(existing)
  if (fmt === null) throw new Error(`cannot tell the timestamp format of "${existing}"`)
  return fmt === 'iso' ? iso : sqlTimestamp(new Date(iso))
}

/** A calendar date `months` months before `asOf`, on `day` of that month (`YYYY-MM-DD`). */
export function monthsAgoDate(asOf: Date, months: number, day = 1): string {
  const d = new Date(Date.UTC(asOf.getUTCFullYear(), asOf.getUTCMonth() - months, day))
  return d.toISOString().slice(0, 10)
}

const HOUR = 3_600_000
const MINUTE = 60_000
const DAY = 24 * HOUR

/**
 * Turns `daysAgo` offsets into concrete instants.
 *
 * A day's events land between 13:00 and 21:00 UTC (a US working day), with jitter drawn
 * from a stream named after the entity, so the result does not depend on call order.
 * Nothing is ever later than `asOf`.
 */
export class Clock {
  readonly asOf: Date
  private readonly seed: string
  private readonly persona: string

  constructor(asOf: Date, seed: string, persona: string) {
    this.asOf = asOf
    this.seed = seed
    this.persona = persona
  }

  /** The latest instant any generated row may carry. */
  get latest(): number {
    return this.asOf.getTime() - MINUTE
  }

  /**
   * An instant `daysAgo` days before `asOf`. With `after`, the result is strictly later
   * than that instant (a child is never older than its parent).
   */
  at(daysAgo: number, stream: string, after?: string): string {
    if (!Number.isInteger(daysAgo) || daysAgo < 0) throw new Error(`daysAgo must be a non-negative integer, got ${daysAgo} (${stream})`)
    const r = rng(this.seed, this.persona, `time:${stream}`)
    const dayStart = Date.UTC(this.asOf.getUTCFullYear(), this.asOf.getUTCMonth(), this.asOf.getUTCDate()) - daysAgo * DAY
    let t = dayStart + 13 * HOUR + r.int(0, 8 * 60 - 1) * MINUTE + r.int(0, 59) * 1000
    if (after !== undefined) {
      const floor = new Date(after).getTime()
      if (t <= floor) t = floor + r.int(5, 90) * MINUTE + r.int(0, 59) * 1000
    }
    t = Math.min(t, this.latest)
    if (after !== undefined && t <= new Date(after).getTime()) {
      throw new Error(`${stream}: cannot place an event after ${after} and before as_of ${isoZ(this.asOf)}`)
    }
    return isoZ(new Date(t))
  }

  /** The dataset's epoch: when the persona's account "started" (`asOf - days`). */
  epoch(days: number): string {
    return this.at(days, 'epoch')
  }
}
