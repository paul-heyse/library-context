//! The model's one registry (DESIGN §15.2).
//!
//! Every canonical relation is listed here exactly once; `RelationId`, the declaration list, the
//! generated DDL and every inventory derive from it. Relations are added in the cutover phase that
//! lands their producer (cutover plan §4), so the registry is empty until phase 2.

crate::model! {}
