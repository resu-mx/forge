import { FORGE_CHANGED_EVENT } from '@forge/runtime'

/**
 * Call `refetch` when stored data changed through the in-browser runtime, including changes made
 * by an agent driving the page. Bursts (an agent saving ten bullets) collapse into one call.
 * Returns the unsubscribe function, so it fits straight into an `$effect`.
 */
export function onForgeChanged(refetch: () => void, delayMs = 150): () => void {
  if (typeof window === 'undefined') return () => {}
  let timer: ReturnType<typeof setTimeout> | undefined
  const handler = () => {
    clearTimeout(timer)
    timer = setTimeout(refetch, delayMs)
  }
  window.addEventListener(FORGE_CHANGED_EVENT, handler)
  return () => {
    clearTimeout(timer)
    window.removeEventListener(FORGE_CHANGED_EVENT, handler)
  }
}
