//! Fact ids and `facts` provenance rows (DESIGN §3.4.1, §B6).

use std::collections::BTreeMap;

use cpg_schema::codebook::{ExtractionMode, Fidelity, Modality, Origin};
use cpg_schema::id::{Id, IdHasher, kind};
use cpg_schema::tables::FactsRow;

/// The model surfaces of this producer (`model_id` = `<producer_id>/<surface>`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Surface {
    RuffAst,
    PyreflyPysa,
    PyreflyPublic,
    Source,
    /// The extractor's own comparison of two surfaces (boundary facts; slice-1 review O4).
    Compare,
    /// Our lexical recognizer (C3).
    Lexical,
    /// Pyrefly's native types (C4).
    PyreflyTypes,
    /// markdown-rs's syntax tree of a document (C5).
    MarkdownRs,
    /// Our docs recognizer (C5 mentions).
    Docs,
    /// Pyrefly's docstring parameter parser (`parse_parameter_documentation`, slice 2.1).
    PyreflyDocstring,
}

impl Surface {
    fn name(self) -> &'static str {
        match self {
            Surface::RuffAst => "ruff-ast",
            Surface::PyreflyPysa => "pyrefly-pysa",
            Surface::PyreflyPublic => "pyrefly-public",
            Surface::Source => "source",
            Surface::Compare => "compare",
            Surface::Lexical => "lctx-lexical",
            Surface::PyreflyTypes => "pyrefly-types",
            Surface::MarkdownRs => "markdown-rs",
            Surface::Docs => "lctx-docs",
            Surface::PyreflyDocstring => "pyrefly-docstring",
        }
    }
}

/// Collects `facts` rows; the same assertion from the same run gets the same id and is kept once.
/// Provenance is not part of the id, so the same payload arriving with different provenance is a
/// mapping bug and fails the extraction (review F8).
pub(crate) struct FactSink {
    pub run_id: Id,
    pub snapshot_id: Id,
    producer_hex: String,
    facts: BTreeMap<[u8; 16], FactsRow>,
    conflicts: Vec<Id>,
}

pub(crate) struct Provenance {
    pub surface: Surface,
    pub mode: ExtractionMode,
    pub origin: Origin,
    pub modality: Modality,
    pub fidelity: Fidelity,
}

impl FactSink {
    pub fn new(run_id: Id, snapshot_id: Id, producer_id: Id) -> Self {
        Self {
            run_id,
            snapshot_id,
            producer_hex: producer_id.hex(),
            facts: BTreeMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// The fact id for a row whose payload `hash` feeds; records the provenance row.
    pub fn fact(&mut self, table: &str, hash: impl FnOnce(&mut IdHasher), p: Provenance) -> Id {
        let mut h = IdHasher::new(kind::FACT);
        h.id(self.run_id).str(table);
        hash(&mut h);
        let fact_id = h.finish_id();
        let row = FactsRow {
            snapshot_id: self.snapshot_id,
            fact_id,
            run_id: self.run_id,
            table_name: table.to_owned(),
            origin: p.origin,
            extraction_mode: p.mode,
            modality: p.modality,
            fidelity: p.fidelity,
            model_id: format!("{}/{}", self.producer_hex, p.surface.name()),
        };
        let kept = self.facts.entry(fact_id.0).or_insert_with(|| row.clone());
        if *kept != row {
            self.conflicts.push(fact_id);
        }
        fact_id
    }

    pub fn into_rows(self) -> Result<Vec<FactsRow>, crate::ExtractError> {
        match self.conflicts.first() {
            Some(id) => Err(crate::ExtractError::ProvenanceConflict(id.hex())),
            None => Ok(self.facts.into_values().collect()),
        }
    }
}

/// Finish a raw row: derive its `fact_id` from the payload (with `snapshot_id` and `fact_id`
/// zeroed, so neither feeds the id), then set both.
macro_rules! fact_row {
    ($sink:expr, $table:ty, $prov:expr, $row:expr) => {{
        let mut row = $row;
        row.snapshot_id = cpg_schema::id::Id::ZERO;
        row.fact_id = cpg_schema::id::Id::ZERO;
        let fact_id = $sink.fact(
            <$table as cpg_schema::table::Table>::NAME,
            |h| row.hash_fields(h),
            $prov,
        );
        row.fact_id = fact_id;
        row.snapshot_id = $sink.snapshot_id;
        row
    }};
}
pub(crate) use fact_row;

/// Keep one row per fact id (the same assertion ingested twice).
pub(crate) fn dedup_by_fact<R>(rows: &mut Vec<R>, fact_id: impl Fn(&R) -> Id) {
    let mut seen = std::collections::HashSet::new();
    rows.retain(|r| seen.insert(fact_id(r).0));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(modality: Modality) -> Provenance {
        Provenance {
            surface: Surface::PyreflyPysa,
            mode: ExtractionMode::NativeTraversal,
            origin: Origin::AnalyzerAssertion,
            modality,
            fidelity: Fidelity::ReportProjection,
        }
    }

    #[test]
    fn a_repeated_fact_keeps_one_row_and_a_provenance_conflict_is_an_error() {
        let payload = |h: &mut IdHasher| {
            h.str("same payload");
        };
        let mut sink = FactSink::new(Id([1; 16]), Id([2; 16]), Id([3; 16]));
        let a = sink.fact("pysa_calls", payload, p(Modality::Definite));
        let b = sink.fact("pysa_calls", payload, p(Modality::Definite));
        assert_eq!(a, b);
        assert_eq!(sink.into_rows().unwrap().len(), 1);

        let mut sink = FactSink::new(Id([1; 16]), Id([2; 16]), Id([3; 16]));
        sink.fact("pysa_calls", payload, p(Modality::Definite));
        sink.fact("pysa_calls", payload, p(Modality::Candidate));
        assert!(matches!(
            sink.into_rows(),
            Err(crate::ExtractError::ProvenanceConflict(_))
        ));
    }
}
