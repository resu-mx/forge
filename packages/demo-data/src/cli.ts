#!/usr/bin/env bun
/**
 * demo-data CLI.
 *
 *   bun run src/cli.ts generate [persona|all] [--seed S] [--as-of ISO] [--out DIR] [--server-bin PATH] [--keep-temp]
 *   bun run src/cli.ts verify   [persona|all] [--out DIR]
 *   bun run src/cli.ts list
 *
 * From the repo root: `just demo-data generate`, `just demo-data verify`.
 */

import { resolve } from 'path'
import { generatePersona } from './generate'
import { personaUuid } from './ids'
import { PERSONAS, selectPersonas } from './personas'
import { REPO_ROOT, resolveServerBin } from './server'
import { DEFAULT_AS_OF } from './time'
import { verifyDataset } from './verify'

export const DEFAULT_SEED = 'forge-demo-v1'
export const DEFAULT_OUT = resolve(REPO_ROOT, 'data/demo')

interface Args {
  command: string | undefined
  positional: string[]
  flags: Record<string, string | true>
}

function parseArgs(argv: string[]): Args {
  const [command, ...rest] = argv
  const positional: string[] = []
  const flags: Record<string, string | true> = {}
  for (let i = 0; i < rest.length; i++) {
    const a = rest[i] as string
    if (a.startsWith('--')) {
      const [name, inline] = a.slice(2).split('=', 2) as [string, string | undefined]
      const next = rest[i + 1]
      if (inline !== undefined) flags[name] = inline
      else if (next !== undefined && !next.startsWith('--') && name !== 'keep-temp') {
        flags[name] = next
        i++
      } else flags[name] = true
    } else positional.push(a)
  }
  return { command, positional, flags }
}

function flag(args: Args, name: string): string | undefined {
  const v = args.flags[name]
  return typeof v === 'string' ? v : undefined
}

async function main(): Promise<number> {
  const args = parseArgs(process.argv.slice(2))
  const outDir = resolve(flag(args, 'out') ?? DEFAULT_OUT)

  switch (args.command) {
    case 'generate': {
      const bin = flag(args, 'server-bin') ?? resolveServerBin()
      if (!bin) {
        console.error('forge-server not found: build it (cargo build -p forge-server) or set FORGE_SERVER_BIN')
        return 2
      }
      for (const corpus of selectPersonas(args.positional[0])) {
        await generatePersona({
          corpus,
          seed: flag(args, 'seed') ?? DEFAULT_SEED,
          asOf: flag(args, 'as-of') ?? DEFAULT_AS_OF,
          outDir,
          serverBin: bin,
          keepTemp: args.flags['keep-temp'] === true,
          log: (m) => console.log(m),
        })
      }
      return 0
    }
    case 'verify': {
      let failed = 0
      for (const corpus of selectPersonas(args.positional[0])) {
        const report = verifyDataset(outDir, corpus)
        if (report.problems.length === 0) {
          console.log(`✓ ${corpus.slug}: ${report.dbPath} (fingerprint ${report.manifest?.content_fingerprint.slice(0, 12)}…)`)
        } else {
          failed++
          console.error(`✗ ${corpus.slug}: ${report.problems.length} problem(s)`)
          for (const p of report.problems.slice(0, 50)) console.error(`    ${p}`)
        }
      }
      return failed === 0 ? 0 : 1
    }
    case 'list': {
      for (const slug of Object.keys(PERSONAS)) console.log(`${slug}\t${personaUuid(slug)}`)
      return 0
    }
    default:
      console.error('usage: cli.ts <generate|verify|list> [persona|all] [--seed S] [--as-of ISO] [--out DIR] [--server-bin PATH]')
      return 2
  }
}

if (import.meta.main) {
  main().then(
    (code) => process.exit(code),
    (err: unknown) => {
      console.error(err instanceof Error ? (err.stack ?? err.message) : err)
      process.exit(1)
    },
  )
}
