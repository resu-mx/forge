/**
 * Whether this browser's database holds data worth keeping: the sync engine's `localHasData`. It
 * decides, when the cloud has nothing, between uploading what is here and just binding to the
 * account, and whether a vanished cloud copy is worth asking about.
 *
 * It counts only the tables the schema never seeds (sources, bullets, resumes, job descriptions):
 * the migrations do insert rows elsewhere (archetypes, domains, templates, and so on), and those
 * are on every fresh database, so counting them would make an empty browser look full.
 */

/** A count of one kind of entity; throws if it cannot be read. */
export type Count = () => Promise<number>

/**
 * True if any count is above zero. If a count could not be read and none of the others found
 * data, the answer is true: when in doubt, treat the browser as holding data, so the worst case is
 * an extra backup or an extra question and never a skipped one.
 */
export async function hasLocalData(counts: readonly Count[]): Promise<boolean> {
  let unreadable = false
  for (const count of counts) {
    try {
      if ((await count()) > 0) return true
    } catch {
      unreadable = true
    }
  }
  return unreadable
}

/** The slice of an SDK resource this uses: a list call that reports the total. */
interface Listable {
  list(params: { limit: number }): Promise<{ ok: true; pagination: { total: number } } | { ok: false }>
}

/** The SDK client's resources that hold user-created data. */
export interface CountableClient {
  sources: Listable
  bullets: Listable
  resumes: Listable
  jobDescriptions: Listable
}

/** One count per user-data resource of `client`, each asking for a single row and reading the total. */
export function entityCounts(client: CountableClient): Count[] {
  return [client.sources, client.bullets, client.resumes, client.jobDescriptions].map((resource) => async () => {
    const result = await resource.list({ limit: 1 })
    if (!result.ok) throw new Error('could not count')
    return result.pagination.total
  })
}
