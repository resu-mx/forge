import type { ResumeTaglineState } from '@forge/sdk'

export type TaglineBadge = 'override' | 'auto' | null

export interface TaglineView {
  /** What the header shows: resolved, then the IR's header.tagline (legacy fallback), then ''. */
  display: string
  badge: TaglineBadge
  /** "Reset to Generated" is offered only while an override is in force. */
  canReset: boolean
  /** What the edit box opens with. */
  editSeed: string
}

/**
 * The header's tagline rules in one place (Phase 92). `state` is null until
 * GET /resumes/:id/tagline answers, or when it fails.
 */
export function taglineView(
  state: ResumeTaglineState | null,
  headerTagline: string | null | undefined,
): TaglineView {
  return {
    display: state?.resolved || headerTagline || '',
    badge: state?.has_override ? 'override' : state?.generated_tagline ? 'auto' : null,
    canReset: !!state?.has_override,
    editSeed: state?.tagline_override ?? state?.generated_tagline ?? '',
  }
}
