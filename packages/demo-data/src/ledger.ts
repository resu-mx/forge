/**
 * The ledger maps corpus keys to the ids the server assigned, and records the timestamps
 * each row should carry. The post-pass writes those timestamps (the server stamps rows with
 * the wall clock) and overlays the statuses no API can set.
 */

export interface LedgerEntry {
  /** Entity kind, e.g. `source`, `bullet`, `resume_entry`. */
  kind: string
  /** The corpus key (unique per kind). */
  key: string
  /** The table holding the row. */
  table: string
  id: string
  /** Intended values for timestamp columns, as `YYYY-MM-DDTHH:MM:SSZ`. */
  times: Record<string, string>
}

export class Ledger {
  private readonly byRef = new Map<string, LedgerEntry>()
  private readonly byId = new Map<string, LedgerEntry>()

  private static ref(kind: string, key: string): string {
    return `${kind}:${key}`
  }

  add(kind: string, key: string, table: string, id: string, times: Record<string, string> = {}): LedgerEntry {
    const ref = Ledger.ref(kind, key)
    if (this.byRef.has(ref)) throw new Error(`ledger: duplicate key ${ref}`)
    if (this.byId.has(id)) throw new Error(`ledger: id ${id} already recorded as ${this.byId.get(id)?.kind}:${this.byId.get(id)?.key}`)
    const entry: LedgerEntry = { kind, key, table, id, times: { ...times } }
    this.byRef.set(ref, entry)
    this.byId.set(id, entry)
    return entry
  }

  has(kind: string, key: string): boolean {
    return this.byRef.has(Ledger.ref(kind, key))
  }

  get(kind: string, key: string): LedgerEntry {
    const entry = this.byRef.get(Ledger.ref(kind, key))
    if (!entry) throw new Error(`ledger: unknown ${kind} "${key}"`)
    return entry
  }

  id(kind: string, key: string): string {
    return this.get(kind, key).id
  }

  /** The intended value of one timestamp column. */
  time(kind: string, key: string, column = 'created_at'): string {
    const t = this.get(kind, key).times[column]
    if (!t) throw new Error(`ledger: ${kind}:${key} has no ${column}`)
    return t
  }

  setTime(kind: string, key: string, column: string, iso: string): void {
    this.get(kind, key).times[column] = iso
  }

  byIdOrNull(id: string): LedgerEntry | null {
    return this.byId.get(id) ?? null
  }

  entries(): LedgerEntry[] {
    return [...this.byRef.values()]
  }
}
