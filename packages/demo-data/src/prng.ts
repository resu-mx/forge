/**
 * Seeded PRNG: cyrb128 hashes a string to 128 bits, which seed sfc32.
 *
 * Used only for small, cosmetic variation (timestamp jitter, tie-breaks, salary jitter).
 * Every stream is derived from `(seed, persona, stream)`, so draws in one stream never
 * shift another, and the order in which phases consume streams does not matter.
 */

/** cyrb128 string hash: four 32-bit words. */
export function cyrb128(str: string): [number, number, number, number] {
  let h1 = 1779033703
  let h2 = 3144134277
  let h3 = 1013904242
  let h4 = 2773480762
  for (let i = 0; i < str.length; i++) {
    const k = str.charCodeAt(i)
    h1 = h2 ^ Math.imul(h1 ^ k, 597399067)
    h2 = h3 ^ Math.imul(h2 ^ k, 2869860233)
    h3 = h4 ^ Math.imul(h3 ^ k, 951274213)
    h4 = h1 ^ Math.imul(h4 ^ k, 2716044179)
  }
  h1 = Math.imul(h3 ^ (h1 >>> 18), 597399067)
  h2 = Math.imul(h4 ^ (h2 >>> 22), 2869860233)
  h3 = Math.imul(h1 ^ (h3 >>> 17), 951274213)
  h4 = Math.imul(h2 ^ (h4 >>> 19), 2716044179)
  h1 ^= h2 ^ h3 ^ h4
  h2 ^= h1
  h3 ^= h1
  h4 ^= h1
  return [h1 >>> 0, h2 >>> 0, h3 >>> 0, h4 >>> 0]
}

/** sfc32: returns floats in [0, 1). */
export function sfc32(a: number, b: number, c: number, d: number): () => number {
  return () => {
    a >>>= 0
    b >>>= 0
    c >>>= 0
    d >>>= 0
    let t = (a + b) | 0
    a = b ^ (b >>> 9)
    b = (c + (c << 3)) | 0
    c = (c << 21) | (c >>> 11)
    d = (d + 1) | 0
    t = (t + d) | 0
    c = (c + t) | 0
    return (t >>> 0) / 4294967296
  }
}

export interface Rng {
  /** A float in [0, 1). */
  next(): number
  /** An integer in [min, max], inclusive. */
  int(min: number, max: number): number
  /** One element of a non-empty array. */
  pick<T>(items: readonly T[]): T
}

/** An independent stream for `(seed, persona, stream)`. */
export function rng(seed: string, persona: string, stream: string): Rng {
  const next = sfc32(...cyrb128(`${seed}\u0000${persona}\u0000${stream}`))
  // Discard the first few outputs: sfc32 mixes poorly for its first draws.
  for (let i = 0; i < 12; i++) next()
  return {
    next,
    int(min, max) {
      if (!Number.isInteger(min) || !Number.isInteger(max) || max < min) {
        throw new Error(`rng.int: bad range [${min}, ${max}]`)
      }
      return min + Math.floor(next() * (max - min + 1))
    },
    pick(items) {
      if (items.length === 0) throw new Error('rng.pick: empty array')
      return items[Math.floor(next() * items.length)] as (typeof items)[number]
    },
  }
}
