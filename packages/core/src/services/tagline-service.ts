/**
 * TaglineService — programmatic tagline generation via TF-IDF keyword
 * extraction from job description text.
 *
 * Phase 92 introduces this service to replace the static tagline field on
 * summaries with an auto-generated tagline per resume. The tagline is
 * computed from all JDs linked to the resume and is regenerated whenever
 * JD links change.
 *
 * ## Algorithm
 *
 * 1. Tokenize each JD's raw_text into lowercased alphanumeric terms.
 * 2. Strip stop words (common English + job-description noise).
 * 3. For each term, compute TF-IDF across the JD corpus:
 *      tf     = term frequency in a single JD (0-1 normalized)
 *      idf    = log(N / df + 1), where N is # JDs and df is # JDs containing the term
 *      tfidf  = tf * idf
 * 4. Aggregate scores across all JDs (sum).
 * 5. Boost terms that match user skills by `SKILL_MATCH_BOOST` (case-insensitive).
 * 6. Return the top-K terms by score.
 *
 * The output is deterministic: ties are broken by alphabetical order so tests
 * can assert exact output.
 *
 * ## Non-goals
 *
 * - LLM-based generation (this is pure statistical extraction)
 * - Stemming / lemmatization (naive lowercase match only — "engineers" and
 *   "engineer" are distinct terms)
 * - Multi-word phrases (unigrams only)
 *
 * The service is stateless: all state lives in the inputs. Callers pass the
 * JD texts, user skills, and options. This makes it trivial to unit test and
 * call from both the API layer and ad-hoc scripts.
 */

import type { Database } from 'bun:sqlite'
import { buildDefaultElm } from '../storage/build-elm'
import type { EntityLifecycleManager } from '../storage/lifecycle-manager'
import type { Skill } from '../types'

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/** Default number of keywords returned. */
export const DEFAULT_TOP_K = 3

/** Boost multiplier applied when an extracted term matches a user skill. */
export const SKILL_MATCH_BOOST = 2.0

/**
 * Stop words — common English function words plus job-description filler.
 * Kept conservative; favor recall over precision so that TF-IDF has room to
 * down-weight genuinely noisy terms.
 */
const STOP_WORDS = new Set([
  // Common English
  'a', 'an', 'and', 'are', 'as', 'at', 'be', 'by', 'for', 'from',
  'has', 'have', 'had', 'he', 'her', 'his', 'i', 'if', 'in', 'into',
  'is', 'it', 'its', 'of', 'on', 'or', 'our', 'out', 'over', 'so',
  'such', 'that', 'the', 'their', 'then', 'there', 'these', 'they',
  'this', 'those', 'to', 'was', 'we', 'were', 'what', 'when', 'where',
  'which', 'while', 'who', 'will', 'with', 'would', 'you', 'your',
  'but', 'not', 'no', 'any', 'all', 'can', 'should', 'may', 'must',
  'do', 'does', 'done', 'been', 'am', 'us', 'me', 'my', 'him', 'she',
  // Job-description filler
  'job', 'role', 'work', 'working', 'company', 'team', 'teams',
  'experience', 'years', 'year', 'skills', 'skill', 'ability', 'abilities',
  'opportunity', 'opportunities', 'position', 'positions', 'candidate',
  'candidates', 'responsibilities', 'requirements', 'required', 'preferred',
  'strong', 'excellent', 'ideal', 'ideal', 'looking', 'seeking', 'join',
  'about', 'us', 'you', 'we', 'our', 'help', 'helping', 'make', 'making',
  'using', 'use', 'used', 'include', 'includes', 'including', 'across',
  'within', 'through', 'throughout', 'more', 'than', 'other', 'others',
  'also', 'both', 'well', 'very', 'most', 'some', 'each', 'every',
  'new', 'great', 'good', 'better', 'best', 'high', 'highly', 'one',
  'two', 'three', 'plus', 'etc', 'eg', 'ie',
])

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** A ranked keyword with its aggregated TF-IDF score. */
export interface RankedKeyword {
  term: string
  score: number
  /** True if the term matched a user skill and received the boost. */
  matchedSkill: boolean
}

export interface GenerateTaglineOptions {
  /** Number of keywords to include in the tagline (default: 3). */
  topK?: number
  /**
   * Optional prefix for the tagline (usually the target role or a JD title).
   * Rendered as `<prefix> -- keyword1 + keyword2 + keyword3`.
   * If omitted, the tagline is just `keyword1 + keyword2 + keyword3`.
   */
  prefix?: string
}

export interface GenerateTaglineResult {
  /** The formatted tagline string. Empty if no keywords could be extracted. */
  tagline: string
  /** Ranked keywords used to build the tagline, top-K only. */
  keywords: RankedKeyword[]
}

// ---------------------------------------------------------------------------
// Tokenization
// ---------------------------------------------------------------------------

/**
 * Tokenize a string into lowercased alphanumeric-plus-hyphen terms.
 * Terms under 2 characters or in the stop list are dropped.
 *
 * Hyphens and `+` are preserved within terms so labels like "c++",
 * "front-end", "ci-cd" survive. Leading/trailing punctuation is stripped.
 */
export function tokenize(text: string): string[] {
  if (!text) return []

  // Split on whitespace + common separators, but keep `+` and `-` inside words
  const raw = text
    .toLowerCase()
    .split(/[\s,;.!?()[\]{}"'/\\|<>]+/u)
    .map((t) => t.replace(/^[^a-z0-9]+|[^a-z0-9+\-#]+$/g, ''))
    .filter((t) => t.length >= 2 && !STOP_WORDS.has(t) && /[a-z]/.test(t))

  return raw
}

// ---------------------------------------------------------------------------
// TF-IDF
// ---------------------------------------------------------------------------

/**
 * Compute TF-IDF scores for all terms across a corpus of JD texts.
 *
 * Returns a Map keyed by term with aggregated (summed) TF-IDF scores across
 * the corpus. If a term appears in multiple JDs, its scores add up so
 * multi-JD repetition is rewarded.
 *
 * `tf` is 0-1 normalized against the longest JD in the corpus.
 * `idf = log((N + 1) / (df + 1)) + 1` (smoothed, always > 0).
 */
export function computeTfIdf(jdTexts: string[]): Map<string, number> {
  const aggregate = new Map<string, number>()
  if (jdTexts.length === 0) return aggregate

  // Tokenize each JD once; keep raw term arrays.
  const tokenized = jdTexts.map((t) => tokenize(t))

  // Document frequency: how many JDs contain each term
  const docFreq = new Map<string, number>()
  for (const tokens of tokenized) {
    const unique = new Set(tokens)
    for (const term of unique) {
      docFreq.set(term, (docFreq.get(term) ?? 0) + 1)
    }
  }

  const N = tokenized.length

  for (const tokens of tokenized) {
    if (tokens.length === 0) continue

    // Term frequency within this JD
    const tf = new Map<string, number>()
    for (const term of tokens) {
      tf.set(term, (tf.get(term) ?? 0) + 1)
    }

    // Normalize TF by the max term frequency in this JD (range-safe, favors
    // the dominant term without penalizing longer documents).
    let maxCount = 0
    for (const count of tf.values()) {
      if (count > maxCount) maxCount = count
    }

    for (const [term, count] of tf.entries()) {
      const normalizedTf = count / maxCount
      const df = docFreq.get(term) ?? 1
      // Smoothed IDF: always positive, never -Infinity.
      const idf = Math.log((N + 1) / (df + 1)) + 1
      const score = normalizedTf * idf
      aggregate.set(term, (aggregate.get(term) ?? 0) + score)
    }
  }

  return aggregate
}

// ---------------------------------------------------------------------------
// Skill-match boost + ranking
// ---------------------------------------------------------------------------

/**
 * Rank keywords from a TF-IDF score map, applying the skill-match boost for
 * terms that case-insensitively match any user skill name.
 *
 * Ties are broken alphabetically (ascending) so output is deterministic.
 */
export function rankKeywords(
  scores: Map<string, number>,
  userSkills: Skill[],
): RankedKeyword[] {
  const skillSet = new Set(userSkills.map((s) => s.name.toLowerCase()))

  const ranked: RankedKeyword[] = []
  for (const [term, baseScore] of scores.entries()) {
    const matched = skillSet.has(term)
    const score = matched ? baseScore * SKILL_MATCH_BOOST : baseScore
    ranked.push({ term, score, matchedSkill: matched })
  }

  ranked.sort((a, b) => {
    if (b.score !== a.score) return b.score - a.score
    return a.term.localeCompare(b.term)
  })

  return ranked
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Generate a tagline from one or more JD texts and the user's skill library.
 *
 * Returns the formatted tagline plus the top-K ranked keywords. An empty
 * corpus or a corpus with no extractable terms yields an empty tagline.
 *
 * Example (with prefix):
 *   generateTagline(['JD text about python and terraform...'], skills, {
 *     topK: 3,
 *     prefix: 'Senior DevOps',
 *   })
 *   => { tagline: 'Senior DevOps -- python + terraform + ansible', ... }
 */
export function generateTagline(
  jdTexts: string[],
  userSkills: Skill[],
  options: GenerateTaglineOptions = {},
): GenerateTaglineResult {
  const topK = options.topK ?? DEFAULT_TOP_K
  const nonEmptyTexts = jdTexts.filter((t) => t && t.trim().length > 0)

  if (nonEmptyTexts.length === 0) {
    return { tagline: '', keywords: [] }
  }

  const scores = computeTfIdf(nonEmptyTexts)
  const ranked = rankKeywords(scores, userSkills).slice(0, topK)

  if (ranked.length === 0) {
    return { tagline: '', keywords: [] }
  }

  const joined = ranked.map((k) => k.term).join(' + ')
  const tagline = options.prefix ? `${options.prefix} -- ${joined}` : joined

  return { tagline, keywords: ranked }
}

// ---------------------------------------------------------------------------
// Blank tagline text (resu-mx/forge#72)
// ---------------------------------------------------------------------------

/**
 * The one rule for a tagline column that may hold text: an empty or whitespace-only string is
 * absent. The writers already store blank as NULL, so this only matters for a row written another
 * way. Every reader (`resolved`, `has_override`, the resume header) uses it, in both runtimes.
 * The text is returned as stored, never trimmed.
 */
export function nonBlank(value: string | null | undefined): string | null {
  return value && value.trim().length > 0 ? value : null
}

/**
 * What a resume's two tagline columns resolve to: a real override, else the generated tagline,
 * else the empty string. `has_override` agrees with `resolved` by construction.
 */
export function resolveTagline(
  override: string | null | undefined,
  generated: string | null | undefined,
): { resolved: string; has_override: boolean } {
  const real = nonBlank(override)
  return { resolved: real ?? nonBlank(generated) ?? '', has_override: real !== null }
}

// ---------------------------------------------------------------------------
// DB-backed regeneration (Phase 92 T92.3)
// ---------------------------------------------------------------------------

export interface RegenerateResult {
  /** The regenerated tagline text (may be empty if no JDs are linked). */
  generated_tagline: string
  /** True if the resume has a tagline_override that is currently shadowing
   *  the generated value. UI should prompt the user to review. */
  has_override: boolean
  /** Top-K ranked keywords used for the generated tagline. */
  keywords: RankedKeyword[]
}

/**
 * Regenerate `resume.generated_tagline` from all JDs currently linked to the
 * resume, matching against all skills in the user's library. The stored
 * value is updated in place. `tagline_override` is left untouched so the
 * user's manual override survives regeneration.
 *
 * Returns the new generated tagline, a flag indicating whether an override
 * is present (so the caller can prompt the user to review), and the ranked
 * keywords for display.
 *
 * Returns null if the resume does not exist. Returns an empty tagline (not
 * null) if the resume exists but has no JDs linked — in that case the
 * `generated_tagline` column is set to NULL and the caller gets an empty
 * string plus `has_override` reflecting the current override state.
 *
 * Phase 1.4: uses EntityLifecycleManager. Optional elm parameter; builds
 * a default one from db when not provided.
 */
export async function regenerateResumeTagline(
  db: Database,
  resumeId: string,
  options: GenerateTaglineOptions = {},
  elm?: EntityLifecycleManager,
): Promise<RegenerateResult | null> {
  const elmi = elm ?? buildDefaultElm(db)

  // Sanity check + existing override lookup
  const resumeResult = await elmi.get('resumes', resumeId)
  if (!resumeResult.ok) return null
  const resumeRow = resumeResult.value as Record<string, unknown>

  // Fetch raw_text for all linked JDs via the junction table
  const jdrResult = await elmi.list('job_description_resumes', {
    where: { resume_id: resumeId },
    limit: 10000,
  })
  const jdTexts: string[] = []
  if (jdrResult.ok) {
    for (const jdr of jdrResult.value.rows) {
      const jdResult = await elmi.get('job_descriptions', jdr.job_description_id as string)
      if (jdResult.ok) {
        const rawText = jdResult.value.raw_text as string | null
        if (rawText && rawText.length > 0) {
          jdTexts.push(rawText)
        }
      }
    }
  }

  // Fetch all skills (used for the match boost) — listAll equivalent
  const skillResult = await elmi.list('skills', {
    limit: 100000,
    orderBy: [{ field: 'name', direction: 'asc' }],
  })
  const skillRows: Skill[] = skillResult.ok
    ? (skillResult.value.rows as unknown as Skill[])
    : []

  // Use the resume's target_role as the default prefix if none provided
  const targetRole = resumeRow.target_role as string | null
  const prefix = options.prefix ?? targetRole ?? undefined
  const result = generateTagline(jdTexts, skillRows, { ...options, prefix })

  // Persist. Empty taglines are stored as NULL so the compiler's fallback
  // chain (tagline_override ?? generated_tagline ?? header ?? target_role)
  // works correctly when no JDs are linked.
  await elmi.update('resumes', resumeId, {
    generated_tagline: result.tagline || null,
  })

  const taglineOverride = resumeRow.tagline_override as string | null
  return {
    generated_tagline: result.tagline,
    has_override: nonBlank(taglineOverride) !== null,
    keywords: result.keywords,
  }
}
