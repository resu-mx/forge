N::User {
    field1: String,
    field2: U32
}

E::Has {
  From: User,
  To: Contact,
  Properties: {
    field1: String,
    field2: U32
  }
}

E::LocatedAt {
    From: User,
    To: UsAddress
}
