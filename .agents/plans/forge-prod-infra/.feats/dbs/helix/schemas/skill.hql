N::Skill {
    UNIQUE INDEX title: String,
    description: String,
    category: String
}

E::Category {
    From: Skill,
    To: Category,
}
