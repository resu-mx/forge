import type { PaginatedResult, PaginationParams } from '@forge/sdk'

/**
 * The most rows one list request returns. Both API runtimes clamp `limit` to this, so
 * `list({ limit: 5000 })` silently stops at 200 rows (pinned by the route contract tests).
 */
export const LIST_PAGE_LIMIT = 200

/**
 * Fetch every row of a paginated list endpoint. Use it wherever all rows are needed (chart
 * counts, aggregates) instead of asking for a large `limit`.
 *
 * The first page reports `pagination.total`; the remaining pages are fetched in parallel,
 * stepping by the number of rows the server actually returned, so a lower server cap still
 * reaches the total.
 *
 * @example listAll((p) => forge.bullets.list(p), { status: 'approved' })
 */
export async function listAll<T, F extends object = Record<never, never>>(
  list: (params: F & PaginationParams) => Promise<PaginatedResult<T>>,
  filter?: F,
): Promise<PaginatedResult<T>> {
  const page = (offset: number) =>
    list({ ...(filter as F), offset, limit: LIST_PAGE_LIMIT })

  const first = await page(0)
  if (!first.ok) return first

  const { total } = first.pagination
  const step = first.data.length
  const offsets: number[] = []
  if (step > 0) for (let offset = step; offset < total; offset += step) offsets.push(offset)

  const data = [...first.data]
  for (const rest of await Promise.all(offsets.map(page))) {
    if (!rest.ok) return rest
    data.push(...rest.data)
  }
  return { ok: true, data, pagination: { total, offset: 0, limit: data.length } }
}
