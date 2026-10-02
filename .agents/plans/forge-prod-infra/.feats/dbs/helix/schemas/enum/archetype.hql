N::Archetype {
    name: String,
}

E::Includes {
  From: Archetype,
  To: Domain,
  Properties: {
    field1: String,
    field2: U32
  }
}
