struct Resume {
    sections: Vec<ResumeSection>,
    credentials: Vec<Credentials>,
    summary: ResumeSummary,
    template: Option<ResumeTemplate::id>,
    title: String,
    display_name: String,
    skills: Vec<Skill>,
    archetype: Archetype,
    // (TODO) domains; not sure how/whether to implement these here
    // (TODO) industry; not sure how/whether to implement these here
    // (TODO) sectors; not sure how/whether to implement these here
}
