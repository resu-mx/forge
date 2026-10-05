//! JD Requirement Parser -- programmatic extraction of individual requirements
//! from raw job description text.
//!
//! Splits on bullet points, numbered lists, and line breaks within
//! recognized sections (Requirements, Qualifications, etc.).
//!
//! Confidence scoring:
//! - 0.9: Structured list with clear bullet points under requirement sections
//! - 0.7: Structured list under responsibility sections (lower weight for alignment)
//! - 0.7-0.9: Semi-structured (line breaks, mixed formatting)
//! - 0.5-0.7: Some structure detected but ambiguous
//! - < 0.5: Prose paragraphs, hard to parse reliably
//!
//! NOTE (M4): Responsibility-section requirements are scored at 0.7 instead of 0.9
//! because responsibilities are less precise signals for skills matching than explicit
//! requirements/qualifications sections.

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

// ── Types ────────────────────────────────────────────────────────────

/// A single parsed requirement extracted from a job description.
pub struct ParsedRequirement {
    pub text: String,
    pub confidence: f64,
    pub section: Option<String>,
}

/// Collection of parsed requirements with an overall confidence score.
pub struct ParsedRequirements {
    pub requirements: Vec<ParsedRequirement>,
    pub overall_confidence: f64,
}

// ── Section Detection ────────────────────────────────────────────────

/// Known section headers that typically contain requirements.
static REQUIREMENT_SECTIONS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?im)^#{1,3}\s*(requirements|required\s+qualifications|minimum\s+qualifications|must[\s-]haves?)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(qualifications|preferred\s+qualifications|desired\s+qualifications)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(what\s+you(?:'ll|\s+will)\s+(?:need|bring)|what\s+we(?:'re|\s+are)\s+looking\s+for)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(responsibilities|key\s+responsibilities|role\s+responsibilities)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(nice[\s-]to[\s-]haves?|preferred|bonus|plus)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(skills|technical\s+skills|required\s+skills)").unwrap(),
        Regex::new(r"(?im)^\*{0,2}(requirements|qualifications|responsibilities|skills|what\s+you.+need)\*{0,2}\s*:?\s*$").unwrap(),
        Regex::new(r"(?im)^(requirements|qualifications|responsibilities|skills|what\s+you.+need)\s*:?\s*$").unwrap(),
    ]
});

/// Patterns that match responsibility-section headers specifically.
static RESPONSIBILITY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)responsibilities").unwrap());

/// Section headers that indicate the end of requirements (e.g., Benefits, About Us).
static NON_REQUIREMENT_SECTIONS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?im)^#{1,3}\s*(benefits|perks|compensation|salary|about\s+(?:us|the\s+company|the\s+team))").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(how\s+to\s+apply|application\s+process|equal\s+opportunity)").unwrap(),
        Regex::new(r"(?im)^#{1,3}\s*(company\s+(?:overview|description)|our\s+(?:mission|values|culture))").unwrap(),
        Regex::new(r"(?im)^\*{0,2}(benefits|perks|about\s+(?:us|the))\*{0,2}\s*:?\s*$").unwrap(),
        Regex::new(r"(?im)^(benefits|perks|about\s+(?:us|the))\s*:?\s*$").unwrap(),
    ]
});

// ── Bullet/List Detection ────────────────────────────────────────────

/// Matches lines starting with bullet characters or numbered list markers.
static BULLET_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\s]*(?:[-*+]|\d+[.)]\s|[a-z][.)]\s|>\s)").unwrap());

/// Sentence boundary pattern: period followed by space(s) and an uppercase letter.
/// The `regex` crate does not support look-ahead, so we match the leading uppercase
/// letter as part of the match and reconstruct splits manually in `split_sentences`.
static SENTENCE_BOUNDARY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.\s+([A-Z])").unwrap());

// ── Core Parser ──────────────────────────────────────────────────────

/// Parse requirements from raw job description text.
///
/// Strategy:
/// 1. Guard against excessively long input (max 100,000 chars).
/// 2. Detect requirement sections by header patterns.
/// 3. Extract content between requirement headers and next section header.
/// 4. Split section content on bullet points, numbered lists, or line breaks.
/// 5. Score each requirement based on how structured its source was.
/// 6. If no sections detected, attempt to parse the entire text.
pub fn parse_requirements(raw_text: &str) -> ParsedRequirements {
    if raw_text.trim().is_empty() {
        return ParsedRequirements {
            requirements: vec![],
            overall_confidence: 0.0,
        };
    }

    // IN3: Max-length guard
    if raw_text.len() > 100_000 {
        return ParsedRequirements {
            requirements: vec![],
            overall_confidence: 0.0,
        };
    }

    let lines: Vec<&str> = raw_text.split('\n').collect();
    let sections = detect_sections(&lines);

    let mut requirements: Vec<ParsedRequirement>;

    if !sections.is_empty() {
        // Parse structured sections
        requirements = Vec::new();
        for section in &sections {
            let is_responsibility = RESPONSIBILITY_PATTERN.is_match(&section.name);
            let parsed =
                parse_section_content(&section.content, Some(&section.name), is_responsibility);
            requirements.extend(parsed);
        }
    } else {
        // No sections detected -- try parsing the whole text
        requirements = parse_section_content(raw_text, None, false);
        // Lower confidence since we could not find section boundaries
        for req in &mut requirements {
            req.confidence *= 0.6;
        }
    }

    // Filter out empty or too-short requirements
    requirements.retain(|r| r.text.len() >= 10);

    // Deduplicate by normalized text
    let mut seen = HashSet::new();
    requirements.retain(|r| {
        let key = r.text.to_lowercase().trim().to_string();
        seen.insert(key)
    });

    let overall_confidence = if requirements.is_empty() {
        0.0
    } else {
        let sum: f64 = requirements.iter().map(|r| r.confidence).sum();
        sum / requirements.len() as f64
    };

    ParsedRequirements {
        requirements,
        overall_confidence,
    }
}

// ── Internal Helpers ─────────────────────────────────────────────────

struct DetectedSection {
    name: String,
    content: String,
}

fn detect_sections(lines: &[&str]) -> Vec<DetectedSection> {
    let mut sections = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;

    for line in lines {
        // Check if this line is a non-requirement section header (end current section)
        let is_end_section = NON_REQUIREMENT_SECTIONS
            .iter()
            .any(|pat| pat.is_match(line));
        if is_end_section {
            if let Some((name, content_lines)) = current.take() {
                sections.push(DetectedSection {
                    name,
                    content: content_lines.join("\n"),
                });
            }
            continue;
        }

        // Check if this line is a requirement section header (start new section)
        let mut matched = false;
        for pat in REQUIREMENT_SECTIONS.iter() {
            if let Some(caps) = pat.captures(line) {
                // Close previous section if any
                if let Some((name, content_lines)) = current.take() {
                    sections.push(DetectedSection {
                        name,
                        content: content_lines.join("\n"),
                    });
                }
                let section_name = caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "Requirements".to_string());
                current = Some((section_name, Vec::new()));
                matched = true;
                break;
            }
        }

        // Accumulate lines into current section (skip the header line itself)
        if !matched {
            if let Some((_, ref mut content_lines)) = current {
                content_lines.push(line);
            }
        }
    }

    // Close final section
    if let Some((name, content_lines)) = current {
        sections.push(DetectedSection {
            name,
            content: content_lines.join("\n"),
        });
    }

    sections
}

fn parse_section_content(
    content: &str,
    section_name: Option<&str>,
    is_responsibility_section: bool,
) -> Vec<ParsedRequirement> {
    let mut requirements = Vec::new();
    let lines: Vec<&str> = content.split('\n').collect();

    // M4: Responsibility sections get lower base confidence (0.7 vs 0.9)
    let base_confidence: f64 = if is_responsibility_section { 0.7 } else { 0.9 };

    // Count how many lines look like bullets
    let bullet_count = lines.iter().filter(|l| BULLET_PATTERN.is_match(l)).count();
    let is_bullet_list = bullet_count >= 1;

    if is_bullet_list {
        // Structured bullet list -- high confidence
        for line in &lines {
            let trimmed = BULLET_PATTERN.replace(line, "").trim().to_string();
            if trimmed.is_empty() {
                continue;
            }

            // Check for semicolon-delimited sub-items
            let parts: Vec<&str> = trimmed.split(';').collect();
            if parts.len() >= 3 {
                for part in parts {
                    let sub = part.trim().to_string();
                    if sub.len() >= 10 {
                        requirements.push(ParsedRequirement {
                            text: sub,
                            confidence: base_confidence - 0.1,
                            section: section_name.map(|s| s.to_string()),
                        });
                    }
                }
            } else {
                requirements.push(ParsedRequirement {
                    text: trimmed,
                    confidence: base_confidence,
                    section: section_name.map(|s| s.to_string()),
                });
            }
        }
    } else {
        // No clear bullet structure -- try splitting on line breaks
        let non_empty: Vec<String> = lines
            .iter()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        if non_empty.len() == 1 {
            // Single block of text -- try splitting on sentences
            let sentences = split_sentences(&non_empty[0]);
            for sentence in &sentences {
                let trimmed = sentence.trim();
                if trimmed.len() >= 10 {
                    requirements.push(ParsedRequirement {
                        text: trimmed.to_string(),
                        confidence: 0.4,
                        section: section_name.map(|s| s.to_string()),
                    });
                }
            }
        } else {
            // Multiple lines without bullet markers -- medium confidence
            for line in &non_empty {
                requirements.push(ParsedRequirement {
                    text: line.clone(),
                    confidence: 0.6,
                    section: section_name.map(|s| s.to_string()),
                });
            }
        }
    }

    requirements
}

/// Split text on sentence boundaries (`. ` followed by uppercase letter) without
/// consuming the uppercase letter. The `regex` crate lacks look-ahead, so we
/// capture the leading letter and prepend it to the next segment.
fn split_sentences(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut last = 0;

    for m in SENTENCE_BOUNDARY.find_iter(text) {
        // The match includes `. X` where X is the uppercase letter.
        // We want to split *before* the uppercase letter.
        let end_of_period_and_space = m.end() - 1; // just before the uppercase letter
        results.push(text[last..m.start()].to_string());
        last = end_of_period_and_space;
    }
    results.push(text[last..].to_string());

    results
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let result = parse_requirements("");
        assert!(result.requirements.is_empty());
        assert_eq!(result.overall_confidence, 0.0);
    }

    #[test]
    fn whitespace_only_returns_empty() {
        let result = parse_requirements("   \n\n  \t  ");
        assert!(result.requirements.is_empty());
        assert_eq!(result.overall_confidence, 0.0);
    }

    #[test]
    fn oversized_input_returns_empty() {
        let input = "a".repeat(100_001);
        let result = parse_requirements(&input);
        assert!(result.requirements.is_empty());
        assert_eq!(result.overall_confidence, 0.0);
    }

    #[test]
    fn parses_structured_requirements_section() {
        let input = "\
## Requirements
- 5+ years of experience with Rust
- Strong understanding of distributed systems
- Experience with cloud platforms (AWS, GCP)
## Benefits
- Health insurance
- 401k matching";
        let result = parse_requirements(input);
        assert_eq!(result.requirements.len(), 3);
        for req in &result.requirements {
            assert_eq!(req.confidence, 0.9);
            assert_eq!(req.section.as_deref(), Some("Requirements"));
        }
        // Must NOT include the benefits items
        let texts: Vec<&str> = result
            .requirements
            .iter()
            .map(|r| r.text.as_str())
            .collect();
        assert!(!texts.iter().any(|t| t.contains("Health insurance")));
        assert!(!texts.iter().any(|t| t.contains("401k")));
    }

    #[test]
    fn responsibility_section_gets_lower_confidence() {
        let input = "\
## Responsibilities
- Design and implement microservices architecture
- Lead code reviews and mentor junior engineers";
        let result = parse_requirements(input);
        assert!(!result.requirements.is_empty());
        for req in &result.requirements {
            assert!(
                (req.confidence - 0.7).abs() < f64::EPSILON,
                "expected 0.7, got {}",
                req.confidence
            );
        }
    }

    #[test]
    fn no_sections_applies_multiplier() {
        let input = "\
- Build and maintain CI/CD pipelines
- Monitor production systems for reliability";
        let result = parse_requirements(input);
        assert!(!result.requirements.is_empty());
        for req in &result.requirements {
            // 0.9 * 0.6 = 0.54
            assert!(
                req.confidence < 0.7,
                "expected confidence < 0.7 (no-section multiplier), got {}",
                req.confidence
            );
        }
    }

    #[test]
    fn filters_short_requirements() {
        let input = "\
## Requirements
- Rust
- Experience with Kubernetes and container orchestration";
        let result = parse_requirements(input);
        // "Rust" is only 4 chars, should be filtered
        assert_eq!(result.requirements.len(), 1);
        assert!(result.requirements[0].text.contains("Kubernetes"));
    }

    #[test]
    fn deduplicates_requirements() {
        let input = "\
## Requirements
- Strong experience with Python programming
- Strong experience with Python programming
- Deep knowledge of machine learning frameworks";
        let result = parse_requirements(input);
        assert_eq!(result.requirements.len(), 2);
    }

    #[test]
    fn handles_multiple_sections() {
        let input = "\
## Requirements
- 3+ years of backend development experience
## Qualifications
- Bachelor's degree in Computer Science or related field";
        let result = parse_requirements(input);
        assert_eq!(result.requirements.len(), 2);
        // Check sections are different
        let sections: Vec<Option<&str>> = result
            .requirements
            .iter()
            .map(|r| r.section.as_deref())
            .collect();
        assert!(sections.contains(&Some("Requirements")));
        assert!(sections.contains(&Some("Qualifications")));
    }

    #[test]
    fn overall_confidence_is_mean() {
        let input = "\
## Requirements
- Proficiency in TypeScript and Node.js ecosystems
- Experience with PostgreSQL and Redis databases";
        let result = parse_requirements(input);
        assert_eq!(result.requirements.len(), 2);
        let expected = result
            .requirements
            .iter()
            .map(|r| r.confidence)
            .sum::<f64>()
            / result.requirements.len() as f64;
        assert!(
            (result.overall_confidence - expected).abs() < f64::EPSILON,
            "overall_confidence {} != mean {}",
            result.overall_confidence,
            expected
        );
    }

    #[test]
    fn handles_prose_without_bullets() {
        let input = "\
## Requirements
Candidates must have strong communication skills. They should be able to work independently in a remote environment.";
        let result = parse_requirements(input);
        // Single line under section -> sentence split at ". T"
        assert!(
            result.requirements.len() >= 2,
            "expected >= 2 sentences, got {}",
            result.requirements.len()
        );
        for req in &result.requirements {
            assert!(
                (req.confidence - 0.4).abs() < f64::EPSILON,
                "prose confidence should be 0.4, got {}",
                req.confidence
            );
        }
    }

    #[test]
    fn stops_at_benefits_section() {
        let input = "\
## Requirements
- Expertise in Rust systems programming language
## Benefits
- Unlimited PTO and flexible schedule
- Company equity and stock options";
        let result = parse_requirements(input);
        assert_eq!(result.requirements.len(), 1);
        assert!(result.requirements[0].text.contains("Rust"));
    }
}
