import { forgeMode } from '$lib/sdk'

/**
 * Whether Settings → Extension Config / Extension Logs can work in this build.
 *
 * In wasm mode the browser extension cannot reach the in-browser database: it runs on its own
 * origin, OPFS is partitioned by origin, and one Worker in one tab owns the database
 * (docs/src/dev/adrs/rust-wasm/0002-browser-runtime-ownership-and-transport.md). Until the bridge
 * (resu-mx/forge#107, #108, #109) and the extension endpoints (#110, #111, #113, #114) land, the
 * pages are hidden from the menu and explain themselves instead of calling /api/extension/*.
 *
 * Remove this gate in the change that lands the bridge, not before (resu-mx/forge#39).
 */
export const extensionPagesAvailable: boolean = forgeMode === 'api'

export const EXTENSION_UNAVAILABLE_TITLE = 'Not available in the browser app'
export const EXTENSION_UNAVAILABLE_MESSAGE =
  "The browser extension can't connect to the in-browser database yet. Run Forge against the API server to configure it."
