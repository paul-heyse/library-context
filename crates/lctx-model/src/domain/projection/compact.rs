//! Primitive source correspondence retained by compact topology construction.
use super::*;
use crate::domain::attribution::{CoverageStatus, FactFamily, ProviderCoverage};
use crate::domain::{
    assertion::AssertionQualification,
    calls::{ProviderModule, ProviderSymbol},
    charged::{ChargedMap, StateCharge},
    lexical::ReferenceObservation,
    resources::ResourceBudget,
    source::{Module, Occurrence, SourceArtifact},
    syntax::ImportAliasObservation,
};
use arrow_array::{Array, FixedSizeBinaryArray, Int16Array};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CoverageFrame {
    pub id: Id<ProviderCoverage>,
    pub scope: Id<CoverageScope>,
    pub context: Id<AnalysisContext>,
    pub family: FactFamily,
    pub status: CoverageStatus,
}
impl HeapSize for CoverageFrame {}

pub(super) trait Metadata {
    fn coverages(&self) -> Box<dyn Iterator<Item = CoverageFrame> + '_>;
    fn artifact_input(&self, id: Id<SourceArtifact>) -> Result<Id<InputRevision>, ModelError>;
    fn occurrence_source(&self, id: Id<Occurrence>) -> Result<Id<SourceArtifact>, ModelError>;
    fn module_source(&self, id: Id<Module>) -> Result<Id<SourceArtifact>, ModelError>;
    fn symbol_frame(
        &self,
        id: Id<ProviderSymbol>,
    ) -> Result<(Id<AnalysisContext>, Id<ProviderModule>), ModelError>;
    fn acquired_module(&self, id: Id<ProviderModule>) -> Result<Option<Id<Module>>, ModelError>;
    fn field_class(&self, id: Id<FieldEntity>) -> Result<Id<ClassEntity>, ModelError>;
    fn import_frame(
        &self,
        id: Id<ImportAliasObservation>,
    ) -> Result<(Id<AssertionQualification>, Id<Occurrence>), ModelError>;
    fn reference_frame(
        &self,
        id: Id<ReferenceObservation>,
    ) -> Result<(Id<AssertionQualification>, Id<Occurrence>), ModelError>;
}
fn absent(name: &'static str) -> ModelError {
    ModelError::Invalid(format!("projection requires {name}"))
}
pub(super) struct PrimitiveMetadata {
    coverage: ChargedMap<Id<ProviderCoverage>, CoverageFrame>,
    artifacts: ChargedMap<Id<SourceArtifact>, Id<InputRevision>>,
    occurrences: ChargedMap<Id<Occurrence>, Id<SourceArtifact>>,
    modules: ChargedMap<Id<Module>, Id<SourceArtifact>>,
    symbols: ChargedMap<Id<ProviderSymbol>, (Id<AnalysisContext>, Id<ProviderModule>)>,
    provider_modules: ChargedMap<Id<ProviderModule>, Option<Id<Module>>>,
    fields: ChargedMap<Id<FieldEntity>, Id<ClassEntity>>,
    imports: ChargedMap<Id<ImportAliasObservation>, (Id<AssertionQualification>, Id<Occurrence>)>,
    references: ChargedMap<Id<ReferenceObservation>, (Id<AssertionQualification>, Id<Occurrence>)>,
    charge: StateCharge,
}
impl PrimitiveMetadata {
    pub(super) fn new(b: &ResourceBudget) -> Self {
        Self {
            coverage: Default::default(),
            artifacts: Default::default(),
            occurrences: Default::default(),
            modules: Default::default(),
            symbols: Default::default(),
            provider_modules: Default::default(),
            fields: Default::default(),
            imports: Default::default(),
            references: Default::default(),
            charge: StateCharge::new(b, "projection-source-correspondence"),
        }
    }
    pub(super) fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        if compact_columns(name).is_none() {
            return Ok(false);
        }
        macro_rules! one {
            ($ty:ty,$map:ident,$field:literal) => {{
                for row in 0..batch.num_rows() {
                    let key: Id<$ty> = nominal(batch, "id", row)?;
                    let value = nominal(batch, $field, row)?;
                    if self
                        .$map
                        .get(&key)
                        .is_some_and(|existing| *existing != value)
                    {
                        return Err(ModelError::Conflict(<$ty>::NAME));
                    }
                    self.$map.insert(&mut self.charge, key, value)?;
                }
            }};
        }
        macro_rules! two {
            ($ty:ty,$map:ident,$first:literal,$second:literal) => {{
                for row in 0..batch.num_rows() {
                    let key: Id<$ty> = nominal(batch, "id", row)?;
                    let value = (nominal(batch, $first, row)?, nominal(batch, $second, row)?);
                    if self
                        .$map
                        .get(&key)
                        .is_some_and(|existing| *existing != value)
                    {
                        return Err(ModelError::Conflict(<$ty>::NAME));
                    }
                    self.$map.insert(&mut self.charge, key, value)?;
                }
            }};
        }
        match name {
            ProviderCoverage::NAME => {
                for row in 0..batch.num_rows() {
                    let id: Id<ProviderCoverage> = nominal(batch, "id", row)?;
                    let value = CoverageFrame {
                        id,
                        scope: nominal(batch, "scope", row)?,
                        context: nominal(batch, "context", row)?,
                        family: code(batch, "family", row)?,
                        status: code(batch, "status", row)?,
                    };
                    if self
                        .coverage
                        .get(&id)
                        .is_some_and(|existing| *existing != value)
                    {
                        return Err(ModelError::Conflict(ProviderCoverage::NAME));
                    }
                    self.coverage.insert(&mut self.charge, id, value)?;
                }
            }
            SourceArtifact::NAME => one!(SourceArtifact, artifacts, "input"),
            Occurrence::NAME => one!(Occurrence, occurrences, "source"),
            Module::NAME => one!(Module, modules, "source"),
            ProviderSymbol::NAME => two!(ProviderSymbol, symbols, "context", "module"),
            FieldEntity::NAME => one!(FieldEntity, fields, "class"),
            ImportAliasObservation::NAME => {
                two!(ImportAliasObservation, imports, "qualification", "alias")
            }
            ReferenceObservation::NAME => {
                two!(ReferenceObservation, references, "qualification", "read")
            }
            ProviderModule::NAME => {
                for row in 0..batch.num_rows() {
                    let key: Id<ProviderModule> = nominal(batch, "id", row)?;
                    let column = batch
                        .column_by_name("acquired_module")
                        .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
                        .ok_or(ModelError::Schema("projection acquired module"))?;
                    let value = if column.is_null(row) {
                        None
                    } else {
                        Some(nominal(batch, "acquired_module", row)?)
                    };
                    if self
                        .provider_modules
                        .get(&key)
                        .is_some_and(|existing| *existing != value)
                    {
                        return Err(ModelError::Conflict(ProviderModule::NAME));
                    }
                    self.provider_modules.insert(&mut self.charge, key, value)?;
                }
            }
            _ => return Err(ModelError::Schema("projection compact metadata")),
        }
        Ok(true)
    }
}
impl Metadata for PrimitiveMetadata {
    fn coverages(&self) -> Box<dyn Iterator<Item = CoverageFrame> + '_> {
        Box::new(self.coverage.values().copied())
    }
    fn artifact_input(&self, id: Id<SourceArtifact>) -> Result<Id<InputRevision>, ModelError> {
        self.artifacts
            .get(&id)
            .copied()
            .ok_or(absent(SourceArtifact::NAME))
    }
    fn occurrence_source(&self, id: Id<Occurrence>) -> Result<Id<SourceArtifact>, ModelError> {
        self.occurrences
            .get(&id)
            .copied()
            .ok_or(absent(Occurrence::NAME))
    }
    fn module_source(&self, id: Id<Module>) -> Result<Id<SourceArtifact>, ModelError> {
        self.modules.get(&id).copied().ok_or(absent(Module::NAME))
    }
    fn symbol_frame(
        &self,
        id: Id<ProviderSymbol>,
    ) -> Result<(Id<AnalysisContext>, Id<ProviderModule>), ModelError> {
        self.symbols
            .get(&id)
            .copied()
            .ok_or(absent(ProviderSymbol::NAME))
    }
    fn acquired_module(&self, id: Id<ProviderModule>) -> Result<Option<Id<Module>>, ModelError> {
        self.provider_modules
            .get(&id)
            .copied()
            .ok_or(absent(ProviderModule::NAME))
    }
    fn field_class(&self, id: Id<FieldEntity>) -> Result<Id<ClassEntity>, ModelError> {
        self.fields
            .get(&id)
            .copied()
            .ok_or(absent(FieldEntity::NAME))
    }
    fn import_frame(
        &self,
        id: Id<ImportAliasObservation>,
    ) -> Result<(Id<AssertionQualification>, Id<Occurrence>), ModelError> {
        self.imports
            .get(&id)
            .copied()
            .ok_or(absent(ImportAliasObservation::NAME))
    }
    fn reference_frame(
        &self,
        id: Id<ReferenceObservation>,
    ) -> Result<(Id<AssertionQualification>, Id<Occurrence>), ModelError> {
        self.references
            .get(&id)
            .copied()
            .ok_or(absent(ReferenceObservation::NAME))
    }
}
fn nominal<R>(
    batch: &arrow_array::RecordBatch,
    name: &str,
    row: usize,
) -> Result<Id<R>, ModelError> {
    let column = batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .ok_or(ModelError::Schema("projection nominal metadata column"))?;
    if column.is_null(row) {
        return Err(ModelError::Invalid(
            "null projection metadata reference".into(),
        ));
    }
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(column.value(row).iter().copied()))
    .map_err(ModelError::codec)
}
fn code<C: Codebook>(
    batch: &arrow_array::RecordBatch,
    name: &str,
    row: usize,
) -> Result<C, ModelError> {
    let column = batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<Int16Array>())
        .ok_or(ModelError::Schema("projection code metadata column"))?;
    if column.is_null(row) {
        return Err(ModelError::Invalid("null projection metadata code".into()));
    }
    C::from_code(column.value(row)).ok_or(ModelError::Schema("projection metadata code"))
}
/// Select these fields in SQL before any rich source metadata is decoded.
pub fn compact_columns(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        ProviderCoverage::NAME => &["id", "scope", "context", "family", "status"],
        SourceArtifact::NAME => &["id", "input"],
        Occurrence::NAME | Module::NAME => &["id", "source"],
        ProviderSymbol::NAME => &["id", "context", "module"],
        ProviderModule::NAME => &["id", "acquired_module"],
        FieldEntity::NAME => &["id", "class"],
        ImportAliasObservation::NAME => &["id", "qualification", "alias"],
        ReferenceObservation::NAME => &["id", "qualification", "read"],
        _ => return None,
    })
}
