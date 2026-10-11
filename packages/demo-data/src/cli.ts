#!/usr/bin/env bun
/**
 * demo-data CLI.
 *
 *   bun run src/cli.ts generate [persona|all] [--seed S] [--as-of ISO] [--out DIR] [--server-bin PATH] [--keep-temp]
 *   bun run src/cli.ts verify   [persona|all] [--out DIR]
 *   bun run src/cli.ts list
 *
 * R2 (credentials from R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY; see src/r2/):
 *   bun run src/cli.ts push      [persona|all] [--target preview] [--out DIR] [--allow-stale] [--force]
 *   bun run src/cli.ts publish   [persona|all] --yes [--out DIR] [--allow-stale] [--force]   (prod bucket)
 *   bun run src/cli.ts pull      <uuid|slug> --target preview|prod [--out FILE] [--force]
 *   bun run src/cli.ts head      <uuid|slug> --target preview|prod
 *   bun run src/cli.ts ls        --target preview|prod [--prefix P]
 *   bun run src/cli.ts roundtrip                     (preview bucket only)
 *
 * From the repo root: `just demo-data generate`, `just demo-data verify`, and the R2 recipes
 * (`push-preview`, `publish`, `pull`, `head`, `ls`, `roundtrip`), which load credentials
 * with `op run`.
 */

import { join, resolve } from 'path'
import { generatePersona } from './generate'
import { personaUuid } from './ids'
import { PERSONAS, selectPersonas } from './personas'
import {
  PullVerificationError,
  PushRefusedError,
  UsageError,
  formatHead,
  headDataset,
  importInstructions,
  listDatasets,
  pullDataset,
  pushDatasets,
  resolveDatasetId,
} from './r2/commands'
import { R2ConfigError, assertPreviewBucket, bucketFor, describeConfig, loadR2Config, missingCredentials, parseTarget } from './r2/config'
import { R2Error, R2Objects } from './r2/objects'
import { REPO_ROOT, resolveServerBin } from './server'
import { DEFAULT_AS_OF } from './time'
import { verifyDataset } from './verify'

export const DEFAULT_SEED = 'forge-demo-v1'
export const DEFAULT_OUT = resolve(REPO_ROOT, 'data/demo')

/** Flags that never take a value, so `--force early-career-developer` keeps the persona. */
const BOOLEAN_FLAGS = new Set(['keep-temp', 'yes', 'force', 'allow-stale'])

const ROUNDTRIP_TEST = join(import.meta.dir, 'r2/__tests__/roundtrip.test.ts')

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
      else if (next !== undefined && !next.startsWith('--') && !BOOLEAN_FLAGS.has(name)) {
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
    case 'push':
    case 'publish':
    case 'pull':
    case 'head':
    case 'ls':
    case 'roundtrip':
      return r2Command(args, outDir)
    default:
      console.error(
        'usage: cli.ts <generate|verify|list|push|publish|pull|head|ls|roundtrip> [persona|all|uuid] [--target preview|prod] [--out DIR|FILE] (see the header of src/cli.ts)',
      )
      return 2
  }
}

/** `selectPersonas`, with an unknown slug reported as a usage error. */
function personasArg(arg: string | undefined) {
  try {
    return selectPersonas(arg)
  } catch (e) {
    throw new UsageError((e as Error).message)
  }
}

async function r2Command(args: Args, outDir: string): Promise<number> {
  try {
    switch (args.command) {
      case 'push':
      case 'publish': {
        const publish = args.command === 'publish'
        const target = publish ? 'prod' : parseTarget(flag(args, 'target') ?? 'preview')
        if (!publish && target === 'prod') {
          console.error('push only targets the preview bucket; use `publish --yes` (just demo-data publish) for production')
          return 2
        }
        if (publish && args.flags.yes !== true) {
          console.error(`publish uploads to the PRODUCTION bucket (${bucketFor('prod')}); re-run with --yes to confirm`)
          return 2
        }
        const store = new R2Objects(loadR2Config(target))
        console.log(`${publish ? 'publish' : 'push'} → ${describeConfig(store.config)}`)
        const outcomes = await pushDatasets(store, {
          personas: personasArg(flag(args, 'persona') ?? args.positional[0]),
          outDir,
          allowStale: args.flags['allow-stale'] === true,
          force: args.flags.force === true,
          log: (m) => console.log(m),
        })
        const uploaded = outcomes.filter((o) => o.action === 'uploaded').length
        console.log(`${uploaded} uploaded, ${outcomes.length - uploaded} already up to date`)
        return 0
      }
      case 'pull': {
        const target = parseTarget(flag(args, 'target'))
        const { uuid, slug } = resolveDatasetId(args.positional[0])
        const outPath = resolve(flag(args, 'out') ?? join(outDir, 'pulled', target, `${slug ?? uuid}.sqlite`))
        const store = new R2Objects(loadR2Config(target))
        const result = await pullDataset(store, { id: uuid, outPath, force: args.flags.force === true })
        console.log(`↓ ${result.key} from ${describeConfig(store.config)}`)
        console.log(`  ${result.path} (${result.bytes} bytes, sha256 ${result.sha256.slice(0, 12)}…, verified)`)
        console.log(`  persona ${result.meta['forge-persona'] ?? '-'}, as of ${result.meta['forge-as-of'] ?? '-'}, schema ${result.meta['forge-schema-head'] ?? '-'}`)
        console.log('')
        console.log(importInstructions(result.path))
        return 0
      }
      case 'head': {
        const target = parseTarget(flag(args, 'target'))
        const store = new R2Objects(loadR2Config(target))
        const { key, slug, head } = await headDataset(store, args.positional[0] as string)
        if (!head) {
          console.error(`no object at ${key} in ${describeConfig(store.config)}`)
          return 1
        }
        console.log(formatHead(key, slug, head))
        return 0
      }
      case 'ls': {
        const target = parseTarget(flag(args, 'target'))
        const store = new R2Objects(loadR2Config(target))
        const entries = await listDatasets(store, flag(args, 'prefix') ?? 'user/')
        for (const e of entries) console.log(`${e.key}\t${e.size}\t${e.lastModified ?? '-'}\t${e.slug ?? '-'}`)
        console.log(`${entries.length} object(s) in ${describeConfig(store.config)}`)
        return 0
      }
      case 'roundtrip': {
        // The round trip lives in a gated test; refuse here rather than let it skip silently.
        const missing = missingCredentials()
        if (missing.length > 0) {
          console.error(`missing ${missing.join(', ')}: run it as \`just demo-data roundtrip\` (loads them with op run)`)
          return 2
        }
        assertPreviewBucket(bucketFor('preview'))
        const proc = Bun.spawn([process.execPath, 'test', ROUNDTRIP_TEST], {
          cwd: resolve(import.meta.dir, '..'),
          env: { ...process.env, FORGE_R2_ROUNDTRIP: '1' },
          stdio: ['inherit', 'inherit', 'inherit'],
        })
        return await proc.exited
      }
    }
    return 2
  } catch (e) {
    if (e instanceof R2ConfigError || e instanceof UsageError) {
      console.error(e.message)
      return 2
    }
    if (e instanceof PushRefusedError || e instanceof PullVerificationError || e instanceof R2Error) {
      console.error(e.message)
      return 1
    }
    throw e
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
