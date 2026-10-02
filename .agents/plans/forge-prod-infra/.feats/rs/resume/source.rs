enum SourceKind {
    Education,
    Presentation,
    Project,
    Role,
}

enum SourceStatus {
    Education,
    Presentation,
    Project,
    Role,
}

trait Source {
    fn get_bullets() -> Vec<Bullet>;
    fn get_skills() -> Vec<Skill>;
    fn get_notes() -> Vec<Skill>;
    fn get_status() -> Option<SourceStatus>;
    fn get_date_range() -> (Date, Date);
}

impl Source for Education {};
impl Source for Presentation {};
impl Source for Project {};
impl Source for Role {};

struct Education {
    description: String,
    type: String, // degree, certificate, minor, course, self-taught
    posture: String, // On-Site, Remote, Hybrid
    field: String, // On-Site, Remote, Hybrid
    url: String, // ie for Courses
    degree_level: String, // On-Site, Remote, Hybrid
    degree_type: String, // On-Site, Remote, Hybrid
    certificate_subtype: String, // On-Site, Remote, Hybrid
    gpa: i8,
    in_progress: bool,
    start_date: Date,
    end_date: Date,
    results: Option<Vec<Qualification>>,
    notes: Option<Vec<&Note>>,
    bullets: Option<Vec<Bullet>>,
    skills: Option<Vec<Skill>>
}

struct Presentation {
    description: String,
    kind: String, // internal, conference, blogpost, etc ...
    is_current: Boolean,
    start_date: Date,
    end_date: Date,
    url: Url,
    org: Organization,
    notes: Option<Vec<&Note>>,
    bullets: Option<Vec<Bullet>>,
    skills: Option<Vec<Skill>>
};

struct Role {
    description: String,
    posture: String, // On-Site, Remote, Hybrid
    is_current: Boolean,
    start_date: Date,
    end_date: Date,
    org: Organization,
    notes: Option<Vec<&Note>>,
    bullets: Option<Vec<Bullet>>,
    skills: Option<Vec<Skill>>,
    //base_salary
    //total_comp_notes
}

struct Project {
    description: String,
    posture: String, // On-Site, Remote, Hybrid
    contribution: Boolean,
    open_source: Boolean,
    url: String,
    start_date: Date,
    end_date: Date,
    status: ProjectStatus,
    org: Organization,
    notes: Option<Vec<&Note>>,
    bullets: Option<Vec<Bullet>>,
    skills: Option<Vec<Skill>>,
}
