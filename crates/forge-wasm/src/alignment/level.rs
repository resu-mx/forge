//! Skill level enum + level-adjustment multiplier table.

use serde::{Deserialize, Serialize};

/// Ordinal skill level. `Junior < Mid < Senior < Staff < Principal`. The
/// numeric distance between levels is the input to the level-adjustment
/// multiplier (one rung gap, two rungs, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillLevel {
    Junior,
    Mid,
    Senior,
    Staff,
    Principal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelMultipliers {
    pub exceeds: f64,
    pub meets: f64,
    pub partial_one_rung: f64,
    pub partial_two_rungs: f64,
    pub partial_three_plus: f64,
    pub missing: f64,
}

impl Default for LevelMultipliers {
    fn default() -> Self {
        Self {
            exceeds: 1.0,
            meets: 1.0,
            partial_one_rung: 0.7,
            partial_two_rungs: 0.5,
            partial_three_plus: 0.3,
            missing: 0.8,
        }
    }
}

/// Compute the level-adjustment multiplier for a JD level requirement
/// given a resume's level (both optional).
pub fn compute_level_multiplier(
    resume: Option<SkillLevel>,
    jd: Option<SkillLevel>,
    m: &LevelMultipliers,
) -> f64 {
    match (resume, jd) {
        (_, None) => m.missing,
        (None, Some(_)) => m.missing,
        (Some(r), Some(j)) if r > j => m.exceeds,
        (Some(r), Some(j)) if r == j => m.meets,
        (Some(r), Some(j)) => {
            let gap = (j as u8) - (r as u8);
            match gap {
                1 => m.partial_one_rung,
                2 => m.partial_two_rungs,
                _ => m.partial_three_plus,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_junior_to_principal() {
        assert!(SkillLevel::Junior < SkillLevel::Mid);
        assert!(SkillLevel::Mid < SkillLevel::Senior);
        assert!(SkillLevel::Senior < SkillLevel::Staff);
        assert!(SkillLevel::Staff < SkillLevel::Principal);
    }

    #[test]
    fn defaults_match_spec() {
        let m = LevelMultipliers::default();
        assert_eq!(m.exceeds, 1.0);
        assert_eq!(m.meets, 1.0);
        assert_eq!(m.partial_one_rung, 0.7);
        assert_eq!(m.partial_two_rungs, 0.5);
        assert_eq!(m.partial_three_plus, 0.3);
        assert_eq!(m.missing, 0.8);
    }

    #[test]
    fn jd_missing_returns_missing_multiplier() {
        let m = LevelMultipliers::default();
        assert_eq!(
            compute_level_multiplier(Some(SkillLevel::Senior), None, &m),
            0.8
        );
    }

    #[test]
    fn resume_missing_returns_missing_multiplier() {
        let m = LevelMultipliers::default();
        assert_eq!(
            compute_level_multiplier(None, Some(SkillLevel::Senior), &m),
            0.8
        );
    }

    #[test]
    fn exceeds_returns_exceeds_when_resume_above_jd() {
        let m = LevelMultipliers::default();
        assert_eq!(
            compute_level_multiplier(Some(SkillLevel::Staff), Some(SkillLevel::Senior), &m),
            1.0
        );
    }

    #[test]
    fn meets_returns_meets_on_equal() {
        let m = LevelMultipliers::default();
        assert_eq!(
            compute_level_multiplier(Some(SkillLevel::Senior), Some(SkillLevel::Senior), &m),
            1.0
        );
    }

    #[test]
    fn partial_one_rung() {
        let m = LevelMultipliers::default();
        assert_eq!(
            compute_level_multiplier(Some(SkillLevel::Mid), Some(SkillLevel::Senior), &m),
            0.7
        );
    }

    #[test]
    fn partial_two_rungs() {
        let m = LevelMultipliers::default();
        assert_eq!(
            compute_level_multiplier(Some(SkillLevel::Junior), Some(SkillLevel::Senior), &m),
            0.5
        );
    }

    #[test]
    fn partial_three_plus() {
        let m = LevelMultipliers::default();
        assert_eq!(
            compute_level_multiplier(Some(SkillLevel::Junior), Some(SkillLevel::Staff), &m),
            0.3
        );
        assert_eq!(
            compute_level_multiplier(Some(SkillLevel::Junior), Some(SkillLevel::Principal), &m),
            0.3
        );
    }
}
