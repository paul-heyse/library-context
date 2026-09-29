//! Temporary migration machinery (cutover plan §3.4–§3.6), deleted in phase 5.
//!
//! Nothing here is part of the model (DESIGN §15.13). It keeps legacy producers and their
//! identities working while each layer cuts over.

use crate::id::IdHasher;

/// The legacy identity domain. Part of every legacy id and digest.
pub const ID_TAG_V1: &[u8] = b"lctx-id/v1";

impl IdHasher {
    /// A legacy `lctx-id/v1` hasher over an ad hoc kind tag. New code uses a declared kind.
    pub fn new(kind_tag: &str) -> Self {
        let mut h = blake3::Hasher::new();
        h.update(ID_TAG_V1);
        let mut this = Self(h);
        this.bytes(kind_tag.as_bytes());
        this
    }

}

/// A legacy behavior an adapter reproduces on purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quirk {
    pub name: &'static str,
    pub description: &'static str,
}

/// Which side of a parity comparison a divergence's rows are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Rows only the legacy producer emits.
    Legacy,
    /// Rows only the new producer (plus adapter) emits.
    New,
}

/// An intentional difference between legacy and new output: rows on `side` matching `predicate`
/// (DataFusion SQL over the relation's columns) are declared, not failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divergence {
    pub name: &'static str,
    /// The review finding or reason it exists (for example `F04`).
    pub finding: &'static str,
    pub reason: &'static str,
    pub side: Side,
    pub predicate: &'static str,
}

/// How an adapter computes its legacy relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Computation {
    /// DataFusion SQL over the adapter's inputs, producing the legacy relation's columns.
    Sql(&'static str),
    /// A named Rust function, where a legacy encoding needs one.
    Rust(&'static str),
}

/// A declared legacy adapter (cutover plan §3.4): one legacy relation computed from new relations,
/// read only by legacy consumers, deleted in `delete_in_phase`.
#[derive(Clone, Copy, Debug)]
pub struct AdapterDecl {
    pub produces: &'static str,
    pub inputs: &'static [&'static str],
    pub computation: Computation,
    pub quirks: &'static [Quirk],
    pub divergences: &'static [Divergence],
    pub delete_in_phase: u8,
}

/// A temporary legacy-identity side relation (cutover plan §3.5): the new producer records, for
/// each new id of a family, the id the legacy recipe gives the same row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyIdDecl {
    pub family: &'static str,
    pub new_kind: crate::id::IdKind,
}

impl LegacyIdDecl {
    pub fn relation(&self) -> String {
        format!("legacy_ids_{}", self.family)
    }

    /// The side relation's table: `new_id → legacy_id`, keyed by the new id.
    pub fn table_spec(&self) -> Result<crate::ddl::TableSpec, crate::ddl::DdlError> {
        use arrow_schema::{DataType, Field, Schema};
        let schema = Schema::new(vec![
            Field::new(crate::decl::relation::GENERATION_COLUMN, DataType::FixedSizeBinary(16), false),
            Field::new("new_id", DataType::FixedSizeBinary(16), false),
            Field::new("legacy_id", DataType::FixedSizeBinary(16), false),
        ]);
        let mut spec = crate::ddl::TableSpec::build(
            &self.relation(),
            &schema,
            crate::decl::relation::GENERATION_COLUMN,
            &crate::ddl::TableOptions {
                key: &[crate::decl::relation::GENERATION_COLUMN, "new_id"],
                unique: &[&["legacy_id"]],
                ..Default::default()
            },
        )?;
        spec.partition.supplied = true;
        Ok(spec)
    }
}

/// Exact multiset comparison of relation contents (cutover plan §3.6).
pub mod parity {
    use std::collections::BTreeMap;

    use arrow_array::{RecordBatch, UInt32Array};
    use arrow_row::{RowConverter, SortField};
    use arrow_schema::ArrowError;
    use serde::Serialize;

    /// The rows only one side holds, with multiplicity.
    #[derive(Clone, Debug)]
    pub struct Diff {
        pub relation: String,
        pub legacy_rows: usize,
        pub new_rows: usize,
        pub legacy_only: RecordBatch,
        pub new_only: RecordBatch,
    }

    impl Diff {
        pub fn equal(&self) -> bool {
            self.legacy_only.num_rows() == 0 && self.new_only.num_rows() == 0
        }
    }

    /// Compare two batches of one relation as multisets of rows, by value (floats by bits, nulls
    /// equal). Field names and types must agree; metadata is ignored.
    pub fn diff(relation: &str, legacy: &RecordBatch, new: &RecordBatch) -> Result<Diff, ArrowError> {
        let same_shape = legacy.schema().fields().len() == new.schema().fields().len()
            && legacy
                .schema()
                .fields()
                .iter()
                .zip(new.schema().fields())
                .all(|(a, b)| a.name() == b.name() && a.data_type() == b.data_type());
        if !same_shape {
            return Err(ArrowError::SchemaError(format!(
                "{relation}: legacy and new schemas differ"
            )));
        }
        let converter = RowConverter::new(
            legacy
                .schema()
                .fields()
                .iter()
                .map(|f| SortField::new(f.data_type().clone()))
                .collect(),
        )?;
        let left = converter.convert_columns(legacy.columns())?;
        let right = converter.convert_columns(new.columns())?;
        // Per distinct row: its first index on each side, and its count on each side.
        let mut counts: BTreeMap<Vec<u8>, (Option<u32>, i64, Option<u32>, i64)> = BTreeMap::new();
        for (i, row) in left.iter().enumerate() {
            let e = counts.entry(row.as_ref().to_vec()).or_default();
            e.0.get_or_insert(i as u32);
            e.1 += 1;
        }
        for (i, row) in right.iter().enumerate() {
            let e = counts.entry(row.as_ref().to_vec()).or_default();
            e.2.get_or_insert(i as u32);
            e.3 += 1;
        }
        let (mut legacy_idx, mut new_idx) = (Vec::new(), Vec::new());
        for (first_legacy, legacy_count, first_new, new_count) in counts.values() {
            for _ in 0..(legacy_count - new_count).max(0) {
                legacy_idx.push(first_legacy.expect("counted on the legacy side"));
            }
            for _ in 0..(new_count - legacy_count).max(0) {
                new_idx.push(first_new.expect("counted on the new side"));
            }
        }
        let take = |batch: &RecordBatch, idx: Vec<u32>| {
            arrow_select::take::take_record_batch(batch, &UInt32Array::from(idx))
        };
        Ok(Diff {
            relation: relation.to_owned(),
            legacy_rows: legacy.num_rows(),
            new_rows: new.num_rows(),
            legacy_only: take(legacy, legacy_idx)?,
            new_only: take(new, new_idx)?,
        })
    }

    /// One relation's verdict after declared divergences are applied.
    #[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
    pub struct RelationVerdict {
        pub relation: String,
        pub legacy_rows: usize,
        pub new_rows: usize,
        pub legacy_only: usize,
        pub new_only: usize,
        pub declared: usize,
        pub undeclared: usize,
        /// Up to a few rendered undeclared rows, for diagnosis.
        pub samples: Vec<String>,
        /// A comparison that could not be made (schema mismatch, a missing relation).
        pub error: Option<String>,
    }

    impl RelationVerdict {
        pub fn passed(&self) -> bool {
            self.undeclared == 0 && self.error.is_none()
        }
    }

    /// A parity run's report.
    #[derive(Clone, Debug, Serialize)]
    pub struct Report {
        pub phase: u8,
        pub corpus: String,
        pub relations: Vec<RelationVerdict>,
    }

    impl Report {
        pub fn passed(&self) -> bool {
            self.relations.iter().all(RelationVerdict::passed)
        }

        pub fn markdown(&self) -> String {
            let mut out = format!(
                "# Parity: phase {}, corpus {}\n\n**{}**: {} relations, {} failing.\n\n\
                 | Relation | Legacy rows | New rows | Legacy only | New only | Declared | Undeclared |\n\
                 |---|---|---|---|---|---|---|\n",
                self.phase,
                self.corpus,
                if self.passed() { "passed" } else { "failed" },
                self.relations.len(),
                self.relations.iter().filter(|r| !r.passed()).count(),
            );
            for r in &self.relations {
                out.push_str(&format!(
                    "| {}{} | {} | {} | {} | {} | {} | {} |\n",
                    r.relation,
                    r.error.as_ref().map(|e| format!(" ({e})")).unwrap_or_default(),
                    r.legacy_rows,
                    r.new_rows,
                    r.legacy_only,
                    r.new_only,
                    r.declared,
                    r.undeclared
                ));
            }
            out
        }
    }
}
