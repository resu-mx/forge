V::SourceRole {
    description: String,
    posture: String, // On-Site, Remote, Hybrid
    is_current: Boolean,
    start_date: Date,
    end_date: Date,
    //base_salary
    //total_comp_notes
}

E::Contains {
  From: SourceRole,
  To: Bullet
}

E::Contains {
  From: SourceRole,
  To: Skill
}

E::DescribedBy UNIQUE {
  From: Source,
  To: SourceRole,
  Properties: {
    updated_at: Date DEFAULT NOW
  }
}

E::WorkedFor UNIQUE {
  From: SourceRole,
  To: Organization
}

E::EmployedBy UNIQUE {
  From: SourceRole,
  To: Organization
}

E::HasNote {
  From: SourceRole,
  To: Note,
  Properties: {
    updated_at: Date DEFAULT NOW
  }
}
