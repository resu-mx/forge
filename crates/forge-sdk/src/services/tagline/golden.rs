//! Golden cases shared with TS: `packages/core/src/services/__tests__/tagline-golden.test.ts`
//! writes and checks the fixture; this module checks the Rust port against it.

use std::path::Path;

use serde_json::Value;

use super::*;
use forge_core::RankedKeyword;

const FIXTURE: &str = include_str!(
    "../../../../../packages/core/src/services/__tests__/fixtures/tagline-golden.json"
);

/// The 27 tests in `packages/core/src/services/__tests__/tagline-service.test.ts`.
const TS_TESTS: [&str; 27] = [
    "tokenize > lowercases input",
    "tokenize > drops stop words",
    "tokenize > drops tokens under 2 characters",
    "tokenize > strips punctuation around tokens",
    "tokenize > preserves internal hyphens and plus signs",
    "tokenize > drops pure numbers",
    "tokenize > empty input returns empty array",
    "tokenize > drops common job-description filler words",
    "computeTfIdf > empty corpus yields empty map",
    "computeTfIdf > single JD assigns positive scores to every term",
    "computeTfIdf > term frequency in one JD boosts score",
    "computeTfIdf > cross-JD repetition still accumulates score",
    "computeTfIdf > ubiquitous terms get down-weighted relative to unique terms",
    "computeTfIdf > stop words are excluded from scores",
    "rankKeywords > returns entries sorted by score descending",
    "rankKeywords > skill match boosts score by SKILL_MATCH_BOOST",
    "rankKeywords > ties break alphabetically ascending",
    "rankKeywords > empty scores yield empty ranking",
    "generateTagline > empty corpus yields empty tagline",
    "generateTagline > whitespace-only texts yield empty tagline",
    "generateTagline > produces top-K keywords joined by \" + \"",
    "generateTagline > prefix renders as \"<prefix> -- <keywords>\"",
    "generateTagline > default topK is DEFAULT_TOP_K",
    "generateTagline > skill-matched terms rank above non-matched with equal base score",
    "generateTagline > handles multi-JD corpus and aggregates scores",
    "generateTagline > matches skills case-insensitively",
    "generateTagline > omitting prefix yields bare keyword list",
];

const TOL: f64 = 1e-9;

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect()
}

fn assert_keywords(name: &str, got: &[RankedKeyword], want: &Value) {
    let want = want.as_array().unwrap();
    assert_eq!(got.len(), want.len(), "{name}: keyword count");
    for (g, w) in got.iter().zip(want) {
        assert_eq!(g.term, w["term"].as_str().unwrap(), "{name}: term order");
        assert_eq!(
            g.matched_skill,
            w["matchedSkill"].as_bool().unwrap(),
            "{name}: matchedSkill"
        );
        let ws = w["score"].as_f64().unwrap();
        assert!(
            (g.score - ws).abs() <= TOL,
            "{name}: {} scored {} vs TS {ws}",
            g.term,
            g.score
        );
    }
}

fn check_generate(name: &str, texts: &[String], case: &Value, want: &Value) {
    let top_k = case["topK"].as_u64().map_or(DEFAULT_TOP_K, |k| k as usize);
    let got = generate_tagline(
        texts,
        &strings(&case["skills"]),
        top_k,
        case["prefix"].as_str(),
    );
    assert_eq!(
        got.tagline,
        want["tagline"].as_str().unwrap(),
        "{name}: tagline"
    );
    assert_keywords(name, &got.keywords, &want["keywords"]);
}

#[test]
fn golden_fixture_covers_every_ts_test() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let names: Vec<&str> = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    for t in TS_TESTS {
        assert!(names.contains(&t), "fixture has no case for TS test {t:?}");
    }
}

#[test]
fn golden_cases_match_ts() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let want = &case["expected"];
        match case["fn"].as_str().unwrap() {
            "tokenize" => assert_eq!(
                tokenize(case["text"].as_str().unwrap_or("")),
                strings(want),
                "{name}"
            ),
            "computeTfIdf" => {
                let got = compute_tf_idf(&strings(&case["texts"]));
                let want = want.as_array().unwrap();
                assert_eq!(got.len(), want.len(), "{name}: term count");
                for ((term, score), w) in got.iter().zip(want) {
                    assert_eq!(term, w[0].as_str().unwrap(), "{name}: insertion order");
                    assert!(
                        (score - w[1].as_f64().unwrap()).abs() <= TOL,
                        "{name}: {term}"
                    );
                }
            }
            "rankKeywords" => {
                let scores: Vec<(String, f64)> = case["scores"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| (p[0].as_str().unwrap().to_owned(), p[1].as_f64().unwrap()))
                    .collect();
                assert_keywords(
                    name,
                    &rank_keywords(&scores, &strings(&case["skills"])),
                    want,
                );
            }
            "generateTagline" => check_generate(name, &strings(&case["texts"]), case, want),
            other => panic!("{name}: unknown fn {other}"),
        }
    }
}

#[test]
fn golden_corpus_matches_ts() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/core/src/parser/__tests__/fixtures");
    let corpus = fixture["corpus"].as_array().unwrap();
    assert!(corpus.len() >= 24, "corpus covers the 24 JD texts");
    for case in corpus {
        let name = case["name"].as_str().unwrap();
        let texts: Vec<String> = strings(&case["files"])
            .iter()
            .map(|f| std::fs::read_to_string(dir.join(f)).unwrap())
            .collect();
        check_generate(name, &texts, case, &case["expected"]);
    }
}
