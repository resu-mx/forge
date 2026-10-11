/**
 * Spawn and stop the Rust `forge-server` against a fresh database file.
 *
 * The spawn / free-port / health-poll / watchdog logic is copied from
 * `packages/core/src/routes/__tests__/helpers.ts` (`createRustTestApp`): packages may not
 * import core's test code (see `packages/AGENTS.md`). Differences: the health poll is async,
 * the watchdog reports the server's pid, and `stop()` sends the server SIGINT, which
 * `forge-server` handles as a graceful shutdown (it closes the database before exiting).
 */

import { existsSync } from 'fs'
import { resolve } from 'path'

/** Repository root (this file is `packages/demo-data/src/server.ts`). */
export const REPO_ROOT = resolve(import.meta.dir, '../../..')

/** `FORGE_SERVER_BIN`, else the debug build, else null. */
export function resolveServerBin(): string | null {
  const fromEnv = process.env.FORGE_SERVER_BIN
  if (fromEnv) return existsSync(fromEnv) ? fromEnv : null
  const debug = resolve(REPO_ROOT, 'target/debug/forge-server')
  return existsSync(debug) ? debug : null
}

// `process.on('exit')` never fires under `bun test`, and a crashed generator must not leave a
// server behind, so the server runs under a small shell watchdog that kills it as soon as this
// process is gone (or the watchdog itself is terminated). It prints the server's pid first.
// Args: $0 = server binary, $1 = this process's pid.
const WATCHDOG = [
  'trap \'kill "$srv" 2>/dev/null\' TERM',
  '"$0" & srv=$!',
  'echo "$srv"',
  'while kill -0 "$1" 2>/dev/null && kill -0 "$srv" 2>/dev/null; do sleep 0.2; done',
  'kill "$srv" 2>/dev/null',
].join('; ')

export interface ForgeServer {
  /** e.g. `http://127.0.0.1:53211` (no `/api`). */
  baseUrl: string
  port: number
  dbPath: string
  /** Graceful stop; resolves once the server process has exited. */
  stop(): Promise<void>
}

export interface StartOptions {
  bin: string
  dbPath: string
  logLevel?: string
  /** How long to wait for `/api/health`, in ms. */
  timeoutMs?: number
}

/** Ask the OS for a free port. */
function freePort(): number {
  const probe = Bun.listen({ hostname: '127.0.0.1', port: 0, socket: { data() {} } })
  const port = probe.port
  probe.stop(true)
  return port
}

async function firstLine(stream: ReadableStream<Uint8Array>): Promise<string> {
  const reader = stream.getReader()
  const decoder = new TextDecoder()
  let text = ''
  while (!text.includes('\n')) {
    const { value, done } = await reader.read()
    if (done) break
    text += decoder.decode(value, { stream: true })
  }
  reader.releaseLock()
  return text.split('\n')[0]?.trim() ?? ''
}

function alive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

export async function startServer(opts: StartOptions): Promise<ForgeServer> {
  const port = freePort()
  const logLevel = opts.logLevel ?? process.env.FORGE_LOG_LEVEL ?? 'warn'
  const proc = Bun.spawn(['sh', '-c', WATCHDOG, opts.bin, String(process.pid)], {
    env: { ...process.env, FORGE_DB_PATH: opts.dbPath, FORGE_PORT: String(port), FORGE_HOST: '127.0.0.1', FORGE_LOG_LEVEL: logLevel },
    stdout: 'pipe',
    stderr: 'inherit',
  })
  const pid = Number(await firstLine(proc.stdout))
  if (!Number.isInteger(pid) || pid <= 0) throw new Error('forge-server watchdog did not report a pid')
  const baseUrl = `http://127.0.0.1:${port}`

  const deadline = Date.now() + (opts.timeoutMs ?? 30_000)
  let up = false
  while (!up && Date.now() < deadline && alive(pid)) {
    try {
      up = (await fetch(`${baseUrl}/api/health`)).ok
    } catch {
      // not listening yet
    }
    if (!up) await Bun.sleep(50)
  }
  if (!up) {
    proc.kill('SIGTERM')
    await proc.exited
    throw new Error(`forge-server (${opts.bin}) did not answer /api/health on port ${port}`)
  }

  let stopped: Promise<void> | undefined
  return {
    baseUrl,
    port,
    dbPath: opts.dbPath,
    stop() {
      stopped ??= (async () => {
        if (alive(pid)) process.kill(pid, 'SIGINT')
        const deadline = Date.now() + 15_000
        while (alive(pid) && Date.now() < deadline) await Bun.sleep(25)
        if (alive(pid)) {
          process.kill(pid, 'SIGKILL')
          while (alive(pid)) await Bun.sleep(25)
          throw new Error('forge-server ignored SIGINT and was killed; its database may hold an unflushed WAL')
        }
        await proc.exited // the watchdog exits once the server is gone
      })()
      return stopped
    },
  }
}
