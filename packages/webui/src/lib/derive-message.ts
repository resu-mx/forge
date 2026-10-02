import { forgeMode } from '$lib/sdk'

/**
 * Derivation is done by an agent, not by a server-side model call. In the browser-first build
 * that agent is Claude in Chrome, driving this tab.
 */
export const DERIVE_VIA_AGENT =
  forgeMode === 'wasm'
    ? 'Ask Claude in Chrome to derive this: it works on this tab (forge-in-chrome skill). What it writes appears here for you to approve.'
    : 'Derivation is done by an agent: use the MCP tools (forge_prepare_derivation), then approve the result here.'
