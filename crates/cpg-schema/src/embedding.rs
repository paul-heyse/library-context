//! The global embedding cache (DESIGN §3.2, §11.1; ADR-0010, ADR-0017 amendment).
//!
//! Vectors are keyed by `(spec_hash, input_hash)`: the embedding spec's SHA-256 and the SHA-256
//! of the exact request text. The table is **global**: it has no `snapshot_id`, it accumulates
//! across attempts and libraries, and a snapshot reads it at a recorded version over all its
//! files, never by commit (its read mode). It is written by an insert-only MERGE, so two attempts
//! can never store two vectors for one key.

use crate::id::Digest;
use crate::table::table;

table!(
    /// One cached vector: the spec and request text that produced it, and the served model.
    EmbeddingCache, EmbeddingCacheRow = "embedding_cache",
    family = EmbeddingCache,
    key = [spec_hash, input_hash],
    checks = [],
    {
        spec_hash: Digest,
        input_hash: Digest,
        vector: Vec<f32>,
        model: String,
    }
);

/// Invoke `$mac!(Table, …)` with every global table (read mode `global`, ADR-0017 amendment).
#[macro_export]
macro_rules! for_each_global_table {
    ($mac:ident) => {
        $mac!($crate::embedding::EmbeddingCache)
    };
}

/// Whether a table is read globally (at its recorded version over all its files, no snapshot
/// filter) rather than by its snapshot's commit. Declared once, by the group above.
pub fn is_global(name: &str) -> bool {
    macro_rules! any {
        ($($t:ty),+) => { [$(<$t as $crate::table::Table>::NAME),+].contains(&name) };
    }
    crate::for_each_global_table!(any)
}
