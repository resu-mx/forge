import type { ForgeError, Result, Skill } from '@forge/sdk'

/** One suggestion to accept. `key` is the extracted name (tracked in acceptedNames); `name` is what is sent. */
export interface AcceptItem {
  key: string
  name: string
}

export interface AcceptFailure extends AcceptItem {
  message: string
}

export interface AcceptOutcome {
  /** Keys of the suggestions that were added. */
  accepted: string[]
  failed: AcceptFailure[]
}

/**
 * Accept suggestions one at a time. Sequential on purpose: `POST …/skills {name}` creates the
 * skill on first use, and parallel accepts of near-duplicates race on that.
 */
export async function acceptSkills(
  items: AcceptItem[],
  add: (name: string) => Promise<Result<Skill>>,
  describe: (error: ForgeError) => string = (e) => e.message,
): Promise<AcceptOutcome> {
  const outcome: AcceptOutcome = { accepted: [], failed: [] }
  for (const item of items) {
    const res = await add(item.name)
    if (res.ok) outcome.accepted.push(item.key)
    else outcome.failed.push({ ...item, message: describe(res.error) })
  }
  return outcome
}

/** One message for every failure, or null when nothing failed. */
export function acceptFailureMessage(failed: AcceptFailure[]): string | null {
  if (failed.length === 0) return null
  if (failed.length === 1) return `Couldn't add "${failed[0].name}": ${failed[0].message}`
  return `Couldn't add ${failed.length} skills: ${failed.map((f) => f.name).join(', ')}`
}
