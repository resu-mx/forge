//! Services that work across entities: compiling a resume, exporting, auditing, reviewing,
//! checking integrity, and generating a tagline.
//!
//! There is no per-entity service layer. The stores in [`crate::db::stores`] are it: routes
//! call them directly, and each store enforces its entity's validation and business rules.

pub mod audit_service;
pub mod compiler_service;
pub mod export_service;
pub mod integrity_service;
pub mod review_service;
pub mod tagline;
pub mod typst_render;

pub use audit_service::AuditService;
pub use compiler_service::CompilerService;
pub use export_service::ExportService;
pub use integrity_service::{DriftedEntity, DriftedEntityType, IntegrityService};
pub use review_service::ReviewService;
pub use typst_render::render_typst;
