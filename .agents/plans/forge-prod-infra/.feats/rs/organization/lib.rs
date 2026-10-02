struct Organization {
    title: String,
    types: Vec<String>, // Thought: Mark 'Primary Type' w/ UNIQUE Edge?
    industry: String,
    about: String
}

type Industry = <Sector, String>;

enum Sector {
    Public,
    Private,
    Financial,
    Tech,
    Healthcare,
}

struct Campus {
    title: String,
    address: Address, // Q: how to say, 'type that implements "Address" trait'?
    about: String
}
