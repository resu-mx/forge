import type {
  AddResumeCertification,
  AddResumeEntry,
  CreateResume,
  ForgeError,
  GapAnalysis,
  JDLink,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  ResumeCertification,
  ResumeDocument,
  ResumeEntry,
  ResumeSectionEntity,
  ResumeSkill,
  ResumeTaglineRegenerationResult,
  ResumeTaglineState,
  ResumeTemplate,
  ResumeWithEntries,
  Result,
  Resume,
  UpdateResume,
  UpdateResumeEntry,
} from '../types'
import type { DebugStore } from '../debug'

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

type FetchFn = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>

export class ResumesResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
    private baseUrl: string = '',
    private debug?: DebugStore,
    private fetchImpl: FetchFn = (input, init) => globalThis.fetch(input, init),
  ) {}

  create(input: CreateResume): Promise<Result<Resume>> {
    return this.request<Resume>('POST', '/api/resumes', input)
  }

  list(pagination?: PaginationParams): Promise<PaginatedResult<Resume>> {
    return this.requestList<Resume>(
      'GET',
      '/api/resumes',
      toParams(pagination),
    )
  }

  get(id: string): Promise<Result<ResumeWithEntries>> {
    return this.request<ResumeWithEntries>('GET', `/api/resumes/${id}`)
  }

  update(id: string, input: UpdateResume): Promise<Result<Resume>> {
    return this.request<Resume>('PATCH', `/api/resumes/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/resumes/${id}`)
  }

  // -- Entry methods (replace perspective methods) --

  addEntry(
    resumeId: string,
    input: AddResumeEntry,
  ): Promise<Result<ResumeEntry>> {
    return this.request<ResumeEntry>(
      'POST',
      `/api/resumes/${resumeId}/entries`,
      input,
    )
  }

  listEntries(resumeId: string): Promise<Result<ResumeEntry[]>> {
    return this.request<ResumeEntry[]>(
      'GET',
      `/api/resumes/${resumeId}/entries`,
    )
  }

  updateEntry(
    resumeId: string,
    entryId: string,
    input: UpdateResumeEntry,
  ): Promise<Result<ResumeEntry>> {
    return this.request<ResumeEntry>(
      'PATCH',
      `/api/resumes/${resumeId}/entries/${entryId}`,
      input,
    )
  }

  removeEntry(resumeId: string, entryId: string): Promise<Result<void>> {
    return this.request<void>(
      'DELETE',
      `/api/resumes/${resumeId}/entries/${entryId}`,
    )
  }

  reorderEntries(
    resumeId: string,
    entries: Array<{ id: string; section_id: string; position: number }>,
  ): Promise<Result<void>> {
    return this.request<void>(
      'PATCH',
      `/api/resumes/${resumeId}/entries/reorder`,
      { entries },
    )
  }

  // -- Section methods --

  createSection(
    resumeId: string,
    input: { title: string; entry_type: string; position?: number },
  ): Promise<Result<ResumeSectionEntity>> {
    return this.request<ResumeSectionEntity>(
      'POST',
      `/api/resumes/${resumeId}/sections`,
      input,
    )
  }

  listSections(resumeId: string): Promise<Result<ResumeSectionEntity[]>> {
    return this.request<ResumeSectionEntity[]>(
      'GET',
      `/api/resumes/${resumeId}/sections`,
    )
  }

  updateSection(
    resumeId: string,
    sectionId: string,
    input: { title?: string; position?: number },
  ): Promise<Result<ResumeSectionEntity>> {
    return this.request<ResumeSectionEntity>(
      'PATCH',
      `/api/resumes/${resumeId}/sections/${sectionId}`,
      input,
    )
  }

  deleteSection(resumeId: string, sectionId: string): Promise<Result<void>> {
    return this.request<void>(
      'DELETE',
      `/api/resumes/${resumeId}/sections/${sectionId}`,
    )
  }

  // -- Skills methods --

  addSkill(
    resumeId: string,
    sectionId: string,
    skillId: string,
  ): Promise<Result<ResumeSkill>> {
    return this.request<ResumeSkill>(
      'POST',
      `/api/resumes/${resumeId}/sections/${sectionId}/skills`,
      { skill_id: skillId },
    )
  }

  listSkills(
    resumeId: string,
    sectionId: string,
  ): Promise<Result<ResumeSkill[]>> {
    return this.request<ResumeSkill[]>(
      'GET',
      `/api/resumes/${resumeId}/sections/${sectionId}/skills`,
    )
  }

  removeSkill(
    resumeId: string,
    sectionId: string,
    skillId: string,
  ): Promise<Result<void>> {
    return this.request<void>(
      'DELETE',
      `/api/resumes/${resumeId}/sections/${sectionId}/skills/${skillId}`,
    )
  }

  reorderSkills(
    resumeId: string,
    sectionId: string,
    skills: Array<{ skill_id: string; position: number }>,
  ): Promise<Result<void>> {
    return this.request<void>(
      'PATCH',
      `/api/resumes/${resumeId}/sections/${sectionId}/skills/reorder`,
      { skills },
    )
  }

  gaps(id: string): Promise<Result<GapAnalysis>> {
    return this.request<GapAnalysis>('GET', `/api/resumes/${id}/gaps`)
  }

  // -- IR & override methods --

  ir(id: string): Promise<Result<ResumeDocument>> {
    return this.request<ResumeDocument>('GET', `/api/resumes/${id}/ir`)
  }

  updateHeader(id: string, header: Record<string, unknown>): Promise<Result<Resume>> {
    return this.request<Resume>('PATCH', `/api/resumes/${id}/header`, header)
  }

  updateMarkdownOverride(id: string, content: string | null): Promise<Result<Resume>> {
    return this.request<Resume>('PATCH', `/api/resumes/${id}/markdown-override`, { content })
  }

  updateLatexOverride(id: string, content: string | null): Promise<Result<Resume>> {
    return this.request<Resume>('PATCH', `/api/resumes/${id}/latex-override`, { content })
  }

  // -- Tagline (Phase 92) --

  /**
   * Get the resume's current tagline state: generated (from linked JDs),
   * override (user-authored), resolved (override takes precedence), and
   * has_override flag for UI prompts.
   */
  getTagline(id: string): Promise<Result<ResumeTaglineState>> {
    return this.request<ResumeTaglineState>('GET', `/api/resumes/${id}/tagline`)
  }

  /** Force a tagline regeneration from currently linked JDs. */
  regenerateTagline(id: string): Promise<Result<ResumeTaglineRegenerationResult>> {
    return this.request<ResumeTaglineRegenerationResult>(
      'POST',
      `/api/resumes/${id}/tagline/regenerate`,
    )
  }

  /**
   * Set or clear the tagline override. Pass a non-empty string to set,
   * null (or empty) to clear and fall back to the generated tagline.
   */
  updateTaglineOverride(id: string, content: string | null): Promise<Result<ResumeTaglineState>> {
    return this.request<ResumeTaglineState>(
      'PATCH',
      `/api/resumes/${id}/tagline-override`,
      { content },
    )
  }

  async pdf(id: string, opts?: { bust?: boolean }): Promise<Result<Blob> & { cacheStatus?: 'hit' | 'miss' }> {
    const bustParam = opts?.bust ? '?bust=1' : ''
    const path = `/api/resumes/${id}/pdf${bustParam}`
    const method = 'POST'
    const start = performance.now()

    try {
      const response = await this.fetchImpl(`${this.baseUrl}${path}`, { method })
      const duration = Math.round(performance.now() - start)

      if (response.ok) {
        const blob = await response.blob()
        const cacheStatus = response.headers.get('X-Forge-Pdf-Cache') as 'hit' | 'miss' | null
        if (this.debug?.logToConsole) {
          console.debug(`[forge:sdk] ← ${method} ${path} ${response.status} ${duration}ms ok (${blob.size} bytes PDF, cache: ${cacheStatus})`)
        }
        return { ok: true, data: blob, cacheStatus: cacheStatus ?? undefined }
      }

      const json = await response.json() as { error?: ForgeError }
      const error = json.error ?? { code: 'UNKNOWN_ERROR', message: `HTTP ${response.status}` }
      if (this.debug?.logToConsole) {
        console.debug(`[forge:sdk] ← ${method} ${path} ${response.status} ${duration}ms ERROR ${error.code}`)
      }
      return { ok: false, error }
    } catch (err) {
      if (this.debug?.logToConsole) {
        console.debug(`[forge:sdk] ✗ ${method} ${path} NETWORK_ERROR`)
      }
      return { ok: false, error: { code: 'NETWORK_ERROR', message: String(err) } }
    }
  }

  saveAsTemplate(
    resumeId: string,
    input: { name: string; description?: string },
  ): Promise<Result<ResumeTemplate>> {
    return this.request<ResumeTemplate>(
      'POST',
      `/api/resumes/${resumeId}/save-as-template`,
      input,
    )
  }

  // ── Certifications (per-resume cert selection) ─────────────────────

  /** Add a certification to a resume. */
  addCertification(
    resumeId: string,
    input: AddResumeCertification,
  ): Promise<Result<ResumeCertification>> {
    return this.request<ResumeCertification>(
      'POST',
      `/api/resumes/${resumeId}/certifications`,
      input,
    )
  }

  /** Remove a certification from a resume. */
  removeCertification(resumeId: string, rcId: string): Promise<Result<void>> {
    return this.request<void>(
      'DELETE',
      `/api/resumes/${resumeId}/certifications/${rcId}`,
    )
  }

  /** List all certifications pinned to a resume. */
  listCertifications(resumeId: string): Promise<Result<ResumeCertification[]>> {
    return this.request<ResumeCertification[]>(
      'GET',
      `/api/resumes/${resumeId}/certifications`,
    )
  }

  // ── Linked JDs ─────────────────────────────────────────────────────

  /** List JDs linked to a resume. */
  listJobDescriptions(resumeId: string): Promise<Result<JDLink[]>> {
    return this.request<JDLink[]>(
      'GET',
      `/api/resumes/${resumeId}/job-descriptions`,
    )
  }
}
