//! The community layers' declared relations (DESIGN §9.4; ADR-0011), stated before any kernel
//! runs; their digest joins the compiler digest and each Leiden invocation's record.
//!
//! - **Invocation layer.** The invocation projection's arcs (`projection::invocation`) between
//!   two subsystem functions: the kernel counts them per unordered pair.
//! - **Co-use layer.** Each call in the official usage code (examples, tests, doc blocks) to a
//!   function, with its enclosing scope (the usage function or module): the kernel counts, per
//!   unordered pair of subsystem functions, the scopes that call both. The call targets are the
//!   flows' (`flows::call_targets`: the projection's accepted evidence, phase `call` or `init`).
//! - **Public callables.** What a community reports: each function exported under a public root
//!   by a path with no private segment, and each public method (or `__init__`, `__call__`) an
//!   exported class declares or inherits along its MRO, the nearest definition winning; each with
//!   every access path. An `@overload` stub is not a callable of its own.
//! - **The preferred path** (the increment-2 review's F4), the one name every consumer shows a
//!   callable by: a path through the class that declares the method first (so a classmethod is
//!   never named through a subclass it would bind differently), then the fewest segments, then
//!   the least. Exactly one path per callable is `preferred`.

use crate::codebook::{AncestryRelation, Codebook, DeclarationKind, EdgeKind, SourceRole};
use crate::flows::{call_targets, codes};
use crate::id::{Digest, IdHasher};

/// The co-use occurrences, ordered by `(scope, target, call site)`.
pub fn co_use_sql() -> String {
    format!(
        "SELECT DISTINCT ec.src_node_id AS scope_node_id, t.target_node_id, \
                t.call_site_node_id \
         FROM ({targets}) t \
         JOIN edges ec ON ec.dst_node_id = t.call_site_node_id AND ec.edge_kind = {encloses} \
         JOIN call_syntax cs ON cs.node_id = t.call_site_node_id \
         JOIN source_files u ON u.module_node_id = cs.module_node_id \
         WHERE u.role IN ({usage}) \
         ORDER BY scope_node_id, target_node_id, call_site_node_id",
        targets = call_targets(),
        encloses = EdgeKind::EnclosesCall.code(),
        usage = codes(&[SourceRole::Example, SourceRole::Test, SourceRole::DocBlock]),
    )
}

fn quoted(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("'{}'", n.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The public callables under `public_roots` with every access path and which one is preferred,
/// ordered by node and path.
pub fn public_callables_sql(public_roots: &[String]) -> String {
    let functions = codes(&[DeclarationKind::Function, DeclarationKind::AsyncFunction]);
    format!(
        "WITH exported AS ( \
           SELECT DISTINCT declaration_node_id AS node_id, access_path FROM exports \
           WHERE split_part(access_path, '.', 1) IN ({roots}) \
             AND strpos(access_path, '._') = 0 AND NOT starts_with(access_path, '_')), \
         direct AS ( \
           SELECT d.node_id, x.access_path, true AS own FROM declarations d \
           JOIN exported x ON x.node_id = d.node_id \
           WHERE d.kind IN ({functions}) AND NOT d.is_overload), \
         classes AS ( \
           SELECT x.node_id AS class_node_id, x.access_path FROM exported x \
           JOIN declarations c ON c.node_id = x.node_id AND c.kind = {class}), \
         lineage AS ( \
           SELECT class_node_id, class_node_id AS ancestor_node_id, -1 AS ordinal FROM classes \
           UNION ALL \
           SELECT c.class_node_id, t.ancestor_node_id, a.ordinal FROM classes c \
           JOIN ancestry_targets t ON t.class_node_id = c.class_node_id \
           JOIN class_ancestry a ON a.fact_id = t.ancestry_fact_id AND a.relation = {mro} \
           WHERE t.ancestor_node_id IS NOT NULL), \
         candidates AS ( \
           SELECT l.class_node_id, d.node_id, d.name, d.kind, l.ordinal FROM lineage l \
           JOIN declarations d ON d.parent_node_id = l.ancestor_node_id), \
         nearest AS ( \
           SELECT class_node_id, name, min(ordinal) AS ordinal FROM candidates \
           GROUP BY class_node_id, name), \
         methods AS ( \
           SELECT c.node_id, x.access_path || '.' || c.name AS access_path, \
                  c.ordinal = -1 AS own FROM candidates c \
           JOIN nearest n ON n.class_node_id = c.class_node_id AND n.name = c.name \
             AND n.ordinal = c.ordinal \
           JOIN classes x ON x.class_node_id = c.class_node_id \
           JOIN declarations dd ON dd.node_id = c.node_id \
           WHERE c.kind IN ({functions}) AND NOT dd.is_overload \
             AND (NOT starts_with(c.name, '_') OR c.name IN ('__init__', '__call__'))), \
         paths AS ( \
           SELECT node_id, access_path, bool_or(own) AS own \
           FROM (SELECT * FROM direct UNION ALL SELECT * FROM methods) \
           GROUP BY node_id, access_path), \
         ranked AS ( \
           SELECT node_id, access_path, row_number() OVER ( \
             PARTITION BY node_id ORDER BY own DESC, \
               length(access_path) - length(replace(access_path, '.', '')), access_path) AS pick \
           FROM paths) \
         SELECT node_id, access_path, pick = 1 AS preferred FROM ranked \
         ORDER BY node_id, access_path",
        roots = quoted(public_roots),
        class = DeclarationKind::Class.code(),
        mro = AncestryRelation::Mro.code(),
    )
}

/// The relations' identity: theirs and the invocation projection's, whose arcs the invocation
/// layer counts (ADR-0011 review F3). The public-callables query with no roots stands for its
/// form.
pub fn digest() -> Digest {
    digest_with(crate::projection::invocation().digest())
}

/// [`digest`] over a given invocation projection's digest.
pub fn digest_with(invocation: Digest) -> Digest {
    IdHasher::new("community-relations")
        .str(&co_use_sql())
        .str(&public_callables_sql(&[]))
        .str(&invocation.hex())
        .finish_digest()
}

/// The declared output schemas.
pub mod schemas {
    use std::sync::Arc;

    use arrow_schema::{DataType, Field, Schema, SchemaRef};

    fn id(name: &str) -> Field {
        Field::new(name, DataType::FixedSizeBinary(16), false)
    }

    pub fn co_use() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("scope_node_id"),
            id("target_node_id"),
            id("call_site_node_id"),
        ]))
    }

    pub fn public_callables() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("node_id"),
            Field::new("access_path", DataType::Utf8, false),
            Field::new("preferred", DataType::Boolean, false),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::digest_with;
    use crate::id::content_digest;

    /// A change to the invocation projection moves the community relations' digest, and so each
    /// community invocation's `projection_digest`.
    #[test]
    fn the_digest_follows_the_invocation_projection() {
        assert_ne!(
            digest_with(content_digest(b"one projection")),
            digest_with(content_digest(b"another"))
        );
    }
}
