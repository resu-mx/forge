/**
 * Component API Guards
 *
 * Regression guards for two call-shape bugs that svelte-check reports but
 * that slipped through because the webui check has pre-existing errors:
 *
 *  - <ConfirmDialog> renders only `{#if open}`, so a usage without `open`
 *    never appears (#174).
 *  - addToast() takes `{ message, type }`; a string first argument is wrong (#175).
 */
import { describe, test, expect } from 'bun:test'
import { readFileSync, readdirSync, statSync } from 'fs'
import { join, relative } from 'path'

const WEBUI_SRC = join(import.meta.dir, '..')

function collectFiles(dir: string, extensions: string[]): string[] {
  const results: string[] = []
  for (const entry of readdirSync(dir)) {
    if (entry === 'node_modules' || entry === '.svelte-kit' || entry === 'build') continue
    const full = join(dir, entry)
    if (statSync(full).isDirectory()) results.push(...collectFiles(full, extensions))
    else if (extensions.some(ext => entry.endsWith(ext))) results.push(full)
  }
  return results
}

/**
 * Return the text of each opening tag `<Name ...>` / `<Name ... />`,
 * skipping over `{...}` expressions (which may contain `>` or `/>`) and quoted strings.
 */
export function extractOpeningTags(source: string, name: string): string[] {
  const tags: string[] = []
  const needle = new RegExp(`<${name}(?=[\\s/>])`, 'g')
  let m: RegExpExecArray | null
  while ((m = needle.exec(source)) !== null) {
    let i = m.index + m[0].length
    let depth = 0
    let quote: string | null = null
    for (; i < source.length; i++) {
      const c = source[i]
      if (quote) {
        if (c === quote) quote = null
      } else if (depth > 0) {
        if (c === '{') depth++
        else if (c === '}') depth--
        else if (c === '`' || c === "'" || c === '"') quote = c
      } else if (c === '{') depth = 1
      else if (c === '"' || c === "'") quote = c
      else if (c === '>') break
    }
    tags.push(source.slice(m.index, i + 1))
  }
  return tags
}

/** Top-level attribute names of an opening tag (ignores text inside {} and quotes). */
export function attributeNames(tag: string): string[] {
  const names: string[] = []
  let depth = 0
  let quote: string | null = null
  let i = tag.indexOf(' ')
  if (i < 0) return names
  let token = ''
  const flush = () => {
    const n = token.match(/^[A-Za-z_:][\w:.-]*/)
    if (n) names.push(n[0])
    token = ''
  }
  for (; i < tag.length; i++) {
    const c = tag[i]
    if (quote) {
      if (c === quote) quote = null
      continue
    }
    if (depth > 0) {
      if (c === '{') depth++
      else if (c === '}') depth--
      else if (c === '`' || c === "'" || c === '"') quote = c
      continue
    }
    if (c === '{') depth = 1
    else if (c === '"' || c === "'") quote = c
    else if (c === '=') {
      flush()
      token = '\0' // skip value start
    } else if (/\s/.test(c)) {
      if (token !== '\0') flush()
      token = ''
    } else if (token !== '\0') token += c
    else if (c !== '\0') token = ''
  }
  return names
}

/** Find `addToast(` calls whose first argument does not start with `{`. */
export function stringStyleToastCalls(source: string): number[] {
  const lines: number[] = []
  const re = /\baddToast\(\s*([^\s])/g
  let m: RegExpExecArray | null
  while ((m = re.exec(source)) !== null) {
    if (m[1] !== '{') lines.push(source.slice(0, m.index).split('\n').length)
  }
  return lines
}

const svelteFiles = collectFiles(WEBUI_SRC, ['.svelte'])
const toastFiles = collectFiles(WEBUI_SRC, ['.svelte', '.ts']).filter(
  f => !f.endsWith('toast.svelte.ts') && !f.endsWith('component-api-guards.test.ts'),
)

describe('Component API guards', () => {
  test('every <ConfirmDialog> passes the required `open` prop', () => {
    const violations: string[] = []
    for (const file of svelteFiles) {
      const src = readFileSync(file, 'utf-8')
      for (const tag of extractOpeningTags(src, 'ConfirmDialog')) {
        if (!attributeNames(tag).includes('open')) {
          const line = src.slice(0, src.indexOf(tag)).split('\n').length
          violations.push(`${relative(WEBUI_SRC, file)}:${line}`)
        }
      }
    }
    expect(violations).toEqual([])
  })

  test('addToast() is never called with a string first argument', () => {
    const violations: string[] = []
    for (const file of toastFiles) {
      for (const line of stringStyleToastCalls(readFileSync(file, 'utf-8'))) {
        violations.push(`${relative(WEBUI_SRC, file)}:${line}`)
      }
    }
    expect(violations).toEqual([])
  })

  test('guard helpers detect the broken shapes', () => {
    const bad = `<ConfirmDialog title="x" oncancel={() => a = false} />`
    const good = `<ConfirmDialog open={a > 1} title="x" oncancel={() => a = false} />`
    expect(attributeNames(extractOpeningTags(bad, 'ConfirmDialog')[0])).not.toContain('open')
    expect(attributeNames(extractOpeningTags(good, 'ConfirmDialog')[0])).toContain('open')
    expect(stringStyleToastCalls(`addToast('hi', 'success')`)).toEqual([1])
    expect(stringStyleToastCalls('addToast(`hi`)')).toEqual([1])
    expect(stringStyleToastCalls(`addToast({ message: 'hi' })`)).toEqual([])
  })
})
