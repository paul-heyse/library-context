//! Conditions (DESIGN §15.7, §3.9; ADR-0082 carrying ADR-0024's kernel clauses).
//!
//! The canonical form of a condition is a reduced ordered BDD over **atoms** keyed by their
//! evaluation occurrence ([`atom`]), persisted as a condition, its content-addressed nodes and
//! its atoms ([`kernel`]). Every operation is preflighted against work and node budgets; a budget
//! hit is a [`KernelBoundary`](kernel::KernelBoundary), which callers record as a
//! `budget_reached` obligation, never as `false`. A rendering is presentation only: a bounded
//! DNF read from capped satisfying paths with a truncation marker. There is no parallel DNF.
//! [`theory`] is the typed primitive theory: exact-input assignments under a closed whitelist.

pub mod atom;
pub mod kernel;
mod substitution;
pub mod theory;

pub use atom::{Atom, Predicate, PredicateKind, Value};
pub use kernel::{CondExpr, Diagram, KernelBoundary};

#[cfg(test)]
mod tests;
