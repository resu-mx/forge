/**
 * BulletService — business logic for bullet points.
 *
 * Phase 1.3.2: uses EntityLifecycleManager instead of BulletRepository.
 *
 * Enforces status transition rules: in_review → approved,
 * in_review → rejected, rejected → in_review (reopen),
 * approved → archived, archived → draft.
 *
 * Multi-table writes:
 * - `bullet_sources` junction for source associations (composite PK)
 * - `bullet_skills` junction for technology tagging (composite PK)
 *
 * The `technologies` field on Bullet is a projection of linked skill
 * names (lowercased/trimmed), computed per read via hydrateTechnologies.
 * Creating or updating technologies does a find-or-create on the skills
 * table (case-insensitive) then inserts the junction row.
 *
 * Embedding lifecycle: wired via the entity map's afterCreate hook
 * (`createEmbedHook(deps.embeddingService, 'bullet')`) — the service
 * does NOT call embeddingService directly.
 */

import { storageErrorToForgeError } from '../storage/error-mapper'
import type { EntityLifecycleManager } from '../storage/lifecycle-manager'
import type {
  Result,
  PaginatedResult,
  Bullet,
  BulletStatus,
  BulletFilter,
  UpdateBulletInput,
} from '../types'
import type { WhereClause } from '../storage/adapter-types'

/** Valid status transitions for bullets. */
const VALID_TRANSITIONS: Record<string, BulletStatus[]> = {
  draft: ['in_review'],
  in_review: ['approved', 'rejected'],
  rejected: ['in_review'],
  approved: ['archived'],
  archived: ['draft'],
}

/** Raw bullets-table row without the derived `technologies` field. */
interface BulletRow {
  id: string
  content: string
  source_content_snapshot: string
  metrics: string | null
  domain: string | null
  status: string
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  created_at: string
}

export class BulletService {
  constructor(protected readonly elm: EntityLifecycleManager) {}

  /**
   * Create a bullet manually (without AI derivation). Defaults to 'draft'
   * status so the user can review before submitting.
   */
  async createBullet(input: {
    content: string
    source_content_snapshot?: string
    metrics?: string | null
    domain?: string | null
    technologies?: string[]
    source_ids?: Array<{ id: string; is_primary?: boolean }>
  }): Promise<Result<Bullet>> {
    if (!input.content || input.content.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Content must not be empty' } }
    }

    const content = input.content.trim()
    const createResult = await this.elm.create('bullets', {
      content,
      source_content_snapshot: input.source_content_snapshot ?? content,
      metrics: input.metrics ?? null,
      domain: input.domain ?? null,
      status: 'draft',
    })
    if (!createResult.ok) {
      return { ok: false, error: storageErrorToForgeError(createResult.error) }
    }
    const bulletId = createResult.value.id

    // Insert bullet_sources junction rows (default is_primary = true
    // unless explicitly false, matching historical semantics)
    if (input.source_ids) {
      for (const src of input.source_ids) {
        const linkResult = await this.elm.create('bullet_sources', {
          bullet_id: bulletId,
          source_id: src.id,
          is_primary: src.is_primary !== false,
        })
        if (!linkResult.ok) {
          return { ok: false, error: storageErrorToForgeError(linkResult.error) }
        }
      }
    }

    // Insert bullet_skills junction rows (find-or-create skills by name)
    if (input.technologies && input.technologies.length > 0) {
      const techResult = await this.insertTechnologies(bulletId, input.technologies)
      if (!techResult.ok) return techResult
    }

    return this.fetchHydrated(bulletId)
  }

  async getBullet(id: string): Promise<Result<Bullet>> {
    return this.fetchHydrated(id)
  }

  async listBullets(
    filter: BulletFilter = {},
    offset = 0,
    limit = 50,
  ): Promise<PaginatedResult<Bullet>> {
    // Junction-based filters: walk the junctions first to collect
    // matching bullet ids, then AND them with the main where clause.
    let idFilter: string[] | undefined

    if (filter.source_id) {
      const linksResult = await this.elm.list('bullet_sources', {
        where: { source_id: filter.source_id },
        limit: 10000,
      })
      if (!linksResult.ok) {
        return { ok: false, error: storageErrorToForgeError(linksResult.error) }
      }
      const ids = linksResult.value.rows.map((r) => r.bullet_id as string)
      idFilter = idFilter ? idFilter.filter((x) => ids.includes(x)) : ids
      if (idFilter.length === 0) {
        return {
          ok: true,
          data: [],
          pagination: { total: 0, offset, limit },
        }
      }
    }

    if (filter.technology) {
      // Look up skill id(s) by case-insensitive name match, then walk
      // bullet_skills for any bullets linked to those skills.
      const target = filter.technology.trim().toLowerCase()
      const skillsResult = await this.elm.list('skills', { limit: 10000 })
      if (!skillsResult.ok) {
        return { ok: false, error: storageErrorToForgeError(skillsResult.error) }
      }
      const matchingSkillIds = skillsResult.value.rows
        .filter(
          (s) =>
            typeof s.name === 'string' &&
            (s.name as string).toLowerCase() === target,
        )
        .map((s) => s.id as string)

      if (matchingSkillIds.length === 0) {
        return {
          ok: true,
          data: [],
          pagination: { total: 0, offset, limit },
        }
      }

      const linksResult = await this.elm.list('bullet_skills', {
        where: { skill_id: { $in: matchingSkillIds } },
        limit: 10000,
      })
      if (!linksResult.ok) {
        return { ok: false, error: storageErrorToForgeError(linksResult.error) }
      }
      const ids = Array.from(
        new Set(linksResult.value.rows.map((r) => r.bullet_id as string)),
      )
      idFilter = idFilter ? idFilter.filter((x) => ids.includes(x)) : ids
      if (idFilter.length === 0) {
        return {
          ok: true,
          data: [],
          pagination: { total: 0, offset, limit },
        }
      }
    }

    // Build the main where clause
    const where: WhereClause = {}
    if (filter.status) where.status = filter.status
    if (filter.domain) where.domain = filter.domain
    if (idFilter !== undefined) where.id = { $in: idFilter }

    const listResult = await this.elm.list('bullets', {
      where: Object.keys(where).length > 0 ? where : undefined,
      orderBy: [{ field: 'created_at', direction: 'desc' }],
      offset,
      limit,
    })
    if (!listResult.ok) {
      return { ok: false, error: storageErrorToForgeError(listResult.error) }
    }

    const data: Bullet[] = []
    for (const row of listResult.value.rows) {
      data.push(await this.hydrate(row as unknown as BulletRow))
    }

    return {
      ok: true,
      data,
      pagination: {
        total: listResult.value.total,
        offset,
        limit,
      },
    }
  }

  async updateBullet(id: string, input: UpdateBulletInput): Promise<Result<Bullet>> {
    if (input.content !== undefined && input.content.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Content must not be empty' } }
    }

    const patch: Record<string, unknown> = {}
    if (input.content !== undefined) patch.content = input.content
    if (input.metrics !== undefined) patch.metrics = input.metrics
    if (input.domain !== undefined) patch.domain = input.domain

    // Apply column update only if there's at least one column to set.
    // An update that only changes technologies must still verify the
    // bullet exists (preserved via a post-check below).
    if (Object.keys(patch).length > 0) {
      const updateResult = await this.elm.update('bullets', id, patch)
      if (!updateResult.ok) {
        return { ok: false, error: storageErrorToForgeError(updateResult.error) }
      }
    } else if (input.technologies !== undefined) {
      // No column patch, but we need to verify existence before touching junctions.
      const existsResult = await this.elm.get('bullets', id)
      if (!existsResult.ok) {
        return { ok: false, error: storageErrorToForgeError(existsResult.error) }
      }
    } else {
      // Nothing to update at all — just return the current row.
      return this.fetchHydrated(id)
    }

    // Replace the bullet's technology links if supplied.
    if (input.technologies !== undefined) {
      const replaceResult = await this.replaceTechnologies(id, input.technologies)
      if (!replaceResult.ok) return replaceResult
    }

    return this.fetchHydrated(id)
  }

  async deleteBullet(id: string): Promise<Result<void>> {
    const delResult = await this.elm.delete('bullets', id)
    if (!delResult.ok) {
      const mapped = storageErrorToForgeError(delResult.error)
      // Restrict violations from perspective references surface as
      // CONFLICT via the error mapper — but the old service returned
      // the specific message "Cannot delete bullet with existing
      // perspectives". Rewrite it here so legacy assertions pass.
      if (mapped.code === 'CONFLICT') {
        return {
          ok: false,
          error: {
            code: 'CONFLICT',
            message: 'Cannot delete bullet with existing perspectives',
          },
        }
      }
      return { ok: false, error: mapped }
    }
    return { ok: true, data: undefined }
  }

  async approveBullet(id: string): Promise<Result<Bullet>> {
    return this.transition(id, 'approved')
  }

  async rejectBullet(id: string, reason: string): Promise<Result<Bullet>> {
    if (!reason || reason.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Rejection reason must not be empty' } }
    }
    return this.transition(id, 'rejected', { rejection_reason: reason })
  }

  async reopenBullet(id: string): Promise<Result<Bullet>> {
    return this.transition(id, 'in_review')
  }

  /**
   * Submit a draft bullet for review (draft -> in_review).
   * Only draft bullets can be submitted. Use reopenBullet for rejected bullets.
   */
  async submitBullet(id: string): Promise<Result<Bullet>> {
    const existing = await this.elm.get('bullets', id)
    if (!existing.ok) {
      return { ok: false, error: storageErrorToForgeError(existing.error) }
    }
    const row = existing.value as unknown as BulletRow
    if (row.status !== 'draft') {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Only draft bullets can be submitted for review' } }
    }
    return this.transition(id, 'in_review')
  }

  /** Archive an approved bullet (approved -> archived). */
  async archiveBullet(id: string): Promise<Result<Bullet>> {
    return this.transition(id, 'archived')
  }

  /** Return an archived bullet to draft (archived -> draft). */
  async unarchiveBullet(id: string): Promise<Result<Bullet>> {
    return this.transition(id, 'draft')
  }

  // ── private helpers ─────────────────────────────────────────────────

  private async transition(
    id: string,
    target: BulletStatus,
    opts?: { rejection_reason?: string },
  ): Promise<Result<Bullet>> {
    const existing = await this.elm.get('bullets', id)
    if (!existing.ok) {
      return { ok: false, error: storageErrorToForgeError(existing.error) }
    }
    const row = existing.value as unknown as BulletRow

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

    const patch: Record<string, unknown> = { status: target }
    if (target === 'approved') {
      patch.approved_at = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z')
      patch.approved_by = 'human'
    } else if (target === 'rejected') {
      patch.rejection_reason = opts?.rejection_reason ?? null
    }

    const updateResult = await this.elm.update('bullets', id, patch)
    if (!updateResult.ok) {
      return { ok: false, error: storageErrorToForgeError(updateResult.error) }
    }

    return this.fetchHydrated(id)
  }

  private async fetchHydrated(id: string): Promise<Result<Bullet>> {
    const result = await this.elm.get('bullets', id)
    if (!result.ok) {
      return { ok: false, error: storageErrorToForgeError(result.error) }
    }
    const bullet = await this.hydrate(result.value as unknown as BulletRow)
    return { ok: true, data: bullet }
  }

  private async hydrate(row: BulletRow): Promise<Bullet> {
    const technologies = await this.getTechnologies(row.id)
    return {
      id: row.id,
      content: row.content,
      source_content_snapshot: row.source_content_snapshot,
      metrics: row.metrics,
      domain: row.domain,
      status: row.status as BulletStatus,
      rejection_reason: row.rejection_reason,
      prompt_log_id: row.prompt_log_id,
      approved_at: row.approved_at,
      approved_by: row.approved_by,
      created_at: row.created_at,
      technologies,
    }
  }

  /**
   * Fetch a bullet's technologies — linked skill names, lowercased/trimmed
   * and sorted for display stability. Matches the historical projection.
   */
  private async getTechnologies(bulletId: string): Promise<string[]> {
    const linksResult = await this.elm.list('bullet_skills', {
      where: { bullet_id: bulletId },
      limit: 1000,
    })
    if (!linksResult.ok) return []
    const techs: string[] = []
    for (const link of linksResult.value.rows) {
      const skillResult = await this.elm.get('skills', link.skill_id as string)
      if (skillResult.ok) {
        const name = (skillResult.value.name as string).toLowerCase().trim()
        if (name.length > 0) techs.push(name)
      }
    }
    return techs.sort()
  }

  /**
   * Replace a bullet's technology links — delete all, insert new.
   * Matches the historical "DELETE then INSERT" pattern in
   * BulletRepository.update.
   */
  private async replaceTechnologies(
    bulletId: string,
    technologies: string[],
  ): Promise<Result<void>> {
    const delResult = await this.elm.deleteWhere('bullet_skills', {
      bullet_id: bulletId,
    })
    if (!delResult.ok) {
      return { ok: false, error: storageErrorToForgeError(delResult.error) }
    }
    return this.insertTechnologies(bulletId, technologies)
  }

  /**
   * Link a list of technology strings to a bullet. Each name is
   * normalized (lowercase+trim), matched case-insensitively to an
   * existing skill, or created with category='other'. Empty strings
   * are skipped. Duplicate names within the list are skipped via the
   * junction's composite-PK uniqueness.
   */
  private async insertTechnologies(
    bulletId: string,
    technologies: string[],
  ): Promise<Result<void>> {
    // Bulk-load all skills once — for N technologies this is O(1)
    // scan instead of N queries. Skills table is bounded (<1000 rows).
    const skillsResult = await this.elm.list('skills', { limit: 10000 })
    if (!skillsResult.ok) {
      return { ok: false, error: storageErrorToForgeError(skillsResult.error) }
    }
    const byLowerName = new Map<string, string>()
    for (const s of skillsResult.value.rows) {
      if (typeof s.name === 'string' && typeof s.id === 'string') {
        byLowerName.set((s.name as string).toLowerCase(), s.id as string)
      }
    }

    const seenInThisCall = new Set<string>()
    for (const tech of technologies) {
      const normalized = tech.toLowerCase().trim()
      if (normalized.length === 0) continue
      if (seenInThisCall.has(normalized)) continue
      seenInThisCall.add(normalized)

      let skillId = byLowerName.get(normalized)
      if (!skillId) {
        const created = await this.elm.create('skills', {
          name: normalized,
          category: 'other',
        })
        if (!created.ok) {
          return { ok: false, error: storageErrorToForgeError(created.error) }
        }
        skillId = created.value.id
        byLowerName.set(normalized, skillId)
      }

      // Idempotent link: composite-PK uniqueness check in the ELM
      // would surface a CONFLICT if we re-inserted the same pair, so
      // pre-check first to preserve "silently skip" semantics.
      const existing = await this.elm.count('bullet_skills', {
        bullet_id: bulletId,
        skill_id: skillId,
      })
      if (existing.ok && existing.value > 0) continue

      const linkResult = await this.elm.create('bullet_skills', {
        bullet_id: bulletId,
        skill_id: skillId,
      })
      if (!linkResult.ok) {
        return { ok: false, error: storageErrorToForgeError(linkResult.error) }
      }
    }

    return { ok: true, data: undefined }
  }
}
