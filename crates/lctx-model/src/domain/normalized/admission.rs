//! Closed physical routes for necessary normalized owner predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope { Callables, Receivers, Events, Bindings }
