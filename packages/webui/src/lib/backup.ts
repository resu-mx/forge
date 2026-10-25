import type { ForgeRuntime } from '@forge/runtime'

/**
 * Download the whole database as a SQLite file (what Settings, Storage calls "Export database").
 * Shared with the cloud-sync conflict dialog, which offers it before anything is replaced.
 * Resolves to the size in bytes.
 */
export async function downloadBackup(runtime: Pick<ForgeRuntime, 'exportDatabase'>): Promise<number> {
  const { bytes } = await runtime.exportDatabase()
  const blob = new Blob([bytes as BlobPart], { type: 'application/vnd.sqlite3' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `forge-${new Date().toISOString().slice(0, 10)}.db`
  a.click()
  URL.revokeObjectURL(url)
  return bytes.byteLength
}
