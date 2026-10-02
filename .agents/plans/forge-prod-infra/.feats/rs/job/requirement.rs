enum Requirements {
    HardRequirement,
    SoftRequirement,
    Preference,
    StrongPreference,
    BonusPoints,
}

trait Requirement {
    fn preferred() -> bool;
    fn required() -> bool;
    fn get_skills() -> Vec<&Skill>;
}

impl Requirement for HardRequirement {}
impl Requirement for SoftRequirement {}
impl Requirement for Preference {}
impl Requirement for StrongPreference {}
impl Requirement for BonusPoints {}
