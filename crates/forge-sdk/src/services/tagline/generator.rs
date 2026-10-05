//! Pure TF-IDF tagline generator, ported from `packages/core/src/services/tagline-service.ts:46-267`.
//! Port of the code, not its comments: the header (`:16`) and the `computeTfIdf` doc (`:146`)
//! misdescribe the IDF and TF normalisation.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use forge_core::RankedKeyword;
use icu_collator::options::CollatorOptions;
use icu_collator::CollatorBorrowed;

use super::{is_js_whitespace, js_trim};

/// Default number of keywords in a tagline (`:47`).
pub const DEFAULT_TOP_K: usize = 3;
/// Score multiplier for a term equal to a lowercased skill name (`:50`).
pub const SKILL_MATCH_BOOST: f64 = 2.0;

/// `STOP_WORDS`, copied verbatim from `:57-79`: 150 entries, 145 unique.
const STOP_WORDS: [&str; 150] = [
    // Common English
    "a",
    "an",
    "and",
    "are",
    "as",
    "at",
    "be",
    "by",
    "for",
    "from",
    "has",
    "have",
    "had",
    "he",
    "her",
    "his",
    "i",
    "if",
    "in",
    "into",
    "is",
    "it",
    "its",
    "of",
    "on",
    "or",
    "our",
    "out",
    "over",
    "so",
    "such",
    "that",
    "the",
    "their",
    "then",
    "there",
    "these",
    "they",
    "this",
    "those",
    "to",
    "was",
    "we",
    "were",
    "what",
    "when",
    "where",
    "which",
    "while",
    "who",
    "will",
    "with",
    "would",
    "you",
    "your",
    "but",
    "not",
    "no",
    "any",
    "all",
    "can",
    "should",
    "may",
    "must",
    "do",
    "does",
    "done",
    "been",
    "am",
    "us",
    "me",
    "my",
    "him",
    "she",
    // Job-description filler
    "job",
    "role",
    "work",
    "working",
    "company",
    "team",
    "teams",
    "experience",
    "years",
    "year",
    "skills",
    "skill",
    "ability",
    "abilities",
    "opportunity",
    "opportunities",
    "position",
    "positions",
    "candidate",
    "candidates",
    "responsibilities",
    "requirements",
    "required",
    "preferred",
    "strong",
    "excellent",
    "ideal",
    "ideal",
    "looking",
    "seeking",
    "join",
    "about",
    "us",
    "you",
    "we",
    "our",
    "help",
    "helping",
    "make",
    "making",
    "using",
    "use",
    "used",
    "include",
    "includes",
    "including",
    "across",
    "within",
    "through",
    "throughout",
    "more",
    "than",
    "other",
    "others",
    "also",
    "both",
    "well",
    "very",
    "most",
    "some",
    "each",
    "every",
    "new",
    "great",
    "good",
    "better",
    "best",
    "high",
    "highly",
    "one",
    "two",
    "three",
    "plus",
    "etc",
    "eg",
    "ie",
];

/// Formatted tagline plus the top-K keywords (TS `GenerateTaglineResult`, `:104-109`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GeneratedTagline {
    pub tagline: String,
    pub keywords: Vec<RankedKeyword>,
}

/// The TS split class `[\s,;.!?()[\]{}"'/\\|<>]`, with JS `\s` semantics (`:128`).
fn is_separator(c: char) -> bool {
    is_js_whitespace(c)
        || matches!(
            c,
            ',' | ';'
                | '.'
                | '!'
                | '?'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '"'
                | '\''
                | '/'
                | '\\'
                | '|'
                | '<'
                | '>'
        )
}

fn is_stop_word(token: &str) -> bool {
    STOP_WORDS.contains(&token)
}

/// `tokenize` (`:122-133`).
pub fn tokenize(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.to_lowercase()
        .split(is_separator)
        // `/^[^a-z0-9]+|[^a-z0-9+\-#]+$/g`: ASCII classes on the lowercased piece.
        .map(|raw| {
            raw.trim_start_matches(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
                .trim_end_matches(|c: char| {
                    !(c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '-' | '#'))
                })
        })
        // JS `t.length >= 2` counts UTF-16 units. A surviving token starts and ends with
        // ASCII, so byte length gives the same answer.
        .filter(|t| t.len() >= 2 && t.bytes().any(|b| b.is_ascii_lowercase()) && !is_stop_word(t))
        .map(str::to_owned)
        .collect()
}

/// `computeTfIdf` (`:149-194`). Scores come back in TS `Map` insertion order.
pub fn compute_tf_idf<S: AsRef<str>>(jd_texts: &[S]) -> Vec<(String, f64)> {
    let tokenized: Vec<Vec<String>> = jd_texts.iter().map(|t| tokenize(t.as_ref())).collect();

    // Lookups only: no HashMap iteration reaches the output.
    let mut doc_freq: HashMap<&str, u32> = HashMap::new();
    for tokens in &tokenized {
        let unique: HashSet<&str> = tokens.iter().map(String::as_str).collect();
        for term in unique {
            *doc_freq.entry(term).or_insert(0) += 1;
        }
    }
    let n = tokenized.len() as f64;

    let mut aggregate: Vec<(String, f64)> = Vec::new();
    let mut slot: HashMap<&str, usize> = HashMap::new();
    for tokens in &tokenized {
        if tokens.is_empty() {
            continue; // counts toward N, contributes nothing
        }
        // Per-JD counts in first-occurrence order (the TS `tf` Map).
        let mut counts: Vec<(&str, u32)> = Vec::new();
        let mut at: HashMap<&str, usize> = HashMap::new();
        for term in tokens {
            match at.get(term.as_str()) {
                Some(&i) => counts[i].1 += 1,
                None => {
                    at.insert(term.as_str(), counts.len());
                    counts.push((term.as_str(), 1));
                }
            }
        }
        let max_count = counts.iter().map(|&(_, c)| c).max().unwrap_or(1) as f64;
        for (term, count) in counts {
            let df = doc_freq.get(term).copied().unwrap_or(1) as f64;
            let idf = ((n + 1.0) / (df + 1.0)).ln() + 1.0;
            let score = (count as f64 / max_count) * idf;
            match slot.get(term) {
                Some(&i) => aggregate[i].1 += score,
                None => {
                    slot.insert(term, aggregate.len());
                    aggregate.push((term.to_owned(), score));
                }
            }
        }
    }
    aggregate
}

/// Bun's `localeCompare`: ICU, en-US (CLDR root), tertiary strength, punctuation not ignored.
pub(crate) fn compare_terms(a: &str, b: &str) -> Ordering {
    // `new_root` is a cheap `const fn` over static data.
    const COLLATOR: CollatorBorrowed<'static> =
        CollatorBorrowed::new_root(CollatorOptions::default());
    COLLATOR.compare(a, b)
}

/// `rankKeywords` (`:206-225`).
pub fn rank_keywords<K: AsRef<str>>(
    scores: &[(String, f64)],
    skill_names: &[K],
) -> Vec<RankedKeyword> {
    let skills: HashSet<String> = skill_names
        .iter()
        .map(|s| s.as_ref().to_lowercase())
        .collect();
    let mut ranked: Vec<RankedKeyword> = scores
        .iter()
        .map(|(term, base)| {
            let matched = skills.contains(term);
            RankedKeyword {
                term: term.clone(),
                score: if matched {
                    base * SKILL_MATCH_BOOST
                } else {
                    *base
                },
                matched_skill: matched,
            }
        })
        .collect();
    // Stable, like Array.prototype.sort.
    ranked.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| compare_terms(&a.term, &b.term))
    });
    ranked
}

/// `generateTagline` (`:244-267`).
pub fn generate_tagline<S: AsRef<str>, K: AsRef<str>>(
    jd_texts: &[S],
    skill_names: &[K],
    top_k: usize,
    prefix: Option<&str>,
) -> GeneratedTagline {
    let texts: Vec<&str> = jd_texts
        .iter()
        .map(AsRef::as_ref)
        .filter(|t| !js_trim(t).is_empty())
        .collect();
    if texts.is_empty() {
        return GeneratedTagline::default();
    }
    let mut keywords = rank_keywords(&compute_tf_idf(&texts), skill_names);
    keywords.truncate(top_k);
    if keywords.is_empty() {
        return GeneratedTagline::default();
    }
    let joined = keywords
        .iter()
        .map(|k| k.term.as_str())
        .collect::<Vec<_>>()
        .join(" + ");
    let tagline = match prefix {
        Some(p) if !p.is_empty() => format!("{p} -- {joined}"),
        _ => joined,
    };
    GeneratedTagline { tagline, keywords }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_examples() {
        assert_eq!(
            tokenize("Front-end (React/TypeScript), C++ and CI-CD!"),
            ["front-end", "react", "typescript", "c++", "ci-cd"]
        );
        let none: [&str; 0] = [];
        let g = generate_tagline(
            &[
                "kubernetes terraform python",
                "kubernetes prometheus grafana",
            ],
            &none,
            3,
            Some("SRE"),
        );
        assert_eq!(g.tagline, "SRE -- kubernetes + grafana + prometheus");
        assert!((g.keywords[1].score - (1.5f64.ln() + 1.0)).abs() < 1e-12);
    }

    #[test]
    fn tie_order_matches_bun_locale_compare() {
        // Ascending orders observed with Bun 1.4.2 `localeCompare`.
        for expected in [
            &["a-b", "a#b", "a+b", "a1", "ab"][..],
            &["c", "c-", "c-d", "c#", "c++", "c0", "ca"][..],
            &["company-s", "company\u{2019}s", "companys"][..],
            &["naive", "na\u{EF}ve", "naivf"][..],
            &["we-re", "we\u{2019}re", "wer", "west"][..],
        ] {
            let mut got: Vec<&str> = expected.iter().rev().copied().collect();
            got.sort_by(|a, b| compare_terms(a, b));
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn stop_words_are_the_ts_list() {
        let unique: HashSet<&str> = STOP_WORDS.iter().copied().collect();
        assert_eq!((STOP_WORDS.len(), unique.len()), (150, 145));
    }

    #[test]
    fn js_whitespace_in_tokenize() {
        assert_eq!(tokenize("alpha\u{FEFF}beta"), ["alpha", "beta"]);
        assert_eq!(tokenize("gamma\u{0085}delta"), ["gamma\u{0085}delta"]);
    }

    #[test]
    fn prefix_is_never_emitted_alone_or_when_empty() {
        let none: [&str; 0] = [];
        assert_eq!(
            generate_tagline(&["the and"], &none, 3, Some("SRE")).tagline,
            ""
        );
        assert_eq!(
            generate_tagline(&["alpha beta"], &none, 3, Some("")).tagline,
            "alpha + beta"
        );
    }
}
