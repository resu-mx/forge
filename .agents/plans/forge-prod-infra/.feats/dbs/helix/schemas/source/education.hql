V::SourceEducation {
    description: String,
    type: String, // degree, certificate, minor, course, self-taught
    posture: String, // On-Site, Remote, Hybrid
    field: String, // On-Site, Remote, Hybrid
    url: String, // ie for Courses
    degree_level: String, // On-Site, Remote, Hybrid
    degree_type: String, // On-Site, Remote, Hybrid
    certificate_subtype: String, // On-Site, Remote, Hybrid
    gpa: I8,
    in_progress: Boolean,
    start_date: Date,
    end_date: Date,
}

E::Contains {
  From: SourceEducation,
  To: Bullet
}

E::Contains {
  From: SourceEducation,
  To: Skill
}

E::TaughtBy UNIQUE {
  From: SourceEducation,
  To: Organization
}

E::ResultedIn {
  From: SourceEducation,
  To: Credential
}

E::ResultedIn {
  From: SourceEducation,
  To: Certification
}

E::InSupportOf {
  From: SourceEducation,
  To: Organization
}

E::HasNote {
  From: SourceEducation,
  To: Note,
  Properties: {
    updated_at: Date DEFAULT NOW
  }
}
