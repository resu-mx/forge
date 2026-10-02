V::SourceProject {
    description: String,
    posture: String, // On-Site, Remote, Hybrid
    contribution: Boolean,
    open_source: Boolean,
    url: String,
    start_date: Date,
    end_date: Date,
}

N::SourceProjectStatus {
    UNIQUE INDEX type: String
}

E::Contains {
  From: SourceProject,
  To: Bullet
}

E::Contains {
  From: SourceProject,
  To: Skill
}

E::StatusIs UNIQUE {
  From: SourceProject,
  To: SourceProjectStatus,
  Properties: {
    updated_at: Date DEFAULT NOW
  }
}

E::HasNote {
  From: SourceProject,
  To: Note,
  Properties: {
    updated_at: Date DEFAULT NOW
  }
}

E::InSupportOf {
  From: SourceProject,
  To: SourceRole
}
