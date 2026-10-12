import type {
  CreateSource,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  Skill,
  Source,
  SourceFilter,
  SourceWithBullets,
  UpdateSource,
} from '../types'

/**
 * Serialize filter + pagination params into a Record<string, string> suitable
 * for `requestList`.  Drops undefined values.
 */
function toParams(
  filter?: object,
): Record<string, string> | undefined {
  if (!filter) return undefined
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(filter)) {
    if (v !== undefined && v !== null) out[k] = String(v)
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export class SourcesResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateSource): Promise<Result<Source>> {
    return this.request<Source>('POST', '/api/sources', input)
  }

  list(
    filter?: SourceFilter & PaginationParams,
  ): Promise<PaginatedResult<Source>> {
    return this.requestList<Source>('GET', '/api/sources', toParams(filter))
  }

  get(id: string): Promise<Result<SourceWithBullets>> {
    return this.request<SourceWithBullets>('GET', `/api/sources/${id}`)
  }

  update(id: string, input: UpdateSource): Promise<Result<Source>> {
    return this.request<Source>('PATCH', `/api/sources/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/sources/${id}`)
  }

  // ── Source skills ─────────────────────────────────────────────────

  listSkills(id: string): Promise<Result<Skill[]>> {
    return this.request<Skill[]>('GET', `/api/sources/${id}/skills`)
  }

  /** Link an existing skill (`{ skill_id }`) or find-or-create one by name (`{ name }`). */
  addSkill(id: string, input: { skill_id: string } | { name: string }): Promise<Result<Skill>> {
    return this.request<Skill>('POST', `/api/sources/${id}/skills`, input)
  }

  removeSkill(id: string, skillId: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/sources/${id}/skills/${skillId}`)
  }
}
