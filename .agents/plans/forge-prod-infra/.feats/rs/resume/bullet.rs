struct Bullet {
    title: String,
    content: String,
    status: BulletStatus,
    type: BulletType,
    priority: U8, // use for ordering in lists
    skills: Vec<SkillId>,
    notes: Vec<NoteId>,
    overriden: bool
}

// ENUM Nodes
struct BulletType {
    name: BulletTypeVariant,
    priority: U8 // use for ordering in lists
}

enum BulletTypeVariant {
    Project,
    Role,
    Education,
    Core,
    Perspective,
    Override
}

struct BulletStatus {
    name: BulletStatusVariant,
    priority: U8, // use for ordering in lists
}

enum BulletStatusVariant {
    Draft,
    Review,
    Approved,
    Rejected,
    Archived
}
