/**
 * The account and snapshot hosts the wasm e2e build is pointed at (`playwright.wasm.config.ts`
 * passes them to the web server as VITE_ID_URL / VITE_DATA_URL). They sit on the reserved `.test`
 * TLD, so a spec that does not stub them fails fast on DNS instead of reaching a real service,
 * and a spec that stubs them with `page.route` is the only thing that answers.
 */
export const E2E_ID_URL = 'https://id.e2e.test'
export const E2E_DATA_URL = 'https://data.e2e.test'
