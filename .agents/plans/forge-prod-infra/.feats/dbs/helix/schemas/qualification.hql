N::Qualification {
    title: String,
    description: String
}

N::Certification {
    title: String,
    field2: U32
}

N::Credential {
    title: String,
    field2: U32
}

E::Requires UNIQUE {
    From: Qualification,
    To: Certification
}

E::Requires UNIQUE {
    From: Qualification,
    To: Credential
}

// Can be fulfilled by more than one thing
E::FulfilledBy{
    From: Qualification,
    To: Certification
}

E::FulfilledBy{
    From: Qualification,
    To: Credential
}
