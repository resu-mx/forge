N::Source {
    type: String,
    UNIQUE INDEX title: String,
    display_name: String,
    start: Date DEFAULT NOW,
    end: Date DEFAULT NOW
    // derived_at
    // updated_at
}

N::SourceStatus {
    UNIQUE INDEX type: String
}

E::DescribedBy UNIQUE {
  From: Source,
  To: SourceProject
}

E::DescribedBy UNIQUE {
  From: Source,
  To: SourceRole
}

E::DescribedBy UNIQUE {
  From: Source,
  To: SourceEducation
}

E::DescribedBy UNIQUE {
  From: Source,
  To: SourcePresentation
}
