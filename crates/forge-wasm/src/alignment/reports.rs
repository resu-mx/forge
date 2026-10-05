//! Gap / strength / coverage report assembly.

use super::config::AlignmentConfig;
use super::engine::{JdAlignmentInput, ResumeAlignmentInput};
use super::result::{
    CoverageReport, GapEntry, GapReport, MatchSummary, SkillScore, StrengthEntry, StrengthReport,
};

/// Build a gap report listing JD skills with score below `gap_threshold`.
pub fn build_gap(
    per_skill: &[SkillScore],
    jd: &JdAlignmentInput,
    config: &AlignmentConfig,
) -> GapReport {
    let entries = per_skill
        .iter()
        .filter(|s| s.score < config.gap_threshold)
        .map(|s| {
            let req_level = jd
                .skills
                .iter()
                .find(|j| j.skill_id == s.skill_id)
                .and_then(|j| j.required_level);
            GapEntry {
                required_skill_id: s.skill_id.clone(),
                severity: config.gap_threshold - s.score,
                best_match: if s.raw_score > 0.0 {
                    Some(MatchSummary {
                        resume_skill_id: s.skill_id.clone(),
                        match_type: s.top_match_type,
                        score: s.raw_score,
                    })
                } else {
                    None
                },
                required_level: req_level,
            }
        })
        .collect();
    GapReport { entries }
}

/// Build a strength report — resume skills with NO JD coverage (transferable surplus).
pub fn build_strength(resume: &ResumeAlignmentInput, per_skill: &[SkillScore]) -> StrengthReport {
    let scored: std::collections::HashSet<&str> =
        per_skill.iter().map(|s| s.skill_id.as_str()).collect();
    let entries = resume
        .skills
        .iter()
        .filter(|s| !scored.contains(s.skill_id.as_str()))
        .map(|s| StrengthEntry {
            skill_id: s.skill_id.clone(),
            evidence_count: s.evidence.len() as u32,
            top_match_score: 0.0,
        })
        .collect();
    StrengthReport { entries }
}

/// Build a coverage report — bucket JD skills by score tier.
pub fn build_coverage(per_skill: &[SkillScore], config: &AlignmentConfig) -> CoverageReport {
    let mut strong = 0;
    let mut moderate = 0;
    let mut weak = 0;
    let mut gap = 0;
    for s in per_skill {
        if s.score > config.strong_threshold {
            strong += 1;
        } else if s.score >= config.gap_threshold {
            moderate += 1;
        } else if s.score > 0.0 {
            weak += 1;
        } else {
            gap += 1;
        }
    }
    let total = per_skill.len() as u32;
    let coverage_pct = if total > 0 {
        (strong + moderate) as f64 / total as f64
    } else {
        0.0
    };
    CoverageReport {
        strong,
        moderate,
        weak,
        gap,
        total_required: total,
        coverage_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::engine::{JdSkillRef, ResumeSkillRef};
    use crate::alignment::match_types::MatchType;

    fn score(id: &str, score: f64) -> SkillScore {
        SkillScore {
            skill_id: id.into(),
            score,
            raw_score: score,
            level_multiplier: 1.0,
            top_match_type: MatchType::Direct,
        }
    }

    #[test]
    fn coverage_buckets_per_threshold() {
        let config = AlignmentConfig::default();
        let scores = vec![
            score("a", 0.9), // strong (>0.8)
            score("b", 0.5), // moderate (≥0.4)
            score("c", 0.2), // weak (>0 and <0.4)
            score("d", 0.0), // gap
        ];
        let r = build_coverage(&scores, &config);
        assert_eq!(r.strong, 1);
        assert_eq!(r.moderate, 1);
        assert_eq!(r.weak, 1);
        assert_eq!(r.gap, 1);
        assert_eq!(r.total_required, 4);
        assert!((r.coverage_pct - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gap_report_lists_only_below_threshold() {
        let config = AlignmentConfig::default();
        let scores = vec![score("a", 0.9), score("b", 0.2)];
        let jd = JdAlignmentInput {
            jd_id: "jd".into(),
            skills: vec![
                JdSkillRef {
                    skill_id: "a".into(),
                    required_level: None,
                    embedding: None,
                },
                JdSkillRef {
                    skill_id: "b".into(),
                    required_level: None,
                    embedding: None,
                },
            ],
        };
        let r = build_gap(&scores, &jd, &config);
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.entries[0].required_skill_id, "b");
    }

    #[test]
    fn strength_report_contains_resume_skills_not_in_jd() {
        let resume = ResumeAlignmentInput {
            resume_id: "r1".into(),
            skills: vec![
                ResumeSkillRef {
                    skill_id: "extra".into(),
                    level: None,
                    evidence: vec![],
                },
                ResumeSkillRef {
                    skill_id: "matched".into(),
                    level: None,
                    evidence: vec![],
                },
            ],
            validated_skill_ids: vec![],
        };
        let scores = vec![score("matched", 1.0)];
        let r = build_strength(&resume, &scores);
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.entries[0].skill_id, "extra");
    }

    #[test]
    fn coverage_pct_zero_when_no_skills() {
        let config = AlignmentConfig::default();
        let r = build_coverage(&[], &config);
        assert_eq!(r.total_required, 0);
        assert_eq!(r.coverage_pct, 0.0);
    }
}
