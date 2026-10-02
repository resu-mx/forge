---
name: forge-in-chrome
description: Derive bullets and perspectives in Forge by driving the Forge tab in Chrome through `window.forge`. Use when asked to derive bullets from a source, write a perspective for a bullet, or otherwise read or write Forge data while the browser-first (wasm) UI is open.
---

# Forge in Chrome

In the browser-first build the database lives in the person's Chrome profile, inside the Forge
tab. There is no server to call. You reach the data through `window.forge`, the same
`ForgeClient` the UI uses, by running JavaScript in that tab.

You write the text; the person approves it. Nothing you derive is final until they approve it
in the UI.

## Before you start

- Needs Chrome with the Claude in Chrome extension allowed on the Forge origin. Forge's data is
  per browser: it lives in Chrome, not in another browser.
- Load the Chrome tools in one `ToolSearch` call (`tabs_context_mcp`, `javascript_tool`,
  `navigate`, `get_page_text`), call `tabs_context_mcp`, and find the tab whose URL is the Forge
  app. Open it with `tabs_create_mcp` only if none exists.
- The UI must be in wasm mode (`VITE_FORGE_MODE=wasm`). Check:
  `typeof window.forge?.derivations?.prepare` should be `'object'`/`'function'` — not `undefined`.
- Only one tab owns the database. If calls return `STORAGE_BUSY`, another Forge tab owns it:
  switch to that tab, or ask the person to close the other one.

## Calling the API

Every call returns `{ ok: true, data }` or `{ ok: false, error }`. Check `ok`. Always return a
small value from `javascript_tool` (ids and a few fields), never whole lists:

```js
const r = await window.forge.sources.list({ limit: 20 })
r.ok ? r.data.map(s => ({ id: s.id, title: s.title })) : r.error
```

Keep results small: pick the fields you need and cap list sizes.

## Source → bullets

1. Find the source (`forge.sources.list`, or `forge.sources.get(id)`).
2. Prepare:
   ```js
   const p = await window.forge.derivations.prepare({ entity_type: 'source', entity_id: SOURCE_ID, client_id: 'claude-in-chrome' })
   p.ok ? { id: p.data.derivation_id, prompt: p.data.prompt, expires: p.data.expires_at } : p.error
   ```
3. **You are the model.** Read `prompt` and write the bullets it asks for, following its
   instructions exactly. Do not call another model. Do not invent facts that are not in the source.
4. Commit within the expiry (**2 minutes** from `prepare`; write the text promptly). Each bullet is an object:
   ```js
   const c = await window.forge.derivations.commitBullets(DERIVATION_ID, {
     bullets: [{ content: '…', technologies: ['Rust'], metrics: null }],
   })
   c.ok ? c.data.map(b => ({ id: b.id, status: b.status })) : c.error
   ```
   `metrics` is a string when the source states a number, otherwise `null`. A validation error
   comes back in `error`; fix the text and commit again (the derivation stays open until it expires).

## Bullet → perspective

1. Pick a bullet that is **approved** (the API rejects any other), and an archetype and domain by name
   (`forge.archetypes.list()`, `forge.domains.list()`), plus a framing:
   `accomplishment`, `responsibility` or `context`.
2. Prepare with `entity_type: 'bullet'`, `entity_id`, and
   `params: { archetype, domain, framing }`.
3. Write the perspective from the returned `prompt`, then
   `forge.derivations.commitPerspective(id, { content, reasoning })`.

## What the person sees

- Derived bullets and perspectives land as **`in_review`**. They show up in the UI by
  themselves: every successful write fires a `forge:changed` event on `window` and the list pages
  refetch, so the person watches your results appear.
- **Approving and rejecting stay with the person.** Do not call `bullets.approve`,
  `perspectives.approve`, `reject` or the review endpoints on your own initiative, even if
  asked to "finish the job" — tell them what is waiting for review instead. Approve
  only if they say so for those specific items, in chat.

## Do not

- Do not edit or delete existing approved content to make a derivation fit.
- Do not read the whole database to look for context. Fetch the one source or bullet.
- Do not use `forge.debug` or `window.forgeRuntime` internals; the client is enough.
- Do not call `fetch('/api/...')` directly; use the client so errors have one shape.

## When it does not work

| You see | It means |
| --- | --- |
| `window.forge` is `undefined` | wrong tab, or the UI is not in wasm mode, or the page has not finished loading |
| `STORAGE_BUSY` | another tab owns the database |
| `STORAGE_UNAVAILABLE` | the runtime did not start; read the page for its message |
| `Derivation has expired` (410) | the 2 minute window passed; `prepare` again |
| `NOT_FOUND` on commit | wrong derivation id |
