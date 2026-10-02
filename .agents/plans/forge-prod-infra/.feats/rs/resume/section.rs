enum ResumeSectionKind {
    Experience(ExperienceKind), // [{Org,Meta,{Title,Date,Meta,[Bullet]}}]
    Skills,                     // [{Key:[Values]}]
    Education,                  // [{Org,Date,Lvl,Name,Campus}]
    Certifications,             // [{Key:[Values]}]
    Projects,                   // [{Title,Name,[Bullet]}]
    Residencies,
    Fellowships,
    Honors,
    Awards,
    Publications,
    Presentations,
}

// (TODO) Q: Does Academic look the same as Professional?
enum ExperienceKind {
    Professional,
    Academic,
}

trait ResumeSection {
    fn get_children(self) -> Result<Vec<SectionContainerChildren>, ResumeError>{
        self.children {
            Some(SectionContainer) -> None
            Some(Bullet) -> None
            None -> None
        }
    };
    fn pop_child(self) -> Result<()>;
    fn push_child(self) -> Result<()>;
    fn insert_child_at(self) -> Result<()>;
    fn remove_child_at(self) -> Result<()>;
    fn swap_children(self, a: u8, b: u8) -> Result<()>;
}

impl ResumeSection for SectionContainer {}

// This covers 'Bullets' && 'SubSections'
struct SectionContainer {
    title: (ResumeSectionKind, String),
    display_name: String,
    children: Option<Vec<SectionContainerChildren>>, // Compare HashMap<i, SectionContainerChildren> vs Vec<SectionContainerChildren> for reordering efficiency
}

enum SectionContainerChildren {
    SectionContainer,
    Bullet,
}
