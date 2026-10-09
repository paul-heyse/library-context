//! Coherent topic controls; common drivers compile once in this target.
#[path = "typed_driver/mod.rs"]
mod typed_driver;

#[path = "cases/synthesis_documentary_templates.rs"]
mod synthesis_documentary_templates;
#[path = "cases/typed_deployment.rs"]
mod typed_deployment;
#[path = "cases/typed_documents.rs"]
mod typed_documents;
