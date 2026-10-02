import type { RuntimeStatus } from '@forge/runtime'
import { runtime } from '$lib/sdk'

// null in `api` mode: there is no in-browser runtime to report on.
let status = $state<RuntimeStatus | null>(runtime ? runtime.status() : null)

if (runtime) {
  runtime.onStatus((next) => {
    status = next
  })
}

/** Current state of the in-browser runtime, or null when the UI talks to an API server. */
export function getRuntimeStatus(): RuntimeStatus | null {
  return status
}
