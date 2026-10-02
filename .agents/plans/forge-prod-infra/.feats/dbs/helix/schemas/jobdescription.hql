N::JD {
    field1: String,
    field2: U32
}

E::Has {
  From: JD,
  To: Requirement
}

E::Has {
  From: JD,
  To: Qualification
}

E::Targets UNIQUE {
  From: JD,
  To: Salary
}
