V::SourcePresentation {
    description: String,
    kind: String, // internal, conference, blogpost, etc ...
    is_current: Boolean,
    start_date: Date,
    end_date: Date,
}

E::Contains {
  From: SourcePresentation,
  To: Bullet
}

E::Contains {
  From: SourcePresentation,
  To: Skill
}

E::HasNote {
  From: SourcePresentation,
  To: Note,
  Properties: {
    updated_at: Date DEFAULT NOW
  }
}

E::InSupportOf {
  From: SourcePresentation,
  To: Organization,
  Properties: {
    updated_at: Date DEFAULT NOW
  }
}
