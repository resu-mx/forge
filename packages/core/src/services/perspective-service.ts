/**
 * PerspectiveService — business logic for perspective reframings.
 *
 * Phase 1.3.3: uses EntityLifecycleManager instead of PerspectiveRepository.
 *
 * Same status transition rules as BulletService: draft → in_review →
 * approved / rejected; rejected → in_review (reopen); approved →
 * archived; archived → draft. Delete is blocked if the perspective is
 * referenced by a resume entry (ELM's RESTRICT rule on
 * resume_entries.perspective_id).
 *
 * bullet_content_snapshot is automatically populated at create time by
 * the `captureBulletSnapshotHook` wired into the entity map — so the
 * service (which does NOT call create) doesn't need to worry about it.
 * Embedding on create is also wired via `createEmbedHook(.., 'perspective')`.
 *
 * `getPerspectiveWithChain` replaces PerspectiveRepository.getWithChain's
 * 3-table JOIN with a 3-step fetch: perspective → bullet → primary
 * source. This costs 3 round trips instead of 1 but avoids adding a
 * named query for a path that runs on the single-row read side.
 */

import { storageErrorToForgeError } from '../storage/error-mapper'
import type { EntityLifecycleManager } from '../storage/lifecycle-manager'
import type {
  Perspective,
  PerspectiveWithChain,
  PerspectiveStatus,
  PerspectiveFilter,
  UpdatePerspectiveInput,
  Result,
  PaginatedResult,
} from '../types'
import type { WhereClause } from '../storage/adapter-types'

/** Valid status transitions for perspectives. */
const VALID_TRANSITIONS: Record<string, PerspectiveStatus[]> = {
  draft: ['in_review'],
  in_review: ['approved', 'rejected'],
  rejected: ['in_review'],
  approved: ['archived'],
  archived: ['draft'],
}

/** Raw perspectives-table row (before type-casting to Perspective). */
interface PerspectiveRow {
  id: string
  bullet_id: string
  content: string
  bullet_content_snapshot: string
  target_archetype: string | null
  domain: string | null
  framing: string
  status: string
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  created_at: string
}

export interface CreatePerspectiveParams {
  bullet_id: string
  content: string
  target_archetype?: string
  domain?: string
  framing?: 'accomplishment' | 'responsibility' | 'context'
  auto_approve?: boolean
}

export class PerspectiveService {
  constructor(protected readonly elm: EntityLifecycleManager) {}

  /**
   * Create a perspective directly from a bullet, bypassing the derivation
   * flow. Intended for filler bullets whose text is already resume-ready.
   *
   * When `auto_approve` is true (default), the perspective is created with
   * status 'approved' so it can be immediately added to a resume.
   * The `captureBulletSnapshotHook` auto-populates `bullet_content_snapshot`.
   */
  async createPerspective(input: CreatePerspectiveParams): Promise<Result<Perspective>> {
    if (!input.content || input.content.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Content must not be empty' } }
    }

    // Verify the bullet exists.
    const bulletResult = await this.elm.get('bullets', input.bullet_id)
    if (!bulletResult.ok) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Bullet ${input.bullet_id} not found` } }
    }

    const now = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z')
    const autoApprove = input.auto_approve !== false // default true

    const data: Record<string, unknown> = {
      bullet_id: input.bullet_id,
      content: input.content.trim(),
      target_archetype: input.target_archetype ?? null,
      domain: input.domain ?? null,
      framing: input.framing ?? 'accomplishment',
      status: autoApprove ? 'approved' : 'draft',
      ...(autoApprove ? { approved_at: now, approved_by: 'direct' } : {}),
    }

    const createResult = await this.elm.create('perspectives', data)
    if (!createResult.ok) {
      return { ok: false, error: storageErrorToForgeError(createResult.error) }
    }

    return this.getPerspective(createResult.value.id)
  }

  async getPerspective(id: string): Promise<Result<Perspective>> {
    const result = await this.elm.get('perspectives', id)
    if (!result.ok) {
      return { ok: false, error: storageErrorToForgeError(result.error) }
    }
    return { ok: true, data: this.toPerspective(result.value as unknown as PerspectiveRow) }
  }

  async getPerspectiveWithChain(id: string): Promise<Result<PerspectiveWithChain>> {
    const pResult = await this.elm.get('perspectives', id)
    if (!pResult.ok) {
      return { ok: false, error: storageErrorToForgeError(pResult.error) }
    }
    const perspective = this.toPerspective(pResult.value as unknown as PerspectiveRow)

    const bResult = await this.elm.get('bullets', perspective.bullet_id)
    if (!bResult.ok) {
      return { ok: false, error: storageErrorToForgeError(bResult.error) }
    }
    const bullet = bResult.value as Record<string, unknown>

    // Look up the primary source link for this bullet (is_primary = true).
    const linksResult = await this.elm.list('bullet_sources', {
      where: { bullet_id: perspective.bullet_id, is_primary: true },
      limit: 1,
    })
    if (!linksResult.ok || linksResult.value.rows.length === 0) {
      return {
        ok: false,
        error: {
          code: 'NOT_FOUND',
          message: `Perspective ${id} has no primary source in its chain`,
        },
      }
    }
    const primarySourceId = linksResult.value.rows[0].source_id as string

    const sResult = await this.elm.get('sources', primarySourceId)
    if (!sResult.ok) {
      return { ok: false, error: storageErrorToForgeError(sResult.error) }
    }
    const source = sResult.value as Record<string, unknown>

    return {
      ok: true,
      data: {
        ...perspective,
        bullet: {
          id: bullet.id as string,
          content: bullet.content as string,
          source_content_snapshot: bullet.source_content_snapshot as string,
          status: bullet.status as PerspectiveWithChain['bullet']['status'],
          created_at: bullet.created_at as string,
        } as PerspectiveWithChain['bullet'],
        source: {
          id: source.id as string,
          title: source.title as string,
          description: source.description as string,
          source_type: source.source_type as string,
          status: source.status as PerspectiveWithChain['source']['status'],
          created_at: source.created_at as string,
        } as PerspectiveWithChain['source'],
      },
    }
  }

  async listPerspectives(
    filter: PerspectiveFilter = {},
    offset = 0,
    limit = 50,
  ): Promise<PaginatedResult<Perspective>> {
    // source_id filter: walk bullet_sources first to collect the set
    // of bullet ids linked to the given source, then filter perspectives
    // by bullet_id ∈ that set.
    let bulletIdFilter: string[] | undefined
    if (filter.source_id !== undefined) {
      const linksResult = await this.elm.list('bullet_sources', {
        where: { source_id: filter.source_id },
        limit: 10000,
      })
      if (!linksResult.ok) {
        return { ok: false, error: storageErrorToForgeError(linksResult.error) }
      }
      const ids = Array.from(
        new Set(linksResult.value.rows.map((r) => r.bullet_id as string)),
      )
      bulletIdFilter = ids
      if (bulletIdFilter.length === 0) {
        return { ok: true, data: [], pagination: { total: 0, offset, limit } }
      }
    }

    const where: WhereClause = {}
    if (filter.target_archetype !== undefined)
      where.target_archetype = filter.target_archetype
    if (filter.domain !== undefined) where.domain = filter.domain
    if (filter.framing !== undefined) where.framing = filter.framing
    if (filter.status !== undefined) where.status = filter.status
    if (filter.bullet_id !== undefined && bulletIdFilter === undefined) {
      where.bullet_id = filter.bullet_id
    } else if (filter.bullet_id !== undefined && bulletIdFilter !== undefined) {
      // Intersect: must be in bulletIdFilter AND equal filter.bullet_id.
      // If filter.bullet_id isn't in the junction set, empty result.
      if (!bulletIdFilter.includes(filter.bullet_id)) {
        return { ok: true, data: [], pagination: { total: 0, offset, limit } }
      }
      where.bullet_id = filter.bullet_id
    } else if (bulletIdFilter !== undefined) {
      where.bullet_id = { $in: bulletIdFilter }
    }

    // When using in-memory search we need ALL matching rows so we can
    // filter, then slice for pagination ourselves.
    const needsInMemorySearch = !!filter.search
    const listResult = await this.elm.list('perspectives', {
      where: Object.keys(where).length > 0 ? where : undefined,
      orderBy: [{ field: 'created_at', direction: 'desc' }],
      offset: needsInMemorySearch ? 0 : offset,
      limit: needsInMemorySearch ? 10000 : limit,
    })
    if (!listResult.ok) {
      return { ok: false, error: storageErrorToForgeError(listResult.error) }
    }

    let rows = listResult.value.rows

    // In-memory text search on content (applied before pagination).
    if (filter.search) {
      const searchLower = filter.search.toLowerCase()
      rows = rows.filter(p => (p.content as string)?.toLowerCase().includes(searchLower))
    }

    const total = needsInMemorySearch ? rows.length : listResult.value.total
    if (needsInMemorySearch) {
      rows = rows.slice(offset, offset + limit)
    }

    return {
      ok: true,
      data: rows.map((r) => this.toPerspective(r as unknown as PerspectiveRow)),
      pagination: {
        total,
        offset,
        limit,
      },
    }
  }

  async updatePerspective(
    id: string,
    input: UpdatePerspectiveInput,
  ): Promise<Result<Perspective>> {
    if (input.content !== undefined && input.content.trim().length === 0) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Content must not be empty' },
      }
    }

    const patch: Record<string, unknown> = {}
    if (input.content !== undefined) patch.content = input.content
    if (input.target_archetype !== undefined)
      patch.target_archetype = input.target_archetype
    if (input.domain !== undefined) patch.domain = input.domain
    if (input.framing !== undefined) patch.framing = input.framing

    if (Object.keys(patch).length === 0) {
      return this.getPerspective(id)
    }

    const updateResult = await this.elm.update('perspectives', id, patch)
    if (!updateResult.ok) {
      return { ok: false, error: storageErrorToForgeError(updateResult.error) }
    }
    return this.getPerspective(id)
  }

  async deletePerspective(id: string): Promise<Result<void>> {
    const delResult = await this.elm.delete('perspectives', id)
    if (!delResult.ok) {
      const mapped = storageErrorToForgeError(delResult.error)
      // RESTRICT violations from resume_entries surface as CONFLICT
      // via the error mapper. Rewrite the message to preserve the
      // legacy wording asserted by tests / clients.
      if (mapped.code === 'CONFLICT') {
        return {
          ok: false,
          error: {
            code: 'CONFLICT',
            message: 'Cannot delete perspective that is in a resume',
          },
        }
      }
      return { ok: false, error: mapped }
    }
    return { ok: true, data: undefined }
  }

  async approvePerspective(id: string): Promise<Result<Perspective>> {
    return this.transition(id, 'approved')
  }

  async rejectPerspective(id: string, reason: string): Promise<Result<Perspective>> {
    if (!reason || reason.trim().length === 0) {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Rejection reason must not be empty' },
      }
    }
    return this.transition(id, 'rejected', { rejection_reason: reason })
  }

  async reopenPerspective(id: string): Promise<Result<Perspective>> {
    return this.transition(id, 'in_review')
  }

  /**
   * Submit a draft perspective for review (draft -> in_review).
   * Only draft perspectives can be submitted. Use reopenPerspective for rejected ones.
   */
  async submitPerspective(id: string): Promise<Result<Perspective>> {
    const existing = await this.elm.get('perspectives', id)
    if (!existing.ok) {
      return { ok: false, error: storageErrorToForgeError(existing.error) }
    }
    const row = existing.value as unknown as PerspectiveRow
    if (row.status !== 'draft') {
      return {
        ok: false,
        error: { code: 'VALIDATION_ERROR', message: 'Only draft perspectives can be submitted for review' },
      }
    }
    return this.transition(id, 'in_review')
  }

  /** Archive an approved perspective (approved -> archived). */
  async archivePerspective(id: string): Promise<Result<Perspective>> {
    return this.transition(id, 'archived')
  }

  /** Return an archived perspective to draft (archived -> draft). */
  async unarchivePerspective(id: string): Promise<Result<Perspective>> {
    return this.transition(id, 'draft')
  }

  // ── private helpers ─────────────────────────────────────────────────

  private async transition(
    id: string,
    target: PerspectiveStatus,
    opts?: { rejection_reason?: string },
  ): Promise<Result<Perspective>> {
    const existing = await this.elm.get('perspectives', id)
    if (!existing.ok) {
      return { ok: false, error: storageErrorToForgeError(existing.error) }
    }
    const row = existing.value as unknown as PerspectiveRow

    const allowed = VALID_TRANSITIONS[row.status] ?? []
    if (!allowed.includes(target)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Cannot transition from '${row.status}' to '${target}'`,
        },
      }
    }

    // Match the historical updateStatus field assignments:
    //   - approved:   status, approved_at (now), approved_by ('human'),
    //                 rejection_reason (null)
    //   - rejected:   status, rejection_reason (reason)
    //                 (approved_at / approved_by untouched)
    //   - other:      status, rejection_reason (null)
    //                 (approved_at / approved_by untouched)
    const patch: Record<string, unknown> = { status: target }
    if (target === 'approved') {
      patch.approved_at = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z')
      patch.approved_by = 'human'
      patch.rejection_reason = null
    } else if (target === 'rejected') {
      patch.rejection_reason = opts?.rejection_reason ?? null
    } else {
      patch.rejection_reason = null
    }

    const updateResult = await this.elm.update('perspectives', id, patch)
    if (!updateResult.ok) {
      return { ok: false, error: storageErrorToForgeError(updateResult.error) }
    }
    return this.getPerspective(id)
  }

  private toPerspective(row: PerspectiveRow): Perspective {
    return {
      id: row.id,
      bullet_id: row.bullet_id,
      content: row.content,
      bullet_content_snapshot: row.bullet_content_snapshot,
      target_archetype: row.target_archetype,
      domain: row.domain,
      framing: row.framing as Perspective['framing'],
      status: row.status as Perspective['status'],
      rejection_reason: row.rejection_reason,
      prompt_log_id: row.prompt_log_id,
      approved_at: row.approved_at,
      approved_by: row.approved_by,
      created_at: row.created_at,
    }
  }
}
