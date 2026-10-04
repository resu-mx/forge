import type { PaginatedResult, Resume } from '@forge/sdk'

/** Used when we know the summary is shared but can't say with which resume. */
export const UNNAMED_SHARER = 'another resume'

/**
 * Who to name in the shared-summary warning. The caller has already decided to warn
 * (linked_resume_count > 0); this only picks the name.
 */
export function sharedSummaryWarningName(result: PaginatedResult<Resume>): string {
  return result.ok && result.data.length > 0 ? result.data[0].name : UNNAMED_SHARER
}
