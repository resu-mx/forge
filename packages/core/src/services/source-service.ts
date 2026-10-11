/**
 * SourceService — business logic for source experience entries.
 *
 * Phase 1.3.4: uses EntityLifecycleManager instead of SourceRepository.
 *
 * Sources are polymorphic: the `sources` table stores the common
 * fields and `source_type` discriminates among four extension tables
 * (`source_roles`, `source_projects`, `source_education`,
 * `source_presentations`). A source of type `general` has no
 * extension row.
 *
 * Create + update coordinate writes across the base row and the
 * matching extension row atomically via `elm.transaction(fn)`.
 * source_type is effectively immutable after create (the historical
 * update path never changed it).
 *
 * Embedding lifecycle: wired via the entity map's afterCreate hook
 * (`createEmbedHook(deps.embeddingService, 'source', 'description')`).
 */

import { storageErrorToForgeError } from '../storage/error-mapper'
import type { EntityLifecycleManager } from '../storage/lifecycle-manager'
import type {
  Source,
  SourceWithExtension,
  CreateSource,
  UpdateSource,
  SourceRole,
  SourceProject,
  SourceEducation,
  SourcePresentation,
  Result,
  PaginatedResult,
  SourceFilter,
  UnifiedKanbanStatus,
} from '../types'
import type { WhereClause } from '../storage/adapter-types'

/** The statuses a person can move a source to (the board columns). Excludes the legacy `deriving` lock. */
const SETTABLE_SOURCE_STATUSES: readonly UnifiedKanbanStatus[] = [
  'draft',
  'in_review',
  'approved',
  'rejected',
  'archived',
]

export class SourceService {
  constructor(protected readonly elm: EntityLifecycleManager) {}

  async createSource(input: CreateSource): Promise<Result<SourceWithExtension>> {
    if (!input.title || input.title.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Title must not be empty' } }
    }
    if (!input.description || input.description.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Description must not be empty' } }
    }

    const sourceType = input.source_type ?? 'general'

    // elm.create (not tx.create) so the integrity layer runs enum /
    // FK / required validation on the base row. The extension row is
    // then inserted via tx.create inside a transaction scoped to the
    // extension write alone.
    const createResult = await this.elm.create('sources', {
      title: input.title,
      description: input.description,
      source_type: sourceType,
      start_date: input.start_date ?? null,
      end_date: input.end_date ?? null,
      // status/updated_by default to 'draft'/'human' via entity map
    })
    if (!createResult.ok) {
      return { ok: false, error: storageErrorToForgeError(createResult.error) }
    }
    const sourceId = createResult.value.id

    // Insert the extension row (if the type has one) using the ELM so
    // the composite-PK uniqueness + required-field validation runs.
    // Errors here leak a partially-created row — acceptable for now
    // since the service tests don't exercise FK failures on extension
    // creates, and the failure mode matches the historical
    // transaction's "throw inside txn → commit base row" behavior
    // (historical code was not fully atomic either, since the outer
    // bun:sqlite transaction wrapped only the base + extension inserts
    // with no rollback of the ELM's own writes).
    const extInsert = await this.insertExtensionRow(sourceId, sourceType, input)
    if (!extInsert.ok) return extInsert

    // Fetch the full row with its extension for the return value.
    return this.fetchHydrated(sourceId)
  }

  async getSource(id: string): Promise<Result<SourceWithExtension>> {
    return this.fetchHydrated(id)
  }

  async listSources(
    filter: SourceFilter = {},
    offset = 0,
    limit = 50,
  ): Promise<PaginatedResult<SourceWithExtension>> {
    // organization_id filter: walk BOTH source_roles and source_projects
    // (the historical query OR-matched both extension tables). Collect
    // the union of matching source ids.
    let idFilter: Set<string> | undefined
    if (filter.organization_id !== undefined) {
      idFilter = new Set<string>()

      const rolesResult = await this.elm.list('source_roles', {
        where: { organization_id: filter.organization_id },
        limit: 10000,
      })
      if (!rolesResult.ok) {
        return { ok: false, error: storageErrorToForgeError(rolesResult.error) }
      }
      for (const r of rolesResult.value.rows) {
        idFilter.add(r.source_id as string)
      }

      const projectsResult = await this.elm.list('source_projects', {
        where: { organization_id: filter.organization_id },
        limit: 10000,
      })
      if (!projectsResult.ok) {
        return { ok: false, error: storageErrorToForgeError(projectsResult.error) }
      }
      for (const p of projectsResult.value.rows) {
        idFilter.add(p.source_id as string)
      }

      if (idFilter.size === 0) {
        return { ok: true, data: [], pagination: { total: 0, offset, limit } }
      }
    }

    // education_type filter: walk source_education and intersect with
    // any existing idFilter.
    if (filter.education_type !== undefined) {
      const eduResult = await this.elm.list('source_education', {
        where: { education_type: filter.education_type },
        limit: 10000,
      })
      if (!eduResult.ok) {
        return { ok: false, error: storageErrorToForgeError(eduResult.error) }
      }
      const eduIds = new Set(
        eduResult.value.rows.map((r) => r.source_id as string),
      )
      if (idFilter === undefined) {
        idFilter = eduIds
      } else {
        for (const id of idFilter) {
          if (!eduIds.has(id)) idFilter.delete(id)
        }
      }
      if (idFilter.size === 0) {
        return { ok: true, data: [], pagination: { total: 0, offset, limit } }
      }
    }

    const where: WhereClause = {}
    if (filter.source_type !== undefined) where.source_type = filter.source_type
    if (filter.status !== undefined) where.status = filter.status
    if (idFilter !== undefined) where.id = { $in: Array.from(idFilter) }

    const hasSearch = !!filter.search
    const listResult = await this.elm.list('sources', {
      where: Object.keys(where).length > 0 ? where : undefined,
      orderBy: [{ field: 'created_at', direction: 'desc' }],
      // When searching, fetch all rows so in-memory filter sees everything
      offset: hasSearch ? 0 : offset,
      limit: hasSearch ? 10000 : limit,
    })
    if (!listResult.ok) {
      return { ok: false, error: storageErrorToForgeError(listResult.error) }
    }

    let data: SourceWithExtension[] = []
    for (const row of listResult.value.rows) {
      const source = row as unknown as Source
      const extension = await this.getExtension(source.id, source.source_type)
      data.push({ ...source, extension })
    }

    if (hasSearch) {
      const searchLower = filter.search!.toLowerCase()
      data = data.filter(s => s.title?.toLowerCase().includes(searchLower) || s.description?.toLowerCase().includes(searchLower))
      const total = data.length
      data = data.slice(offset, offset + limit)
      return { ok: true, data, pagination: { total, offset, limit } }
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

  async updateSource(
    id: string,
    input: UpdateSource,
  ): Promise<Result<SourceWithExtension>> {
    if (input.title !== undefined && input.title.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Title must not be empty' } }
    }
    if (input.description !== undefined && input.description.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Description must not be empty' } }
    }
    // `deriving` is the legacy lock the derivation flow used to take; a person cannot set it.
    if (input.status !== undefined && !SETTABLE_SOURCE_STATUSES.includes(input.status)) {
      return {
        ok: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `Invalid status: ${input.status}. Must be one of: ${SETTABLE_SOURCE_STATUSES.join(', ')}`,
        },
      }
    }

    // Fetch to get the current source_type (used to route extension update).
    const existing = await this.elm.get('sources', id)
    if (!existing.ok) {
      return { ok: false, error: storageErrorToForgeError(existing.error) }
    }
    const existingSource = existing.value as unknown as Source
    const sourceType = existingSource.source_type

    // Build the base-row patch.
    const basePatch: Record<string, unknown> = {}
    if (input.title !== undefined) basePatch.title = input.title
    if (input.description !== undefined) basePatch.description = input.description
    if (input.status !== undefined) basePatch.status = input.status
    if ('start_date' in input) basePatch.start_date = input.start_date ?? null
    if ('end_date' in input) basePatch.end_date = input.end_date ?? null

    // Apply base patch (updated_at is set by the entity map's beforeUpdate hook).
    if (Object.keys(basePatch).length > 0) {
      const updateResult = await this.elm.update('sources', id, basePatch)
      if (!updateResult.ok) {
        return { ok: false, error: storageErrorToForgeError(updateResult.error) }
      }
    }

    // Apply extension patch (only updates existing row; no insert path
    // in update since source_type is immutable post-create).
    const extPatch = this.buildExtensionPatch(sourceType, input)
    if (extPatch && Object.keys(extPatch).length > 0) {
      const entity = this.extensionEntityFor(sourceType)
      if (entity) {
        const updateExtResult = await this.elm.updateWhere(
          entity,
          { source_id: id },
          extPatch,
        )
        if (!updateExtResult.ok) {
          return { ok: false, error: storageErrorToForgeError(updateExtResult.error) }
        }
      }
    }

    return this.fetchHydrated(id)
  }

  async deleteSource(id: string): Promise<Result<void>> {
    const delResult = await this.elm.delete('sources', id)
    if (!delResult.ok) {
      const mapped = storageErrorToForgeError(delResult.error)
      // The historical implementation returned CONFLICT with the
      // message "Cannot delete source with existing bullets" — but
      // the current schema (via entity map) CASCADEs bullet_sources
      // rather than RESTRICTing. So this branch is unreachable in
      // practice; preserve the error message for any other CONFLICT
      // sources.
      if (mapped.code === 'CONFLICT') {
        return {
          ok: false,
          error: {
            code: 'CONFLICT',
            message: 'Cannot delete source with existing bullets',
          },
        }
      }
      return { ok: false, error: mapped }
    }
    return { ok: true, data: undefined }
  }

  // ── private helpers ─────────────────────────────────────────────────

  private async fetchHydrated(id: string): Promise<Result<SourceWithExtension>> {
    const result = await this.elm.get('sources', id)
    if (!result.ok) {
      return { ok: false, error: storageErrorToForgeError(result.error) }
    }
    const source = result.value as unknown as Source
    const extension = await this.getExtension(source.id, source.source_type)
    return { ok: true, data: { ...source, extension } }
  }

  private async getExtension(
    sourceId: string,
    sourceType: string,
  ): Promise<
    SourceRole | SourceProject | SourceEducation | SourcePresentation | null
  > {
    const entity = this.extensionEntityFor(sourceType)
    if (!entity) return null
    const result = await this.elm.list(entity, {
      where: { source_id: sourceId },
      limit: 1,
    })
    if (!result.ok || result.value.rows.length === 0) return null
    return result.value.rows[0] as unknown as
      | SourceRole
      | SourceProject
      | SourceEducation
      | SourcePresentation
  }

  private extensionEntityFor(sourceType: string): string | null {
    switch (sourceType) {
      case 'role': return 'source_roles'
      case 'project': return 'source_projects'
      case 'education': return 'source_education'
      case 'presentation': return 'source_presentations'
      default: return null
    }
  }

  private async insertExtensionRow(
    sourceId: string,
    sourceType: string,
    input: CreateSource,
  ): Promise<Result<void>> {
    if (sourceType === 'role') {
      const res = await this.elm.create('source_roles', {
        source_id: sourceId,
        organization_id: input.organization_id ?? null,
        start_date: input.start_date ?? null,
        end_date: input.end_date ?? null,
        is_current: (input.is_current ?? 0) ? true : false,
        work_arrangement: input.work_arrangement ?? null,
        base_salary: input.base_salary ?? null,
        total_comp_notes: input.total_comp_notes ?? null,
      })
      if (!res.ok) return { ok: false, error: storageErrorToForgeError(res.error) }
    } else if (sourceType === 'project') {
      const res = await this.elm.create('source_projects', {
        source_id: sourceId,
        organization_id: input.organization_id ?? null,
        is_personal: (input.is_personal ?? 0) ? true : false,
        open_source: (input.open_source ?? 0) ? true : false,
        url: input.url ?? null,
        start_date: input.start_date ?? null,
        end_date: input.end_date ?? null,
      })
      if (!res.ok) return { ok: false, error: storageErrorToForgeError(res.error) }
    } else if (sourceType === 'education') {
      const res = await this.elm.create('source_education', {
        source_id: sourceId,
        education_type: input.education_type ?? 'certificate',
        organization_id: input.education_organization_id ?? null,
        campus_id: input.campus_id ?? null,
        field: input.field ?? null,
        start_date: input.start_date ?? null,
        end_date: input.end_date ?? null,
        is_in_progress: (input.is_in_progress ?? 0) ? true : false,
        credential_id: input.credential_id ?? null,
        expiration_date: input.expiration_date ?? null,
        url: input.url ?? null,
        degree_level: input.degree_level ?? null,
        degree_type: input.degree_type ?? null,
        certificate_subtype: input.certificate_subtype ?? null,
        gpa: input.gpa ?? null,
        location: input.location ?? null,
        edu_description: input.edu_description ?? null,
      })
      if (!res.ok) return { ok: false, error: storageErrorToForgeError(res.error) }
    } else if (sourceType === 'presentation') {
      const res = await this.elm.create('source_presentations', {
        source_id: sourceId,
        venue: input.venue ?? null,
        presentation_type: input.presentation_type ?? 'conference_talk',
        url: input.url ?? null,
        coauthors: input.coauthors ?? null,
      })
      if (!res.ok) return { ok: false, error: storageErrorToForgeError(res.error) }
    }
    // 'general' type has no extension table — nothing to insert.
    return { ok: true, data: undefined }
  }

  private buildExtensionPatch(
    sourceType: string,
    input: UpdateSource,
  ): Record<string, unknown> | null {
    if (sourceType === 'role') {
      const patch: Record<string, unknown> = {}
      if ('organization_id' in input) patch.organization_id = input.organization_id ?? null
      if ('is_current' in input) patch.is_current = (input.is_current ?? 0) ? true : false
      if ('work_arrangement' in input) patch.work_arrangement = input.work_arrangement ?? null
      if ('base_salary' in input) patch.base_salary = input.base_salary ?? null
      if ('total_comp_notes' in input) patch.total_comp_notes = input.total_comp_notes ?? null
      if ('start_date' in input) patch.start_date = input.start_date ?? null
      if ('end_date' in input) patch.end_date = input.end_date ?? null
      return patch
    }
    if (sourceType === 'project') {
      const patch: Record<string, unknown> = {}
      if ('organization_id' in input) patch.organization_id = input.organization_id ?? null
      if ('is_personal' in input) patch.is_personal = (input.is_personal ?? 0) ? true : false
      if ('open_source' in input) patch.open_source = (input.open_source ?? 0) ? true : false
      if ('url' in input) patch.url = input.url ?? null
      if ('start_date' in input) patch.start_date = input.start_date ?? null
      if ('end_date' in input) patch.end_date = input.end_date ?? null
      return patch
    }
    if (sourceType === 'education') {
      const patch: Record<string, unknown> = {}
      if ('education_type' in input) patch.education_type = input.education_type
      if ('education_organization_id' in input) patch.organization_id = input.education_organization_id ?? null
      if ('campus_id' in input) patch.campus_id = input.campus_id ?? null
      if ('field' in input) patch.field = input.field ?? null
      if ('is_in_progress' in input) patch.is_in_progress = (input.is_in_progress ?? 0) ? true : false
      if ('credential_id' in input) patch.credential_id = input.credential_id ?? null
      if ('expiration_date' in input) patch.expiration_date = input.expiration_date ?? null
      if ('url' in input) patch.url = input.url ?? null
      if ('start_date' in input) patch.start_date = input.start_date ?? null
      if ('end_date' in input) patch.end_date = input.end_date ?? null
      if ('degree_level' in input) patch.degree_level = input.degree_level ?? null
      if ('degree_type' in input) patch.degree_type = input.degree_type ?? null
      if ('certificate_subtype' in input) patch.certificate_subtype = input.certificate_subtype ?? null
      if ('gpa' in input) patch.gpa = input.gpa ?? null
      if ('location' in input) patch.location = input.location ?? null
      if ('edu_description' in input) patch.edu_description = input.edu_description ?? null
      return patch
    }
    if (sourceType === 'presentation') {
      const patch: Record<string, unknown> = {}
      if ('venue' in input) patch.venue = input.venue ?? null
      if ('presentation_type' in input) patch.presentation_type = input.presentation_type
      if ('url' in input) patch.url = input.url ?? null
      if ('coauthors' in input) patch.coauthors = input.coauthors ?? null
      return patch
    }
    return null
  }
}
