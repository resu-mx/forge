struct Note {
    title: String,
    content: Markdown,
}

struct Markdown {
    headers: Vec<(u8, String)>,  // TODO
    lists: Vec<&MarkdownList>,   // TODO
    paragraphs: Vec<&String>,    // TODO
    comments: Vec<&String>,      // TODO
    codeblocks: Vec<&CodeBlock>, // TODO
    quotes: Vec<(u8, String)>,   // TODO
                                 // (future?) callouts: Vec<x>,
                                 // (future?) html: Vec<x>,
                                 // (future?) jsx: Vec<x>,
}

enum MarkdownList {
    // Q: Should I capture 'indentation' here?
    Ordered(Vec<(u16, String)>), // Q: Do I need to specify the 'kind' of ordered here? (ie 'a)'v'1.'v'1a', etc)
    Unordered(Vec<String>),
}
