struct Contact {
    name: String,
    title: String,
    email: String,
    phone: String,
    urls: HashMap<String, Url>,
    notes: Vec<NoteId>,
}
