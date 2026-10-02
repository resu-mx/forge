N::Organization {
    UNIQUE INDEX title: String,
    about: String
}

N::Campus {
    UNIQUE INDEX title: String,
    about: String
}

E::LocatedAt {
    From: Campus,
    To: UsAddress
}

E::Published {
  From: Organization,
  To: JD,
  Properties: {
    created: Date DEFAULT NOW,
    updated: Date DEFAULT NOW
  }
}

E::HasNote {
  From: Organization,
  To: Note,
  Properties: {
    verified: Date DEFAULT NOW
  }
}

E::Has {
  From: Organization,
  To: Benefits,
  Properties: {
    verified: Date DEFAULT NOW
  }
}

N::Benefits {
    field1: String,
    field2: U32
}
