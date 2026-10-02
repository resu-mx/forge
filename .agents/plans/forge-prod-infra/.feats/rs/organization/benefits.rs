enum Benefit {
    Leave,
    HealthCare,
    Stocks, // V Are these different enough to warran seperate variants?
    Equity, // ^ Are these different enough to warran seperate variants?
}

struct HealthCarePlan {
    medical: String,
    dental: String,
    vision: String,
    ivf: String,
}

struct Leave {
    pto: PTOKind,
    holidays: Vec<Holiday>, // floating, company, federal, etc
    sick: u8,
}

enum PTOKind {
    Unlimited(UnlimitedPTO),
    Accrued(AccruedPTO),
}

// NOTE: Need to think about this one more from a UserStory perspective
struct UnlimitedPTO {
    unlimited: bool,
    reviews: Vec<String>,
}

struct AccruedPTO {
    accrural_rate: f32,
    starting: f64,
    limit: f64,
    rollover: bool,
}
