//! The community layers' declared relations (DESIGN §9.4; ADR-0011), stated before any kernel
//! runs; their digest joins the compiler digest and each Leiden invocation's record.
//!
//! - **Invocation layer.** The invocation projection's arcs (`projection::invocation`) between
//!   two subsystem functions: the kernel counts them per unordered pair.
//! - **Co-use layer.** Each call in the official usage code (examples, tests, doc blocks) to a
//!   function, with its enclosing scope (the usage function or module): the kernel counts, per
//!   unordered pair of subsystem functions, the scopes that call both. The call targets are the
//!   flows' (`flows::call_targets`: the projection's accepted evidence, phase `call` or `init`).
//! - **Public callables.** What a community reports: each public callable of `public_paths`
//!   (`cpg_schema::public`, the one public-path authority), shown by its preferred path.

use crate::codebook::{Codebook, EdgeKind, MentionClass, SourceRole, TypeRole};
use crate::flows::{call_targets, codes, receivers_sql};
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

/// The type layer's relation (slice 3.2, the `+type-layer` variant): each function with each
/// release class its declared parameter types name anywhere in their structure (the receiver
/// aside), ordered by `(function, class)`. Two functions taking one class are a pair, 1 per class.
pub fn shared_types_sql() -> String {
    format!(
        "WITH RECURSIVE receivers AS ({receivers}), \
         declared AS ( \
           SELECT ps.function_node_id, o.term_node_id FROM parameter_syntax ps \
           JOIN type_observations o ON o.subject_node_id = ps.node_id \
             AND o.role = {parameter} AND o.declared \
           LEFT ANTI JOIN receivers r ON r.parameter_node_id = ps.node_id), \
         reach(function_node_id, term_node_id) AS ( \
           SELECT function_node_id, term_node_id FROM declared \
           UNION ALL \
           SELECT r.function_node_id, a.child_node_id FROM reach r \
           JOIN type_term_args a ON a.parent_node_id = r.term_node_id) \
         SELECT DISTINCT r.function_node_id AS target_node_id, \
                t.class_node_id AS scope_node_id \
         FROM reach r JOIN type_class_targets t ON t.term_node_id = r.term_node_id \
         JOIN declarations c ON c.node_id = t.class_node_id \
         ORDER BY scope_node_id, target_node_id",
        receivers = receivers_sql(),
        parameter = TypeRole::Parameter.code(),
    )
}

/// The mention layer's relation (slice 3.2, the `+mention-layer` variant; C5 O1): each doc
/// passage with each declaration an exact mention in it names, ordered by `(passage, target)`.
/// Two functions one passage names are a pair, 1 per passage.
pub fn co_mention_sql() -> String {
    format!(
        "SELECT DISTINCT t.passage_node_id AS scope_node_id, t.target_node_id \
         FROM mention_targets t JOIN mentions m ON m.fact_id = t.mention_fact_id \
         WHERE m.class = {exact} AND t.target_node_id IS NOT NULL \
         ORDER BY scope_node_id, target_node_id",
        exact = MentionClass::Exact.code(),
    )
}

/// The extra layers' identity, joined to the community relations' when a variant enables them.
pub fn extra_digest(base: Digest, layers: &[&str]) -> Digest {
    let mut h = IdHasher::new("community-extra-layers");
    h.digest_field(base);
    for layer in layers {
        h.str(layer);
        match *layer {
            "type" => h.str(&shared_types_sql()),
            "mention" => h.str(&co_mention_sql()),
            _ => &mut h,
        };
    }
    h.finish_digest()
}

/// The relations' identity: theirs and the invocation projection's, whose arcs the invocation
/// layer counts (ADR-0011 review F3), and the public paths' (`cpg_schema::public`), which name
/// what a community reports.
pub fn digest() -> Digest {
    digest_with(crate::projection::invocation().digest())
}

/// [`digest`] over a given invocation projection's digest.
pub fn digest_with(invocation: Digest) -> Digest {
    IdHasher::new("community-relations")
        .str(&co_use_sql())
        .str(&crate::public::public_paths().sql)
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

    /// The extra layers' `(scope, target)` relations (slice 3.2).
    pub fn scope_targets() -> SchemaRef {
        Arc::new(Schema::new(vec![id("scope_node_id"), id("target_node_id")]))
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
