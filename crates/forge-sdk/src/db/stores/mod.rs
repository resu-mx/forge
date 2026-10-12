//! One store per entity. The stores are the service layer of `forge-sdk`.
//!
//! Routes call them directly. Each store enforces its entity's validation and business rules,
//! in both runtimes, so a caller never repeats them. The TypeScript services in
//! `packages/core/src/services/` remain the reference for those rules. A rule a store does not
//! enforce yet is marked `TODO(resu-mx/forge#240)` on the method that should carry it, so
//! `grep -rn 'TODO(resu-mx/forge#240)' crates/forge-sdk/src/db/stores` lists them.
//!
//! Work that spans entities (compiling, exporting, auditing) lives in [`crate::services`].

pub mod address;
pub mod answer_bank;
pub mod archetype;
pub mod bullet;
pub mod campus;
pub mod certification;
pub mod contact;
pub mod credential;
pub mod derivation;
pub mod domain;
pub mod extension;
pub mod industry;
pub mod jd;
pub mod jd_resume;
pub mod lookup;
pub mod note;
pub mod organization;
pub mod perspective;
pub mod profile;
pub mod resume;
pub mod role_type;
pub mod skill;
pub mod skill_graph;
pub mod source;
pub mod summary;
pub mod template;

pub use address::AddressStore;
pub use answer_bank::AnswerBankStore;
pub use archetype::ArchetypeStore;
pub use bullet::BulletStore;
pub use campus::CampusStore;
pub use certification::CertificationStore;
pub use contact::ContactStore;
pub use credential::CredentialStore;
pub use derivation::DerivationStore;
pub use domain::DomainStore;
pub use extension::{ExtensionConfigStore, ExtensionLogStore};
pub use industry::IndustryStore;
pub use jd::{parse_status_filter, JdStore};
pub use jd_resume::JdResumeStore;
pub use note::NoteStore;
pub use organization::OrganizationStore;
pub use perspective::PerspectiveStore;
pub use profile::ProfileStore;
pub use resume::ResumeStore;
pub use role_type::RoleTypeStore;
pub use skill::SkillStore;
pub use skill_graph::SqlSkillGraphStore;
pub use source::SourceStore;
pub use summary::SummaryStore;
pub use template::TemplateStore;
