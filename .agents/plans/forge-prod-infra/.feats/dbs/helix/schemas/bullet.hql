N::Bullet {
    title: String,
    content: String,
    status: String,
    type: String,
    priority: U8 // use for ordering in lists
}

// ENUM Nodes
N::BulletType {
    name: String,
    priority: U8 // use for ordering in lists
}

N::BulletStatus {
    name: String,
    priority: U8 // use for ordering in lists
}

// === === === === === === === === === === ===
//  Edges
// === === === === === === === === === === ===

E::HasStatus UNIQUE {
  From: Bullet,
  To: BulletStatus,
  Properties: {
    updated: Date DEFAULT NOW,
    field2: U32
  }
}

E::Displays {
  From: Bullet,
  To: Skill
}

// === === === === === === === === === === ===
//  Vectors
// === === === === === === === === === === ===

V::Bullet {
  field1: String
}
