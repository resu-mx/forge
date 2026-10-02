struct User {
    name: FullName,
    address: Address,
    contact: ContactCard,
}

// (Legal, Preferred, Display); might swap display for some other encoding
type NameTriple = (String, String, bool, bool);

struct FullName {
    first: NameTriple,
    middle: Option<Vec<NameTriple>>,
    last: NameTriple,
    prefix: Option<NamePrefix>,
    suffix: Option<NameSuffix>,
    pronouns: Option<Vec<ProNoun>>,
    nickname: Option<String>,
}
