//! Properties of an already completed occurrence stream; stored nominal identity is preserved.
//! This is neither a canonical Occurrence row nor independently mintable semantic evidence.
use super::{Occurrence, OccurrenceRole, SourceArtifact, SyntaxKind};
use crate::domain::{
    analysis::sources::CompletedInput,
    charged::{ChargedMap, StateCharge},
    resources::ResourceBudget,
    *,
};
mod sealed {
    pub trait OccurrenceView {}
}
/// The source properties consumed by call composition. Implementations are model-owned.
pub trait OccurrenceView: sealed::OccurrenceView + Send + Sync {
    fn occurrence_id(&self) -> Id<Occurrence>;
    fn syntax_kind(&self) -> SyntaxKind;
    fn role(&self) -> OccurrenceRole;
    fn validate_site(&self) -> Result<(), ModelError>;
}
impl sealed::OccurrenceView for Occurrence {}
impl OccurrenceView for Occurrence {
    fn occurrence_id(&self) -> Id<Occurrence> {
        Record::id(self)
    }
    fn syntax_kind(&self) -> SyntaxKind {
        self.syntax_kind
    }
    fn role(&self) -> OccurrenceRole {
        self.role
    }
    fn validate_site(&self) -> Result<(), ModelError> {
        Record::validate(self)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccurrenceProperties {
    id: Id<Occurrence>,
    pub(crate) source: Id<SourceArtifact>,
    pub(crate) start: i64,
    pub(crate) end: i64,
    pub(crate) syntax_kind: SyntaxKind,
    pub(crate) role: OccurrenceRole,
}
impl HeapSize for OccurrenceProperties {}
impl OccurrenceProperties {
    pub fn id(&self) -> Id<Occurrence> {
        self.id
    }
    pub fn syntax_kind(&self) -> SyntaxKind {
        self.syntax_kind
    }
    pub(crate) fn from_row(row: &Occurrence) -> Self {
        Self {
            id: Record::id(row),
            source: row.source,
            start: row.start,
            end: row.end,
            syntax_kind: row.syntax_kind,
            role: row.role,
        }
    }
}
impl sealed::OccurrenceView for OccurrenceProperties {}
impl OccurrenceView for OccurrenceProperties {
    fn occurrence_id(&self) -> Id<Occurrence> {
        self.id
    }
    fn syntax_kind(&self) -> SyntaxKind {
        self.syntax_kind
    }
    fn role(&self) -> OccurrenceRole {
        self.role
    }
    fn validate_site(&self) -> Result<(), ModelError> {
        if self.start < 0 || self.end < self.start {
            Err(ModelError::Invalid(
                "invalid completed occurrence property span".into(),
            ))
        } else {
            Ok(())
        }
    }
}
#[derive(serde::Deserialize)]
struct ProjectedOccurrence {
    id: Id<Occurrence>,
    source: Id<SourceArtifact>,
    start: i64,
    end: i64,
    syntax_kind: SyntaxKind,
    role: OccurrenceRole,
}
pub struct OccurrenceIndex {
    rows: ChargedMap<Id<Occurrence>, OccurrenceProperties>,
    charge: StateCharge,
}
impl OccurrenceIndex {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            rows: Default::default(),
            charge: StateCharge::new(budget, "completed-occurrence-properties"),
        }
    }
    pub fn columns() -> &'static str {
        "id,source,start,end,syntax_kind,role"
    }
    pub fn visit(
        &mut self,
        _permit: &CompletedInput<Occurrence>,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        // Projection size is fixed per row; no structural path participates in this transfer.
        let _transfer = self.charge.budget().expect("property budget").reserve(
            "occurrence-property-transfer",
            batch
                .num_rows()
                .saturating_mul(size_of::<OccurrenceProperties>() * 2),
        )?;
        let rows: Vec<ProjectedOccurrence> =
            serde_arrow::from_record_batch(batch).map_err(ModelError::codec)?;
        for raw in rows {
            let row = OccurrenceProperties {
                id: raw.id,
                source: raw.source,
                start: raw.start,
                end: raw.end,
                syntax_kind: raw.syntax_kind,
                role: raw.role,
            };
            row.validate_site()?;
            if let Some(previous) = self.rows.get(&row.id) {
                if previous != &row {
                    return Err(ModelError::Conflict(Occurrence::NAME));
                }
            } else {
                self.rows.insert(&mut self.charge, row.id, row)?;
            }
        }
        Ok(())
    }
    pub fn get(&self, id: Id<Occurrence>) -> Option<&OccurrenceProperties> {
        self.rows.get(&id)
    }
    pub fn iter(&self) -> impl Iterator<Item = &OccurrenceProperties> {
        self.rows.values()
    }
}

mod symbol_sealed {
    pub trait ProviderSymbolView {}
}
pub trait ProviderSymbolView: symbol_sealed::ProviderSymbolView + Send + Sync {
    fn symbol_id(&self) -> Id<calls::ProviderSymbol>;
}
impl symbol_sealed::ProviderSymbolView for calls::ProviderSymbol {}
impl ProviderSymbolView for calls::ProviderSymbol {
    fn symbol_id(&self) -> Id<calls::ProviderSymbol> {
        Record::id(self)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolProperties {
    id: Id<calls::ProviderSymbol>,
}
impl symbol_sealed::ProviderSymbolView for SymbolProperties {}
impl ProviderSymbolView for SymbolProperties {
    fn symbol_id(&self) -> Id<calls::ProviderSymbol> {
        self.id
    }
}
impl HeapSize for SymbolProperties {}
impl SymbolProperties {
    pub fn id(&self) -> Id<calls::ProviderSymbol> {
        self.id
    }
    pub(crate) fn from_row(row: &calls::ProviderSymbol) -> Self {
        Self {
            id: Record::id(row),
        }
    }
}
#[derive(serde::Deserialize)]
struct ProjectedSymbol {
    id: Id<calls::ProviderSymbol>,
}
pub struct SymbolIndex {
    rows: ChargedMap<Id<calls::ProviderSymbol>, SymbolProperties>,
    charge: StateCharge,
}
impl SymbolIndex {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            rows: Default::default(),
            charge: StateCharge::new(budget, "completed-symbol-identities"),
        }
    }
    pub fn columns() -> &'static str {
        "id"
    }
    pub fn visit(
        &mut self,
        _permit: &CompletedInput<calls::ProviderSymbol>,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        let _transfer = self.charge.budget().expect("symbol budget").reserve(
            "symbol-property-transfer",
            batch
                .num_rows()
                .saturating_mul(size_of::<ProjectedSymbol>() * 2),
        )?;
        let rows: Vec<ProjectedSymbol> =
            serde_arrow::from_record_batch(batch).map_err(ModelError::codec)?;
        for row in rows {
            if !self.rows.contains_key(&row.id) {
                self.rows
                    .insert(&mut self.charge, row.id, SymbolProperties { id: row.id })?;
            }
        }
        Ok(())
    }
    pub fn get(&self, id: Id<calls::ProviderSymbol>) -> Option<&SymbolProperties> {
        self.rows.get(&id)
    }
}
#[derive(serde::Deserialize)]
struct ProjectedArtifact {
    id: Id<SourceArtifact>,
    input: Id<input::InputRevision>,
}
pub struct ArtifactIndex {
    rows: ChargedMap<Id<SourceArtifact>, Id<input::InputRevision>>,
    charge: StateCharge,
}
impl ArtifactIndex {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            rows: Default::default(),
            charge: StateCharge::new(budget, "completed-artifact-inputs"),
        }
    }
    pub fn columns() -> &'static str {
        "id,input"
    }
    pub fn visit(
        &mut self,
        _permit: &CompletedInput<SourceArtifact>,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        let _transfer = self.charge.budget().expect("artifact budget").reserve(
            "artifact-property-transfer",
            batch
                .num_rows()
                .saturating_mul(size_of::<ProjectedArtifact>() * 2),
        )?;
        let rows: Vec<ProjectedArtifact> =
            serde_arrow::from_record_batch(batch).map_err(ModelError::codec)?;
        for row in rows {
            if let Some(previous) = self.rows.get(&row.id) {
                if previous != &row.input {
                    return Err(ModelError::Conflict(SourceArtifact::NAME));
                }
            } else {
                self.rows.insert(&mut self.charge, row.id, row.input)?;
            }
        }
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn get(&self, id: Id<SourceArtifact>) -> Option<Id<input::InputRevision>> {
        self.rows.get(&id).copied()
    }
}

#[cfg(test)]
mod property_controls {
    use super::*;
    fn nominal<R>(n: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn completed_symbol_identity_does_not_decode_native_name_payload() {
        let symbol = calls::ProviderSymbol {
            provider: nominal(1),
            context: nominal(2),
            module: nominal(3),
            native_key: "native".repeat(1 << 20),
            name: "unused name".repeat(1 << 20),
            kind: calls::SymbolKind::Function,
        };
        let batch = <calls::ProviderSymbol as Record>::encode(&[symbol.clone()]).unwrap();
        let projected = batch.project(&[0]).unwrap();
        let budget = ResourceBudget::fixed(96 << 10).unwrap();
        let permit = CompletedInput::<calls::ProviderSymbol>::new(
            "properties",
            ContentHash::of(b"contract"),
            ContentHash::of(b"implementation"),
            ContentHash::of(b"symbol-stream"),
            1,
        )
        .unwrap();
        let mut index = SymbolIndex::new(&budget);
        index.visit(&permit, &projected).unwrap();
        assert_eq!(index.get(symbol.id()).unwrap().symbol_id(), symbol.id());
        drop(index);
        assert_eq!(budget.reserved(), 0);
        let mut rich = crate::domain::normalized::Rows::<calls::ProviderSymbol>::new(&budget);
        assert!(rich.decode(&batch).is_err());
        drop(rich);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn completed_occurrence_preserves_nominal_id_and_conditional_restatement_without_path_payload()
    {
        use crate::domain::{
            conditions::{rebase::*, *},
            normalized::Rows,
            value::Predicate,
        };
        let occurrence = Occurrence {
            source: nominal(1),
            start: 3,
            end: 15,
            syntax_kind: SyntaxKind::ExprCall,
            role: OccurrenceRole::Call,
            structural_path: vec![7; 2 << 20],
        };
        let batch = <Occurrence as Record>::encode(&[occurrence.clone()]).unwrap();
        let projected = batch.project(&[0, 1, 2, 3, 4, 5]).unwrap();
        let budget = ResourceBudget::fixed(256 << 10).unwrap();
        let permit = CompletedInput::<Occurrence>::new(
            "properties",
            ContentHash::of(b"contract"),
            ContentHash::of(b"implementation"),
            ContentHash::of(b"stream"),
            1,
        )
        .unwrap();
        let mut index = OccurrenceIndex::new(&budget);
        index.visit(&permit, &projected).unwrap();
        let site = index.get(Record::id(&occurrence)).unwrap();
        assert_eq!(site.id(), Record::id(&occurrence));
        let predicate = Predicate::Opaque {
            text: "café λ 😀".into(),
        };
        let atom = EvaluationAtom {
            evaluation: nominal(2),
            context: nominal(3),
            predicate: predicate.id(),
            operand: None,
        };
        let atoms = std::collections::BTreeMap::from([(atom.id(), atom.clone())]);
        let predicates = std::collections::BTreeMap::from([(predicate.id(), predicate)]);
        let places = std::collections::BTreeMap::new();
        let roots = std::collections::BTreeMap::new();
        let catalog = GuardCatalog {
            atoms: &atoms,
            predicates: &predicates,
            places: &places,
            roots: &roots,
        };
        let source = Diagram::from_atom(atom.id()).not().unwrap();
        let ordinary =
            rebase_local_guards(&source, &occurrence, atom.context, &catalog, &budget).unwrap();
        let selected = rebase_local_guards(&source, site, atom.context, &catalog, &budget).unwrap();
        assert_eq!(ordinary.condition.id(), selected.condition.id());
        assert_eq!(ordinary.atoms, selected.atoms);
        assert_eq!(ordinary.predicates, selected.predicates);
        assert_eq!(selected.atoms[0].evaluation, Record::id(&occurrence));
        drop(ordinary);
        drop(selected);
        drop(index);
        assert_eq!(budget.reserved(), 0);
        let mut rich = Rows::<Occurrence>::new(&budget);
        assert!(rich.decode(&batch).is_err());
        drop(rich);
        assert_eq!(budget.reserved(), 0);
    }
}
