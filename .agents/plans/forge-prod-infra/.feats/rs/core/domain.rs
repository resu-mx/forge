type Archetype = Vec<&Domain>;

struct Domain {
    name: DomainType,
    description: U32,
}

// Q: Should 'Domain' be runtime Change-able?
enum DomainType {
    Foo,
    Bar,
}
