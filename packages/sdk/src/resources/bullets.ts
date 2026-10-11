import type {
  Bullet,
  BulletFilter,
  BulletWithRelations,
  PaginatedResult,
  PaginationParams,
  RejectInput,
  RequestFn,
  RequestListFn,
  Result,
  Skill,
  Source,
  UpdateBullet,
} from '../types'

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

export class BulletsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  /** Create a bullet manually (no AI derivation). Starts as 'draft' status. */
  create(input: {
    content: string
    source_content_snapshot?: string
    metrics?: string | null
    domain?: string | null
    technologies?: string[]
    source_ids?: Array<{ id: string; is_primary?: boolean }>
  }): Promise<Result<Bullet>> {
    return this.request<Bullet>('POST', '/api/bullets', input)
  }

  list(
    filter?: BulletFilter & PaginationParams,
  ): Promise<PaginatedResult<Bullet>> {
    return this.requestList<Bullet>('GET', '/api/bullets', toParams(filter))
  }

  get(id: string): Promise<Result<BulletWithRelations>> {
    return this.request<BulletWithRelations>('GET', `/api/bullets/${id}`)
  }

  update(id: string, input: UpdateBullet): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/bullets/${id}`)
  }

  approve(id: string): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/approve`)
  }

  reject(id: string, input: RejectInput): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/reject`, input)
  }

  reopen(id: string): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/reopen`)
  }

  /** Submit a draft bullet for review (draft -> in_review). */
  submit(id: string): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/submit`)
  }

  /** Archive an approved bullet (approved -> archived). */
  archive(id: string): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/archive`)
  }

  /** Return an archived bullet to draft (archived -> draft). */
  unarchive(id: string): Promise<Result<Bullet>> {
    return this.request<Bullet>('PATCH', `/api/bullets/${id}/unarchive`)
  }

  // ── Bullet Skills ───────────────────────────────────────────────────

  /** List all skills linked to this bullet. */
  listSkills(bulletId: string): Promise<Result<Skill[]>> {
    return this.request<Skill[]>('GET', `/api/bullets/${bulletId}/skills`)
  }

  /** Link an existing skill or create a new one and link it. */
  addSkill(bulletId: string, input: { skill_id: string } | { name: string; category?: string }): Promise<Result<Skill>> {
    return this.request<Skill>('POST', `/api/bullets/${bulletId}/skills`, input)
  }

  /** Unlink a skill from this bullet. */
  removeSkill(bulletId: string, skillId: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/bullets/${bulletId}/skills/${skillId}`)
  }

  // ── Bullet Sources ──────────────────────────────────────────────────

  /** List sources associated with this bullet (with is_primary flag). */
  listSources(bulletId: string): Promise<Result<Array<Source & { is_primary: number }>>> {
    return this.request<Array<Source & { is_primary: number }>>('GET', `/api/bullets/${bulletId}/sources`)
  }
}
