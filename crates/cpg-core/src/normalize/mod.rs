//! Computed normalization stages over admitted completed-stage inputs.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use arrow_array::Array;
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::{
        Rows,
        entity_normalization::{self, EntityData},
    },
    stages::*,
    *,
};
use std::sync::Arc;
mod admission;
pub(crate) mod call_scope;
mod callable_scope;
mod projection_admission;
mod receiver_scope;
pub use admission::{validate_bindings, validate_events, validate_receivers};
pub use projection_admission::validate_projections;

/// Actual normalization output authority bound to the immutable attempt descriptors.
pub(crate) struct ProducedNormalization<T> {
    premises: CompletedInputs,
    outputs: CompletedInputs,
    value: T,
}
impl<T> ProducedNormalization<T> {
    fn borrow(&self, access: &CompletedInputs, workspace: &Workspace) -> Result<&T, ModelError> {
        self.premises.require_subset(workspace, access)?;
        self.outputs.require_subset(workspace, access)?;
        Ok(&self.value)
    }
}
fn selected_premises(
    access: &CompletedInputs,
    inputs: Vec<ValidationInput>,
) -> Result<CompletedInputs, ModelError> {
    access.select(
        &inputs
            .into_iter()
            .filter(|input| access.table_for(input).is_ok())
            .collect::<Vec<_>>(),
    )
}
// A compact exact-key work list closes a semantic kernel's premises without retaining a copy of
// any other scope. Reverse candidate selection is SQL; forward nominal references use bounded
// key reads against the immutable, qualified workspace views.
struct PremiseClosure {
    wanted: charged::ChargedSet<(&'static str, [u8; 16])>,
    loaded: charged::ChargedSet<(&'static str, [u8; 16])>,
    charge: charged::StateCharge,
}
impl PremiseClosure {
    fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            wanted: Default::default(),
            loaded: Default::default(),
            charge: charged::StateCharge::new(budget, "normalization-scope-keys"),
        }
    }
    fn absorb<R: Record>(
        &mut self,
        batch: &arrow_array::RecordBatch,
        rows: &mut Rows<R>,
    ) -> Result<(), ModelError> {
        let budget = self.charge.budget().expect("closure budget");
        let _decode =
            budget.reserve("normalization-scope-decode", decode_allowance::<R>(batch)?)?;
        for row in R::decode(batch)? {
            self.loaded
                .insert(&mut self.charge, (R::NAME, *row.id().bytes()))?;
            for reference in row.references() {
                self.wanted
                    .insert(&mut self.charge, (reference.target, reference.key))?;
            }
            rows.insert(row)?;
        }
        Ok(())
    }
    async fn seed<R: Record>(
        &mut self,
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        predicate: &str,
        rows: &mut Rows<R>,
    ) -> Result<(), ModelError> {
        if !access.contains::<R>() {
            return Ok(());
        }
        let _input = access.read::<R>()?;
        let table = access.table_at::<R>(None)?;
        let sql = format!("SELECT * FROM \"{table}\" WHERE {predicate}");
        let mut stream = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            self.absorb(&batch, rows)?;
        }
        Ok(())
    }
    async fn fetch<R: Record>(
        &mut self,
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        rows: &mut Rows<R>,
    ) -> Result<bool, ModelError> {
        if !access.contains::<R>() {
            return Ok(false);
        }
        let mut changed = false;
        loop {
            let keys: Vec<_> = self
                .wanted
                .iter()
                .filter(|(name, key)| *name == R::NAME && !self.loaded.contains(&(*name, *key)))
                .take(128)
                .map(|(_, key)| *key)
                .collect();
            if keys.is_empty() {
                break;
            }
            let _keys = self
                .charge
                .budget()
                .expect("closure budget")
                .reserve("normalization-key-transfer", 128 * 256)?;
            let predicate = format!(
                "id IN ({})",
                keys.iter().map(key_literal).collect::<Vec<_>>().join(",")
            );
            self.seed(access, session, &predicate, rows).await?;
            if keys
                .iter()
                .any(|key| !self.loaded.contains(&(R::NAME, *key)))
            {
                return Err(ModelError::Invalid(format!(
                    "normalization scope is missing a required {} premise",
                    R::NAME
                )));
            }
            changed = true;
        }
        Ok(changed)
    }
}
fn key_literal(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("X'{hex}'")
}
struct ModuleKeys {
    stream: datafusion::physical_plan::SendableRecordBatchStream,
    batch: Option<arrow_array::RecordBatch>,
    row: usize,
}
impl ModuleKeys {
    async fn new(session: &datafusion::prelude::SessionContext) -> Result<Self, ModelError> {
        let sql = format!(
            "SELECT source,id FROM {} ORDER BY source,id",
            source::Module::NAME
        );
        let stream = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        Ok(Self {
            stream,
            batch: None,
            row: 0,
        })
    }
    async fn next(&mut self) -> Result<Option<([u8; 16], [u8; 16])>, ModelError> {
        loop {
            if let Some(batch) = &self.batch
                && self.row < batch.num_rows()
            {
                let key = |column: usize| {
                    batch
                        .column(column)
                        .as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                        .ok_or(ModelError::Schema("module key stream"))?
                        .value(self.row)
                        .try_into()
                        .map_err(ModelError::codec)
                };
                let result = (key(0)?, key(1)?);
                self.row += 1;
                return Ok(Some(result));
            }
            self.batch = self.stream.try_next().await.map_err(ModelError::codec)?;
            self.row = 0;
            if self.batch.is_none() {
                return Ok(None);
            }
        }
    }
}
async fn entity_closure(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    seeds: &std::collections::BTreeMap<&'static str, String>,
) -> Result<EntityData, ModelError> {
    let mut data = EntityData::new(budget);
    let mut closure = PremiseClosure::new(budget);
    macro_rules! seed_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(if let Some(predicate) = seeds.get(<$ty>::NAME) { closure.seed(access, session, predicate, &mut data.$field).await?; })* }; }
    lctx_model::normalized_entity_inputs!(seed_inputs);
    loop {
        let mut changed = false;
        macro_rules! fetch_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(if !matches!(stringify!($field), "symbols" | "terms" | "places") { changed |= closure.fetch(access, session, &mut data.$field).await?; })* }; }
        lctx_model::normalized_entity_inputs!(fetch_inputs);
        if !changed {
            break;
        }
    }
    Ok(data)
}
fn entity_symbol_seeds(
    symbols: String,
    public: Option<String>,
) -> std::collections::BTreeMap<&'static str, String> {
    use lctx_model::domain::{calls::*, declarations::*, symbols::*, types::*};
    let mut seeds = std::collections::BTreeMap::new();
    let selected = format!("SELECT id FROM {} WHERE {symbols}", ProviderSymbol::NAME);
    let declarations = format!(
        "SELECT id FROM {} WHERE symbol IN ({selected})",
        SymbolDeclaration::NAME
    );
    let signatures = format!(
        "SELECT id FROM {} WHERE symbol IN ({selected})",
        Signature::NAME
    );
    let parameters = format!(
        "SELECT id FROM {} WHERE signature IN ({signatures})",
        SignatureParameter::NAME
    );
    seeds.insert(ProviderSymbol::NAME, symbols);
    seeds.insert(SymbolDeclaration::NAME, format!("symbol IN ({selected})"));
    seeds.insert(
        SymbolDeclarationSupport::NAME,
        format!("assertion IN ({declarations})"),
    );
    seeds.insert(
        FunctionTraitObservation::NAME,
        format!("symbol IN ({selected})"),
    );
    seeds.insert(
        ClassTraitObservation::NAME,
        format!("symbol IN ({selected})"),
    );
    if public.is_none() {
        seeds.insert(Signature::NAME, format!("symbol IN ({selected})"));
        seeds.insert(
            SignatureParameter::NAME,
            format!("signature IN ({signatures})"),
        );
        seeds.insert(
            ParameterDeclaration::NAME,
            format!("parameter IN ({parameters})"),
        );
        seeds.insert(
            RecordFieldObservation::NAME,
            format!("class IN ({selected})"),
        );
    }
    if let Some(public) = public {
        let names = format!(
            "SELECT id FROM {} WHERE {public}",
            PublicNameObservation::NAME
        );
        let observations = format!(
            "SELECT id FROM {} WHERE symbol IN ({selected})",
            SymbolObservation::NAME
        );
        let enums = format!(
            "SELECT e.id FROM {} e JOIN {} p ON e.access=p.access JOIN {} eq ON eq.id=e.qualification JOIN {} pq ON pq.id=p.qualification WHERE p.id IN ({names}) AND eq.context=pq.context AND eq.scope=pq.scope",
            ExportEnumerationObservation::NAME,
            PublicNameObservation::NAME,
            assertion::AssertionQualification::NAME,
            assertion::AssertionQualification::NAME
        );
        seeds.insert(PublicNameObservation::NAME, public);
        seeds.insert(PublicNameSupport::NAME, format!("assertion IN ({names})"));
        seeds.insert(SymbolObservation::NAME, format!("symbol IN ({selected})"));
        seeds.insert(
            SymbolSupport::NAME,
            format!("assertion IN ({observations})"),
        );
        seeds.insert(
            ExportEnumerationObservation::NAME,
            format!("id IN ({enums})"),
        );
        seeds.insert(
            ExportEnumerationSupport::NAME,
            format!("assertion IN ({enums})"),
        );
    }
    seeds
}
async fn emit_entities(
    output: &ProducerOutput,
    rows: &entity_normalization::EntityOutput,
    public_only: bool,
) -> Result<(), ModelError> {
    macro_rules! write { ($($field:ident: $ty:ty,)*) => { $(if !public_only || matches!(stringify!($field), "exposures" | "public_enumerations" | "exposure_candidates") { for row in rows.$field.iter() { output.push(row.clone()).await?; } })* }; }
    lctx_model::normalized_entity_outputs!(write);
    Ok(())
}

pub async fn entities(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{
        calls::*, lexical::BindingEvent, normalized::entities::*, source::*, symbols::*,
        syntax::ClassFieldSyntaxObservation,
    };
    let session = access.session(runtime).await?;
    macro_rules! declare { ($($field:ident: $ty:ty,)*) => { $(output.declare::<$ty>()?;)* }; }
    lctx_model::normalized_entity_outputs!(declare);
    let mut owners = entity_normalization::OwnershipSweep::new(runtime.budget());
    // Merge compact module keys with the structural stream. A rich occurrence-side hash join
    // cannot spill when statistics pick the wrong build side; two external sorts can.
    let _occurrences = access.read::<Occurrence>()?;
    let _modules = access.read::<Module>()?;
    let mut modules = ModuleKeys::new(&session).await?;
    let mut pending = modules.next().await?;
    let mut source_module: Option<([u8; 16], Option<Id<Module>>)> = None;
    let sql = format!(
        "SELECT * FROM {} ORDER BY source,structural_path",
        Occurrence::NAME
    );
    let mut stream = crate::sql::query(&session, &sql)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let _decode = runtime.budget().reserve(
            "ownership-stream-decode",
            decode_allowance::<Occurrence>(&batch)?,
        )?;
        for row in Occurrence::decode(&batch)?.iter() {
            if source_module
                .as_ref()
                .is_none_or(|(source, _)| source != row.source.bytes())
            {
                while pending
                    .as_ref()
                    .is_some_and(|(source, _)| source < row.source.bytes())
                {
                    pending = modules.next().await?;
                }
                let mut count = 0usize;
                let mut only = None;
                while let Some((source, module)) = pending
                    && source == *row.source.bytes()
                {
                    count = count
                        .checked_add(1)
                        .ok_or(ModelError::Schema("source module membership count"))?;
                    only = Some(
                        serde_json::from_value(
                            serde_json::to_value(module).map_err(ModelError::codec)?,
                        )
                        .map_err(ModelError::codec)?,
                    );
                    pending = modules.next().await?;
                }
                source_module = Some((*row.source.bytes(), if count == 1 { only } else { None }));
            }
            let module = source_module.as_ref().expect("selected source").1;
            let rows = owners.push(row, module, runtime.budget())?;
            emit_entities(&output, &rows, false).await?;
        }
    }
    drop(stream);
    drop(modules);
    drop(owners);
    // These vocabulary rows have no reducer state or dependency dictionary.
    macro_rules! refs {
        ($ty:ty, $variant:ident, $field:ident) => {{
            let _input = access.read::<$ty>()?;
            let mut stream = crate::sql::query(&session, &format!("SELECT * FROM {}", <$ty>::NAME))
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                let _decode = runtime
                    .budget()
                    .reserve("entity-vocabulary-decode", decode_allowance::<$ty>(&batch)?)?;
                for row in <$ty>::decode(&batch)? {
                    output
                        .push(EntityRef::$variant { $field: row.id() })
                        .await?;
                }
            }
        }};
    }
    refs!(Module, Module, module);
    refs!(types::TypeTerm, Type, term);
    refs!(value::Place, Place, place);
    for (relation, kernel) in [
        (
            ProviderSymbol::NAME,
            entity_normalization::EntityKernel::Symbol,
        ),
        (
            ClassFieldSyntaxObservation::NAME,
            entity_normalization::EntityKernel::SyntaxField,
        ),
        (
            PublicNameObservation::NAME,
            entity_normalization::EntityKernel::Public,
        ),
        (
            ExportEnumerationObservation::NAME,
            entity_normalization::EntityKernel::Enumeration,
        ),
    ] {
        let mut roots =
            crate::sql::query(&session, &format!("SELECT id FROM {relation} ORDER BY id"))
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            let keys = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("entity scope keys"))?;
            for index in 0..keys.len() {
                let key = key_literal(&keys.value(index).try_into().map_err(ModelError::codec)?);
                let seeds = match kernel {
                    entity_normalization::EntityKernel::Symbol => {
                        entity_symbol_seeds(format!("id={key}"), None)
                    }
                    entity_normalization::EntityKernel::Public => {
                        let symbols = format!(
                            "id IN (SELECT s.id FROM {} s JOIN {} p ON p.id={key} JOIN {} q ON q.id=p.qualification JOIN {} origin ON origin.id=p.origin WHERE s.context=q.context AND s.module=origin.traced_module AND s.name=origin.traced_name)",
                            ProviderSymbol::NAME,
                            PublicNameObservation::NAME,
                            assertion::AssertionQualification::NAME,
                            ExportOrigin::NAME
                        );
                        entity_symbol_seeds(symbols, Some(format!("id={key}")))
                    }
                    entity_normalization::EntityKernel::Enumeration => {
                        let mut seeds = std::collections::BTreeMap::new();
                        seeds.insert(ExportEnumerationObservation::NAME, format!("id={key}"));
                        seeds.insert(ExportEnumerationSupport::NAME, format!("assertion={key}"));
                        seeds
                    }
                    entity_normalization::EntityKernel::SyntaxField => {
                        let mut seeds = std::collections::BTreeMap::new();
                        seeds.insert(ClassFieldSyntaxObservation::NAME, format!("id={key}"));
                        seeds.insert(
                            BindingEvent::NAME,
                            format!(
                                "site IN (SELECT target FROM {} WHERE id={key})",
                                ClassFieldSyntaxObservation::NAME
                            ),
                        );
                        seeds
                    }
                };
                let data = entity_closure(&access, &session, runtime.budget(), &seeds).await?;
                let rows =
                    entity_normalization::normalize_scope(data.inputs(), kernel, runtime.budget())?;
                drop(data);
                emit_entities(
                    &output,
                    &rows,
                    matches!(
                        kernel,
                        entity_normalization::EntityKernel::Public
                            | entity_normalization::EntityKernel::Enumeration
                    ),
                )
                .await?;
            }
        }
    }
    drop(session);
    output.finish(ProviderOutcome::Complete).await
}

async fn relation_close(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    closure: &mut PremiseClosure,
    data: &mut lctx_model::domain::normalized::relation_normalization::RelationData,
) -> Result<(), ModelError> {
    loop {
        let mut changed = false;
        macro_rules! fetch_facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(changed |= closure.fetch(access, session, &mut data.facts.$field).await?;)* }; }
        lctx_model::normalized_entity_inputs!(fetch_facts);
        macro_rules! fetch_entities { ($($field:ident: $ty:ty,)*) => { $(changed |= closure.fetch(access, session, &mut data.entities.$field).await?;)* }; }
        lctx_model::normalized_entity_outputs!(fetch_entities);
        macro_rules! fetch_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(changed |= closure.fetch(access, session, &mut data.$field).await?;)* }; }
        lctx_model::normalized_relation_inputs!(fetch_inputs);
        // Every referenced native symbol has one total N1 correspondence. The reverse lookup is
        // by exact selected symbols; no global resolution or EntityRef dictionary is retained.
        let unresolved: Vec<_> = data
            .facts
            .symbols
            .iter()
            .filter(|symbol| {
                !data
                    .entities
                    .resolutions
                    .iter()
                    .any(|resolution| resolution.symbol == symbol.id())
            })
            .take(128)
            .map(|symbol| key_literal(symbol.id().bytes()))
            .collect();
        if !unresolved.is_empty() {
            let before = data.entities.resolutions.len();
            closure
                .seed(
                    access,
                    session,
                    &format!("symbol IN ({})", unresolved.join(",")),
                    &mut data.entities.resolutions,
                )
                .await?;
            if data.entities.resolutions.len() == before {
                return Err(ModelError::Invalid(
                    "scoped relation requires total symbol correspondence".into(),
                ));
            }
            changed = true;
        }
        if !changed {
            break;
        }
    }
    Ok(())
}
async fn relation_scope(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    root: &'static str,
    key: &str,
    kernel: lctx_model::domain::normalized::relation_normalization::RelationKernel,
) -> Result<lctx_model::domain::normalized::relation_normalization::RelationData, ModelError> {
    use lctx_model::domain::{
        attribution::*,
        calls::*,
        lexical::*,
        normalized::{
            entities::*,
            relation_normalization::{RelationData, RelationKernel},
        },
        ruff::*,
        source::*,
        symbols::*,
        syntax::*,
        types::*,
    };
    let mut data = RelationData::new(budget);
    let mut closure = PremiseClosure::new(budget);
    let mut seeds = std::collections::BTreeMap::<&'static str, String>::new();
    seeds.insert(root, format!("id={key}"));
    let qtable = assertion::AssertionQualification::NAME;
    let same = |_candidate: &str| {
        format!(
            "qualification IN (SELECT id FROM {qtable} WHERE context IN (SELECT q.context FROM {qtable} q JOIN {root} root ON root.qualification=q.id WHERE root.id={key}))"
        )
    };
    match kernel {
        RelationKernel::Reference => {
            seeds.insert(
                LexicalResolution::NAME,
                format!(
                    "read IN (SELECT read FROM {root} WHERE id={key}) AND {}",
                    same(LexicalResolution::NAME)
                ),
            );
            let native = format!(
                "subject IN (SELECT read FROM {root} WHERE id={key}) AND {}",
                same(RuffContextObservation::NAME)
            );
            seeds.insert(RuffContextObservation::NAME, native.clone());
            let contexts = format!(
                "SELECT id FROM {} WHERE {native}",
                RuffContextObservation::NAME
            );
            seeds.insert(
                RuffContextSupport::NAME,
                format!("assertion IN ({contexts})"),
            );
            let bindings = format!(
                "event IN (SELECT final_binding FROM {} WHERE {native}) AND {}",
                RuffContextObservation::NAME,
                same(RuffBindingObservation::NAME)
            );
            seeds.insert(RuffBindingObservation::NAME, bindings.clone());
            seeds.insert(
                RuffBindingSupport::NAME,
                format!(
                    "assertion IN (SELECT id FROM {} WHERE {bindings})",
                    RuffBindingObservation::NAME
                ),
            );
        }
        RelationKernel::NativeDefinition => {
            seeds.insert(
                DeclarationObservation::NAME,
                format!(
                    "declaration IN (SELECT declaration FROM {root} WHERE id={key}) AND {}",
                    same(DeclarationObservation::NAME)
                ),
            );
            seeds.insert(RuffDefinitionSupport::NAME, format!("assertion={key}"));
        }
        RelationKernel::Import => {
            seeds.insert(
                ModuleResolutionObservation::NAME,
                format!(
                    "alias IN (SELECT alias FROM {root} WHERE id={key}) AND {}",
                    same(ModuleResolutionObservation::NAME)
                ),
            );
        }
        RelationKernel::Ancestry => {
            seeds.insert(
                SymbolSequenceMember::NAME,
                format!("sequence IN (SELECT ancestors FROM {root} WHERE id={key})"),
            );
        }
        RelationKernel::TestOperand => {
            seeds.insert(
                TypeObservation::NAME,
                format!(
                    "subject IN (SELECT operand FROM {root} WHERE id={key}) AND {}",
                    same(TypeObservation::NAME)
                ),
            );
            // Coverage is source-local and may contain multiple provider runs and statuses.
            seeds.insert(ProviderCoverage::NAME, format!("EXISTS (SELECT 1 FROM {root} leaf JOIN {qtable} q ON q.id=leaf.qualification JOIN {} occurrence ON occurrence.id=leaf.test JOIN {} scope ON scope.artifact_artifact=occurrence.source WHERE leaf.id={key} AND {}.context=q.context AND {}.scope=scope.id)", Occurrence::NAME, CoverageScope::NAME, ProviderCoverage::NAME, ProviderCoverage::NAME));
        }
        RelationKernel::Binder => {
            let source = format!(
                "SELECT m.source FROM {root} v JOIN {} pm ON pm.id=v.module JOIN {} m ON m.id=pm.acquired_module WHERE v.id={key}",
                ProviderModule::NAME,
                Module::NAME
            );
            let eligible = [
                SyntaxKind::StmtAssign,
                SyntaxKind::StmtAnnAssign,
                SyntaxKind::StmtTypeAlias,
                SyntaxKind::StmtFunctionDef,
                SyntaxKind::StmtClassDef,
            ]
            .map(|kind| (kind as i16).to_string())
            .join(",");
            let candidates = format!(
                "SELECT o.id FROM {} o JOIN {root} v ON v.id={key} WHERE o.source IN ({source}) AND o.start<=v.anchor_start AND o.end>=v.anchor_end AND o.syntax_kind IN ({eligible})",
                Occurrence::NAME
            );
            seeds.insert(DeclarationObservation::NAME, format!("declaration IN ({candidates}) AND qualification IN (SELECT q.id FROM {qtable} q JOIN {root} v ON v.context=q.context WHERE v.id={key})"));
            // Only the nearest eligible ancestor contributes a Binding premise in the owner
            // kernel. Rank compact ids/paths in SQL before loading any rich binding rows; an
            // outer class/function candidate must not pull in its entire unrelated body.
            let bindings = format!(
                "SELECT event FROM (SELECT b.id AS event,ancestor.id AS ancestor,row_number() OVER (PARTITION BY b.id ORDER BY array_length(ancestor.structural_path) DESC) AS proximity FROM {} b JOIN {} site ON site.id=b.site JOIN {} ancestor ON ancestor.source=site.source AND array_slice(site.structural_path,1,CAST(array_length(ancestor.structural_path) AS BIGINT))=ancestor.structural_path WHERE site.source IN ({source}) AND ancestor.syntax_kind IN ({eligible})) nearest WHERE proximity=1 AND ancestor IN ({candidates})",
                BindingEvent::NAME,
                Occurrence::NAME,
                Occurrence::NAME
            );
            let binding_kinds = [
                BindingEventKind::Assignment,
                BindingEventKind::AnnotationOnly,
                BindingEventKind::TypeAlias,
                BindingEventKind::TypeParam,
            ]
            .map(|kind| (kind as i16).to_string())
            .join(",");
            let observations = format!(
                "event IN ({bindings}) AND kind IN ({binding_kinds}) AND qualification IN (SELECT q.id FROM {qtable} q JOIN {root} v ON v.context=q.context WHERE v.id={key})"
            );
            seeds.insert(BindingObservation::NAME, observations.clone());
            let sites = format!(
                "SELECT site.structural_path FROM {} site JOIN {} event ON event.site=site.id JOIN {} observation ON observation.event=event.id WHERE {observations}",
                Occurrence::NAME,
                BindingEvent::NAME,
                BindingObservation::NAME
            );
            let sites = sites.replace("AND qualification IN", "AND observation.qualification IN");
            let ancestors = format!(
                "SELECT ancestor.id FROM {} ancestor JOIN ({sites}) selected_site ON array_slice(selected_site.structural_path,1,CAST(array_length(ancestor.structural_path) AS BIGINT))=ancestor.structural_path WHERE ancestor.source IN ({source})",
                Occurrence::NAME
            );
            seeds.insert(
                Occurrence::NAME,
                format!("id IN ({candidates}) OR id IN ({ancestors})"),
            );
        }
        RelationKernel::Mention | RelationKernel::Type | RelationKernel::Place => {}
    }
    macro_rules! seed_facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(if let Some(predicate) = seeds.get(<$ty>::NAME) { closure.seed(access, session, predicate, &mut data.facts.$field).await?; })* }; }
    lctx_model::normalized_entity_inputs!(seed_facts);
    macro_rules! seed_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(if let Some(predicate) = seeds.get(<$ty>::NAME) { closure.seed(access, session, predicate, &mut data.$field).await?; })* }; }
    lctx_model::normalized_relation_inputs!(seed_inputs);
    relation_close(access, session, &mut closure, &mut data).await?;
    if matches!(kernel, RelationKernel::Mention) {
        let mention = data
            .mentions
            .iter()
            .next()
            .ok_or(ModelError::Schema("mention root"))?;
        let own_input = lctx_model::domain::normalized::relation_normalization::captured_input(
            &data,
            mention.qualification,
        )?;
        let own = key_literal(own_input.bytes());
        closure
            .seed(
                access,
                session,
                &format!("corpus={own}"),
                &mut data.corpus_libraries,
            )
            .await?;
        let inputs = format!(
            "SELECT {own} AS input UNION SELECT library FROM {} WHERE corpus={own}",
            input::CorpusLibrary::NAME
        );
        let names = format!(
            "SELECT access_path AS name FROM {root} WHERE id={key} AND access_path IS NOT NULL UNION SELECT qualified_name AS name FROM {root} WHERE id={key} AND qualified_name IS NOT NULL"
        );
        let exposures = format!(
            "SELECT exposure.id FROM {} exposure JOIN {} m ON m.id=exposure.access JOIN {} artifact ON artifact.id=m.source JOIN {} public ON public.id=exposure.observation WHERE artifact.input IN ({inputs}) AND concat(m.qualified_name,'.',public.name) IN ({names})",
            PublicExposure::NAME,
            Module::NAME,
            SourceArtifact::NAME,
            PublicNameObservation::NAME
        );
        closure
            .seed(
                access,
                session,
                &format!("id IN ({exposures})"),
                &mut data.entities.exposures,
            )
            .await?;
        closure
            .seed(
                access,
                session,
                &format!("exposure IN ({exposures})"),
                &mut data.entities.exposure_candidates,
            )
            .await?;
        // Narrow qualified-name leaves by module prefix and final spelling, then close all parent
        // observations. Ambiguous parents are retained in full and remain unresolved in the model.
        let module_name =
            "COALESCE(m.qualified_name, pm.bundled_name, pm.namespace_name, pm.unresolved_name)";
        let scope_input = "COALESCE(scope.input_input, artifact.input, module_artifact.input)";
        let observations = format!(
            "SELECT observation.id FROM {} observation JOIN {} s ON s.id=observation.symbol JOIN {} pm ON pm.id=s.module LEFT JOIN {} m ON m.id=pm.acquired_module JOIN {qtable} q ON q.id=observation.qualification JOIN {} scope ON scope.id=q.scope LEFT JOIN {} artifact ON artifact.id=scope.artifact_artifact LEFT JOIN {} scope_module ON scope_module.id=scope.module_module LEFT JOIN {} module_artifact ON module_artifact.id=scope_module.source JOIN {root} mention ON mention.id={key} WHERE {scope_input} IN ({inputs}) AND starts_with(mention.qualified_name,concat({module_name},'.')) AND ends_with(mention.qualified_name,concat('.',s.name))",
            SymbolObservation::NAME,
            ProviderSymbol::NAME,
            ProviderModule::NAME,
            Module::NAME,
            CoverageScope::NAME,
            SourceArtifact::NAME,
            Module::NAME,
            SourceArtifact::NAME
        );
        closure
            .seed(
                access,
                session,
                &format!("id IN ({observations})"),
                &mut data.symbol_observations,
            )
            .await?;
        let mut ancestors = charged::ChargedSet::default();
        let mut ancestor_charge = charged::StateCharge::new(budget, "mention-parent-closure");
        loop {
            let parents: Vec<_> = data
                .symbol_observations
                .iter()
                .filter_map(|row| row.parent)
                .filter(|parent| !ancestors.contains(parent))
                .take(128)
                .collect();
            if parents.is_empty() {
                break;
            }
            for parent in &parents {
                ancestors.insert(&mut ancestor_charge, *parent)?;
            }
            closure
                .seed(
                    access,
                    session,
                    &format!(
                        "symbol IN ({})",
                        parents
                            .iter()
                            .map(|id| key_literal(id.bytes()))
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    &mut data.symbol_observations,
                )
                .await?;
        }
        relation_close(access, session, &mut closure, &mut data).await?;
        let _keys = budget.reserve(
            "mention-selected-key-transfer",
            data.entities.resolutions.len().saturating_mul(128),
        )?;
        let resolutions: Vec<_> = data
            .entities
            .resolutions
            .iter()
            .map(|row| key_literal(row.id().bytes()))
            .collect();
        for chunk in resolutions.chunks(128) {
            closure
                .seed(
                    access,
                    session,
                    &format!("resolution IN ({})", chunk.join(",")),
                    &mut data.entities.candidates,
                )
                .await?;
        }
        relation_close(access, session, &mut closure, &mut data).await?;
    }
    if matches!(kernel, RelationKernel::Reference | RelationKernel::Place) {
        // Membership alternatives at the exact anchor, never an ancestor/name inference.
        let _keys = budget.reserve(
            "anchor-selected-key-transfer",
            data.facts.occurrences.len().saturating_mul(128),
        )?;
        let occurrences: Vec<_> = data
            .facts
            .occurrences
            .iter()
            .map(|row| key_literal(row.id().bytes()))
            .collect();
        for chunk in occurrences.chunks(128) {
            let ids = chunk.join(",");
            let predicate = format!(
                "occurrence_occurrence IN ({ids}) OR callable_callable IN (SELECT id FROM {} WHERE source_declaration IN ({ids})) OR class_class IN (SELECT id FROM {} WHERE source_declaration IN ({ids})) OR parameter_parameter IN (SELECT id FROM {} WHERE source_declaration IN ({ids}))",
                CallableEntity::NAME,
                ClassEntity::NAME,
                ParameterEntity::NAME
            );
            closure
                .seed(access, session, &predicate, &mut data.entities.refs)
                .await?;
        }
        if matches!(kernel, RelationKernel::Place) {
            // Global/field roots can point at module or class field identities without an occurrence.
            for row in data.roots.iter() {
                let entity = match row {
                    value::PlaceRoot::Global { module, .. } => {
                        Some(EntityRef::Module { module: *module })
                    }
                    value::PlaceRoot::Field { class, name } => Some(EntityRef::Field {
                        field: FieldEntity {
                            class: ClassEntity::Source {
                                declaration: *class,
                            }
                            .id(),
                            name: name.as_str().into(),
                        }
                        .id(),
                    }),
                    _ => None,
                };
                if let Some(entity) = entity {
                    closure
                        .wanted
                        .insert(&mut closure.charge, (EntityRef::NAME, *entity.id().bytes()))?;
                }
            }
        }
        relation_close(access, session, &mut closure, &mut data).await?;
    }
    Ok(data)
}

pub async fn relations(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{
        documents::*,
        flow::*,
        lexical::*,
        normalized::relation_normalization::{self, RelationKernel},
        ruff::*,
        symbols::*,
        syntax::*,
        types::*,
        value::*,
    };
    let session = access.session(runtime).await?;
    macro_rules! declare { ($($field:ident: $ty:ty,)*) => { $(output.declare::<$ty>()?;)* }; }
    lctx_model::normalized_relation_outputs!(declare);
    let roots = [
        (ReferenceObservation::NAME, RelationKernel::Reference),
        (
            RuffDefinitionObservation::NAME,
            RelationKernel::NativeDefinition,
        ),
        (ImportAliasObservation::NAME, RelationKernel::Import),
        (ClassAncestryObservation::NAME, RelationKernel::Ancestry),
        (DocumentMentionObservation::NAME, RelationKernel::Mention),
        (TypeTerm::NAME, RelationKernel::Type),
        (TypeVariable::NAME, RelationKernel::Binder),
        (Place::NAME, RelationKernel::Place),
        (FlowTestLeafObservation::NAME, RelationKernel::TestOperand),
    ];
    for (relation, kernel) in roots {
        if relation == FlowTestLeafObservation::NAME
            && !access.contains::<FlowTestLeafObservation>()
        {
            continue;
        }
        let mut stream =
            crate::sql::query(&session, &format!("SELECT id FROM {relation} ORDER BY id"))
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let keys = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("relation scope keys"))?;
            for index in 0..keys.len() {
                let key = key_literal(&keys.value(index).try_into().map_err(ModelError::codec)?);
                let data =
                    relation_scope(&access, &session, runtime.budget(), relation, &key, kernel)
                        .await?;
                let rows =
                    relation_normalization::normalize_scope(&data, kernel, runtime.budget())?;
                drop(data);
                macro_rules! write { ($($field:ident: $ty:ty,)*) => { $(for row in rows.$field.iter() { output.push(row.clone()).await?; })* }; }
                lctx_model::normalized_relation_outputs!(write);
            }
        }
    }
    drop(session);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn callables(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{
        calls::Signature, normalized::callable_normalization, normalized::entities::CallableEntity,
        types::NativeOverloadObservation,
    };
    macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;)*};}
    lctx_model::normalized_callable_outputs!(declare);
    let session = access.session(runtime).await?;
    for (kernel, root) in [
        (callable_scope::Kernel::Callable, CallableEntity::NAME),
        (callable_scope::Kernel::Signature, Signature::NAME),
        (
            callable_scope::Kernel::Overload,
            NativeOverloadObservation::NAME,
        ),
    ] {
        let scopes = callable_scope::CallableScopes::prepare(
            &access,
            &session,
            model,
            runtime.budget(),
            kernel,
        )
        .await?;
        let declaration = callable_normalization::CallableData::validation_inputs()
            .into_iter()
            .find(|input| input.name() == root)
            .ok_or(ModelError::Schema("callable root input"))?;
        let table = access.table_for(&declaration)?;
        let mut stream =
            crate::sql::query(&session, &format!("SELECT id FROM \"{table}\" ORDER BY id"))
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let keys = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("callable root projection"))?;
            for index in 0..keys.len() {
                let rows = match kernel {
                    callable_scope::Kernel::Callable => {
                        let key: Id<CallableEntity> = callable_scope::nominal(keys.value(index))?;
                        let data = scopes.data(&access, key, runtime.budget()).await?;
                        let rows = callable_normalization::normalize_callable(
                            &data,
                            key,
                            runtime.budget(),
                        )?;
                        drop(data);
                        rows
                    }
                    callable_scope::Kernel::Signature => {
                        let key: Id<Signature> = callable_scope::nominal(keys.value(index))?;
                        let data = scopes.data(&access, key, runtime.budget()).await?;
                        let rows = callable_normalization::normalize_signature(
                            &data,
                            key,
                            runtime.budget(),
                        )?;
                        drop(data);
                        rows
                    }
                    callable_scope::Kernel::Overload => {
                        let key: Id<NativeOverloadObservation> =
                            callable_scope::nominal(keys.value(index))?;
                        let data = scopes.data(&access, key, runtime.budget()).await?;
                        let rows = callable_normalization::normalize_overload(
                            &data,
                            key,
                            runtime.budget(),
                        )?;
                        drop(data);
                        rows
                    }
                };
                macro_rules! emit {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter() {output.push(row.clone()).await?;})*};}
                lctx_model::normalized_callable_outputs!(emit);
                drop(rows);
            }
        }
        drop(scopes);
    }
    drop(session);
    output.finish(ProviderOutcome::Complete).await
}

/// Dispatch target for the model's typed necessary Callables admission scope.
pub async fn validate_callables(
    invariant: &Invariant,
    tables: Vec<crate::consumed_rows::ClosureTable>,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    cancellation: &crate::workspace::Cancellation,
) -> Result<(), ModelError> {
    callable_scope::validate_callables(invariant, tables, session, budget, cancellation).await
}

/// Complete callable metadata is produced one actual assessment, initializer or class at a time.
pub async fn aspects(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::callable_aspects::{self, AspectKernel};
    let session = access.session(runtime).await?;
    let inputs = callable_aspects::scoped_inputs();
    macro_rules! acknowledge {($($field:ident:$ty:ty,)*)=>{$(let binding=inputs.iter().find(|input|input.type_id()==std::any::TypeId::of::<$ty>()).ok_or(ModelError::Schema("aspect source binding"))?;access.read_at::<$ty>(binding.prefix())?;)*};}
    lctx_model::callable_aspect_inputs!(acknowledge);
    let tables = inputs
        .iter()
        .map(|input| {
            Ok(crate::consumed_rows::ClosureTable {
                relation: model
                    .relation(input.name())
                    .ok_or(ModelError::Schema(input.name()))?
                    .clone(),
                alias: access.table_for(input)?,
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    let prepared = crate::scoped_aspects::AspectScopes::prepare(
        inputs,
        tables,
        &callable_aspects::aspect_scope(),
        model,
        &session,
        runtime.budget(),
    )
    .await?;
    macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;)*};}
    lctx_model::callable_aspect_outputs!(declare);
    for (index, root) in prepared.roots.iter().enumerate() {
        let filter = if index == 2 {
            format!(" WHERE kind={}", syntax::DeclarationKind::Class as i16)
        } else {
            String::new()
        };
        let mut roots = crate::sql::query(
            &session,
            &format!(
                "SELECT id FROM {}{filter} ORDER BY id",
                crate::consumed_rows::identifier(&prepared.root_tables[index])
            ),
        )
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            runtime.cancellation().check()?;
            let keys = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("aspect owner keys"))?;
            for row in 0..keys.len() {
                runtime.cancellation().check()?;
                let id: [u8; 16] = keys.value(row).try_into().map_err(ModelError::codec)?;
                let scoped = prepared
                    .edges
                    .grain(
                        *root,
                        &crate::scoped_admission::root_predicate(&[id]),
                        runtime.budget(),
                    )
                    .await?;
                let (data, prior) =
                    crate::scoped_aspects::load_data(&scoped, &prepared.inputs, runtime.budget())
                        .await?;
                drop(prior);
                let kernel = crate::scoped_aspects::kernel(index, id)?;
                let rows = callable_aspects::normalize_scope(&data, kernel, runtime.budget())?;
                drop(data);
                drop(scoped);
                // A class computes its field defaults only as scratch for source-field policy.
                // The field owner emits those canonical rows exactly once through its own grain.
                macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(if !matches!(kernel,AspectKernel::Class(_)) || !matches!(stringify!($field),"sources"|"aspects"|"defaults"|"fields") {for row in rows.$field.iter() {output.push(row.clone()).await?;}})*};}
                lctx_model::callable_aspect_outputs!(write);
            }
        }
    }
    drop(prepared);
    drop(session);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn receivers(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    receivers_produced(access, output, runtime, model)
        .await
        .map(|_| ())
}
pub(crate) async fn receivers_produced(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<ProducedNormalization<normalized::receiver::VerifiedReceivers>, ModelError> {
    use lctx_model::domain::normalized::receiver;
    let premises = selected_premises(&access, receiver::ReceiverData::validation_inputs())?;
    let mut verified = receiver::normalize_produced(
        &receiver::ReceiverData::new(runtime.budget()),
        runtime.budget(),
    )?
    .1;
    let profile = access.profile();
    let session = access.session(runtime).await?;
    let scopes =
        receiver_scope::ReceiverScopes::prepare(&access, &session, model, runtime.budget()).await?;
    let declaration = ValidationInput::of::<calls::CallTarget>(&["id"]);
    let _permit = access.read_at::<calls::CallTarget>(declaration.prefix())?;
    let table = access.table_for(&declaration)?;
    macro_rules! declare {($($field:ident: $ty:ty,)*) => {$(output.declare::<$ty>()?;)*};}
    lctx_model::normalized_receiver_outputs!(declare);
    let mut stream = crate::sql::query(
        &session,
        &format!(
            "SELECT * FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&table)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let mut roots = Rows::<calls::CallTarget>::new(runtime.budget());
        roots.decode(&batch)?;
        for target in roots.iter() {
            let data = scopes.data(&access, target.id(), runtime.budget()).await?;
            let (rows, authority) = crate::stage_runtime::borrowed_cpu(access.name(), || {
                receiver::normalize_target_produced(&data, target.id(), runtime.budget())
            })?;
            verified.append(authority)?;
            macro_rules! write {($($field:ident: $ty:ty,)*) => {$(for row in rows.$field.iter() {output.push(row.clone()).await?;})*};}
            lctx_model::normalized_receiver_outputs!(write);
            tokio::task::yield_now().await;
        }
    }
    output.finish(ProviderOutcome::Complete).await?;
    let outputs = runtime.inputs(
        "receiver-produced-authority",
        profile,
        receiver::relations()
            .into_iter()
            .map(|relation| relation.name()),
    )?;
    Ok(ProducedNormalization {
        premises,
        outputs,
        value: verified,
    })
}

async fn emit_event_rows(
    rows: &normalized::event_normalization::EventOutput,
    output: &ProducerOutput,
) -> Result<(), ModelError> {
    macro_rules! write {($($field:ident:$record:ty,)*)=>{$(for row in rows.$field.iter(){output.push(row.clone()).await?;})*};}
    lctx_model::normalized_event_outputs!(write);
    Ok(())
}

pub(crate) async fn events_produced(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    receivers: &ProducedNormalization<normalized::receiver::VerifiedReceivers>,
) -> Result<ProducedNormalization<normalized::event_normalization::VerifiedEvents>, ModelError> {
    use normalized::event_normalization::{self, EventData, EventKey};
    let receivers = receivers.borrow(&access, runtime)?;
    let premises = selected_premises(&access, EventData::validation_inputs())?;
    let profile = access.profile();
    let session = access.session(runtime).await?;
    let scopes =
        call_scope::CallScopes::prepare(&access, &session, model, runtime.budget(), false).await?;
    let mut verified = event_normalization::normalize_events_produced(
        &EventData::new(runtime.budget()),
        receivers,
        runtime.budget(),
    )?
    .1;
    let mut seen: charged::ChargedSet<EventKey> = Default::default();
    let mut charge = charged::StateCharge::new(runtime.budget(), "event-root-keys");
    macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(output.declare::<$ty>()?;)*};}
    lctx_model::normalized_event_outputs!(declare);
    // Every provider site, target and resolution is a root, including unsupported/empty sets.
    // Compact deduplication roots the complete qualified domain exactly once.
    macro_rules! roots {($ty:ty) => {{
        let input = ValidationInput::of::<$ty>(&["id"]);
        let _permit = access.read_at::<$ty>(input.prefix())?;
        let table = access.table_for(&input)?;
        let qualifications = access.table_for(&EventData::validation_inputs().into_iter().find(|input| input.type_id() == std::any::TypeId::of::<assertion::AssertionQualification>()).expect("event qualification declaration"))?;
        let sql = format!("SELECT r.*,q.context AS root_context FROM {} r JOIN {} q ON q.id=r.qualification ORDER BY r.id", crate::consumed_rows::identifier(&table), crate::consumed_rows::identifier(&qualifications));
        let mut stream = crate::sql::query(&session, &sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let mut rows = Rows::<$ty>::new(runtime.budget());
            // Decode the record projection without the extra context column.
            let projection = batch.project(&(0..batch.num_columns()-1).collect::<Vec<_>>()).map_err(ModelError::codec)?;
            rows.decode(&projection)?;
            let contexts = batch.column(batch.num_columns()-1).as_any().downcast_ref::<arrow_array::FixedSizeBinaryArray>().ok_or_else(|| ModelError::Invalid("event context key has another Arrow type".into()))?;
            for (ordinal, root) in rows.iter().enumerate() {
                let context: Id<attribution::AnalysisContext> = serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new(contexts.value(ordinal).iter().copied())).map_err(ModelError::codec)?;
                let key = (root.site, root.origin, context);
                if !seen.insert(&mut charge, key)? { continue; }
                let data = scopes.event_data(&access, root.id(), runtime.budget()).await?;
                let (rows, authority) = crate::stage_runtime::borrowed_cpu(access.name(), || event_normalization::normalize_event_produced(&data, key, receivers, runtime.budget()))?;
                verified.append(authority)?;
                emit_event_rows(&rows,&output).await?;
                tokio::task::yield_now().await;
            }
        }
    }};}
    roots!(calls::ProviderCallSite);
    roots!(calls::CallTarget);
    roots!(calls::CallResolution);
    if access.contains::<flow::FlowValuePathObservation>() {
        let input = ValidationInput::of::<flow::FlowValuePathObservation>(&["id"]);
        let _permit = access.read_at::<flow::FlowValuePathObservation>(input.prefix())?;
        let table = access.table_for(&input)?;
        let mut stream = crate::sql::query(
            &session,
            &format!(
                "SELECT * FROM {} ORDER BY id",
                crate::consumed_rows::identifier(&table)
            ),
        )
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let mut paths = Rows::<flow::FlowValuePathObservation>::new(runtime.budget());
            paths.decode(&batch)?;
            for path in paths.iter() {
                let data = scopes
                    .event_data(&access, path.id(), runtime.budget())
                    .await?;
                let rows = event_normalization::normalize_flow_path(
                    &data,
                    path.id(),
                    &verified,
                    runtime.budget(),
                )?;
                for row in rows.flow_links.iter() {
                    output.push(row.clone()).await?;
                }
            }
        }
    }
    output.finish(ProviderOutcome::Complete).await?;
    let outputs = runtime.inputs(
        "event-produced-authority",
        profile,
        normalized::events::relations()
            .into_iter()
            .map(|relation| relation.name()),
    )?;
    Ok(ProducedNormalization {
        premises,
        outputs,
        value: verified,
    })
}
/// Complete native enumeration authority is a binding-owner premise even when a constructor
/// initializer never becomes an ordinary call event. Decode one symbol's five native families
/// at a time; only compact exact keys/content/counts survive in the application authority.
async fn prepare_enumeration_authority(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    application: &mut normalized::binding_normalization::VerifiedBindings,
    runtime: &Workspace,
) -> Result<(), ModelError> {
    use calls::{
        ProviderSymbol, Signature, SignatureEnumerationMember, SignatureEnumerationObservation,
    };
    use normalized::binding_normalization::BindingData;
    let declared = BindingData::validation_inputs();
    let input = |kind| {
        declared
            .iter()
            .find(|input| input.type_id() == kind)
            .ok_or(ModelError::Schema("binding enumeration input"))
    };
    let table = |kind| {
        access
            .table_for(input(kind)?)
            .map(|name| crate::consumed_rows::identifier(&name))
    };
    let enumerations = table(std::any::TypeId::of::<SignatureEnumerationObservation>())?;
    let members = table(std::any::TypeId::of::<SignatureEnumerationMember>())?;
    let signatures = table(std::any::TypeId::of::<Signature>())?;
    let qualifications = table(std::any::TypeId::of::<assertion::AssertionQualification>())?;
    let symbols = table(std::any::TypeId::of::<ProviderSymbol>())?;
    // The root scan carries only the primitive symbol key, never native record bodies.
    let mut stream = crate::sql::query(
        session,
        &format!("SELECT DISTINCT symbol FROM {enumerations} ORDER BY symbol"),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let _roots = runtime.budget().reserve(
            "binding-native-enumeration-root-batch",
            logical_batch_bytes(&batch)?,
        )?;
        let roots = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("binding enumeration symbol key"))?;
        for index in 0..roots.len() {
            runtime.cancellation().check()?;
            if roots.is_null(index) {
                return Err(ModelError::Schema("binding enumeration symbol key"));
            }
            let key: [u8; 16] = roots
                .value(index)
                .try_into()
                .map_err(|_| ModelError::Schema("binding enumeration symbol key"))?;
            let key = crate::scoped_admission::root_predicate(&[key]);
            let key = key.replacen("id IN", "symbol IN", 1);
            let mut data = BindingData::new(runtime.budget());
            macro_rules! read {
                ($field:ident, $ty:ty, $sql:expr) => {{
                    let input = input(std::any::TypeId::of::<$ty>())?;
                    let permit = access.read_at::<$ty>(input.prefix())?;
                    crate::consumed_rows::stream_query_at(
                        &permit,
                        input,
                        session,
                        &$sql,
                        |_, batch| data.$field.decode(batch),
                    )
                    .await?;
                }};
            }
            read!(
                signature_enumerations,
                SignatureEnumerationObservation,
                format!("SELECT * FROM {enumerations} WHERE {key}")
            );
            read!(
                signature_enumeration_members,
                SignatureEnumerationMember,
                format!(
                    "SELECT m.* FROM {members} m JOIN {enumerations} e ON e.id=m.enumeration WHERE {}",
                    key.replacen("symbol IN", "e.symbol IN", 1)
                )
            );
            // All native signatures for this symbol are required, including an omitted member:
            // the canonical validator compares the entire qualification/role variant domain.
            read!(
                signatures,
                Signature,
                format!("SELECT * FROM {signatures} WHERE {key}")
            );
            read!(
                qualifications,
                assertion::AssertionQualification,
                format!(
                    "SELECT * FROM {qualifications} WHERE id IN (SELECT qualification FROM {enumerations} WHERE {key} UNION SELECT qualification FROM {signatures} WHERE {key})"
                )
            );
            read!(
                symbols,
                ProviderSymbol,
                format!(
                    "SELECT * FROM {symbols} WHERE {}",
                    key.replacen("symbol IN", "id IN", 1)
                )
            );
            application.admit_enumerations(&data, runtime.budget())?;
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}

pub(crate) async fn bindings_prepared(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    receivers: &ProducedNormalization<normalized::receiver::VerifiedReceivers>,
    events: &ProducedNormalization<normalized::event_normalization::VerifiedEvents>,
) -> Result<normalized::binding_normalization::VerifiedBindings, ModelError> {
    use normalized::binding_normalization::{self, BindingData};
    let receivers = receivers.borrow(&access, runtime)?;
    let events = events.borrow(&access, runtime)?;
    let session = access.session(runtime).await?;
    let scopes =
        call_scope::CallScopes::prepare(&access, &session, model, runtime.budget(), true).await?;
    let mut application = binding_normalization::normalize_produced(
        &BindingData::new(runtime.budget()),
        receivers,
        events,
        runtime.budget(),
    )?
    .1;
    macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(output.declare::<$ty>()?;)*};}
    lctx_model::normalized_binding_outputs!(declare);
    let input = ValidationInput::of::<normalized::events::NormalizedCallEvent>(&["id"]);
    let _permit = access.read_at::<normalized::events::NormalizedCallEvent>(input.prefix())?;
    let table = access.table_for(&input)?;
    let mut stream = crate::sql::query(
        &session,
        &format!(
            "SELECT * FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&table)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let mut roots = Rows::<normalized::events::NormalizedCallEvent>::new(runtime.budget());
        roots.decode(&batch)?;
        for event in roots.iter() {
            let data = scopes
                .binding_data(&access, event.id(), runtime.budget())
                .await?;
            let (rows, verified) = crate::stage_runtime::borrowed_cpu(access.name(), || {
                binding_normalization::normalize_event_produced(
                    &data,
                    event.id(),
                    receivers,
                    events,
                    runtime.budget(),
                )
            })?;
            application.append(verified)?;
            macro_rules! write {($($field:ident:$ty:ty,)*) => {$(for row in rows.$field.iter() { output.push(row.clone()).await?; })*};}
            lctx_model::normalized_binding_outputs!(write);
            tokio::task::yield_now().await;
        }
    }
    prepare_enumeration_authority(&access, &session, &mut application, runtime).await?;
    output.finish(ProviderOutcome::Complete).await?;
    Ok(application)
}

/// Generation-local computational snapshots are built once, after their canonical inputs finish.
pub async fn projections(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::projection::{
        compact::compact_columns,
        normalization::{CompactProjectionData, ProjectionData},
    };
    let session = access.session(runtime).await?;
    let mut data = CompactProjectionData::new(runtime.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*) => {$({
        let input=ProjectionData::validation_inputs().into_iter().find(|input|input.type_id()==std::any::TypeId::of::<$ty>()).expect("projection declared input");
        let permit=access.read_at::<$ty>(input.prefix())?;
        let table=access.table_for(&input)?;
        let columns=compact_columns(<$ty>::NAME).map(|columns|columns.iter().map(|column|crate::consumed_rows::identifier(column)).collect::<Vec<_>>().join(",")).unwrap_or_else(||"*".into());
        let sql=format!("SELECT {columns} FROM {} ORDER BY id",crate::consumed_rows::identifier(&table));
        crate::consumed_rows::stream_query_at(&permit,&input,&session,&sql,|_,batch|data.visit(<$ty>::NAME,batch).map(|_|())).await?;
    })*};}
    lctx_model::projection_inputs!(read);
    let prepared = data.prepare(runtime.budget())?;
    macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(output.declare::<$ty>()?;)*};}
    lctx_model::projection_outputs!(declare);
    for key in prepared.keys() {
        let rows = crate::stage_runtime::borrowed_cpu(access.name(), || {
            prepared.produce(key, runtime.budget())
        })?;
        macro_rules! write {($($field:ident:$ty:ty,)*) => {$(for row in rows.$field.iter(){output.push(row.clone()).await?;})*};}
        lctx_model::projection_outputs!(write);
        drop(rows);
        tokio::task::yield_now().await;
    }
    output.finish(ProviderOutcome::Complete).await
}

pub(crate) mod pipeline;

/// Assemble exact normalized scope outcomes over the private, admitted facts checkpoint.
pub async fn coverage(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{normalized::coverage::*, source::SourceArtifact};
    let sources = access.snapshots().collect::<Vec<_>>();
    let profile = access.profile();
    let evidence = runtime.facts_availability(profile)?;
    let session = access.session(runtime).await?;
    let _permit = access.read::<SourceArtifact>()?;
    let table = access.table_at::<SourceArtifact>(None)?;
    let mut artifacts = ArtifactInputIndex::new(runtime.budget());
    // Coverage retains only the primitive artifact/input correspondence. Paths and artifact
    // payload metadata are excluded by the physical projection before any model decoding.
    let mut stream = crate::sql::query(
        &session,
        &format!("SELECT id,input FROM \"{table}\" ORDER BY id"),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        runtime.cancellation().check()?;
        artifacts.ingest(&batch)?;
        tokio::task::yield_now().await;
    }
    drop(stream);
    drop(session);
    output.declare::<NormalizationComputation>()?;
    output.declare::<NormalizationOutputReceipt>()?;
    output.declare::<NormalizationCoverage>()?;
    output.declare::<NormalizationPremise>()?;
    output.declare::<NormalizationEvidenceSet>()?;
    output.declare::<NormalizationEvidenceMember>()?;
    let prepared = CoveragePreparation::new(&evidence, &artifacts, runtime.budget())?;
    for record in prepared.evidence_records() {
        runtime.cancellation().check()?;
        let (set, member) = record?;
        output.push(set).await?;
        output.push(member).await?;
    }
    for capability in Capability::ALL {
        let stage = capability.producer(profile);
        let mut computation = NormalizationComputation {
            capability,
            policy: lctx_model::domain::normalized::policy_revision(),
            producer: stage.name.into(),
            declaration: stage.digest(),
            profile: profile.name().into(),
            availability: EvidenceAvailability::NoScope,
        };
        let mut aggregate = CoverageAggregate::default();
        for (scope, context) in prepared.scopes(capability) {
            runtime.cancellation().check()?;
            let scoped = prepared.outcome(capability, scope, context)?;
            aggregate.include(scoped.availability);
            let row = NormalizationCoverage {
                computation: computation.id(),
                scope,
                context,
                availability: scoped.availability,
            };
            let outcome = row.id();
            output.push(row).await?;
            for premise in &scoped.premises {
                output
                    .push(NormalizationPremise {
                        outcome,
                        premise: premise.id(),
                    })
                    .await?;
            }
        }
        computation.availability = aggregate.finish(capability, profile);
        let id = computation.id();
        output.push(computation).await?;
        for relation in &stage.outputs {
            let source = sources
                .iter()
                .find(|source| {
                    source.relation() == relation.name() && source.producer() == stage.name
                })
                .ok_or_else(|| {
                    ModelError::Frontier(
                        "normalization output has no completed producer receipt".into(),
                    )
                })?;
            output
                .push(NormalizationOutputReceipt {
                    computation: id,
                    relation: relation.name().into(),
                    rows: source.rows(),
                    content: source.content(),
                })
                .await?;
        }
    }
    output.finish(ProviderOutcome::Complete).await
}
