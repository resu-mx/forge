N::Resume {
    field1: String,
    field2: U32
}

E::Shows {
  From: Resume,
  To: Skill,
  Properties: {
    field1: String,
    field2: U32
  }
}

E::Include {
  From: Resume,
  To: Section,
  Properties: {
    field1: String,
    field2: U32
  }
}
