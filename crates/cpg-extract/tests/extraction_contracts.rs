//! Coherent topic controls; common drivers compile once in this target.
#[path = "typed_driver/mod.rs"]
mod typed_driver;

#[path = "cases/attachment_oracle.rs"]
mod attachment_oracle;
#[path = "cases/determinism.rs"]
mod determinism;
#[path = "cases/harness.rs"]
mod harness;
#[path = "cases/python_reference_oracle.rs"]
mod python_reference_oracle;
#[path = "cases/typed_conformance.rs"]
mod typed_conformance;
