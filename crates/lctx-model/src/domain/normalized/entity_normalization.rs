//! Pure correspondence operations. Input selection/joins and store effects belong to the stage.
use super::{Rows, entities::*, policy_revision};
use crate::domain::{
    assertion::AssertionQualification,
    calls::*,
    charged::{ChargedMap, ChargedSet, StateCharge},
    declarations::*,
    occurrence_owner::OwnerTable,
    resources::ResourceBudget,
    source::*,
    symbols::*,
    *,
};

macro_rules! inputs {
    ($($field:ident: $ty:ty => $family:ident,)*) => {
        pub struct EntityInputs<'a> { $(pub $field: &'a Rows<$ty>,)* }
        pub struct EntityData { $(pub $field: Rows<$ty>,)* }
        impl EntityData {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn inputs(&self) -> EntityInputs<'_> { EntityInputs { $($field: &self.$field,)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })*
                Ok(false)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { super::facts_inputs(vec![$(ValidationInput::of::<$ty>(&["id"]),)*]) }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::completed::<$ty>()
                .availability(attribution::FactFamily::$family, stages::AvailabilityPolicy::ObserveAvailability),)*] }
        }
    }
}
crate::normalized_entity_inputs!(inputs);
macro_rules! outputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct EntityOutput { $(pub $field: Rows<$ty>,)* }
        impl EntityOutput {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })*
                Ok(false)
            }
            pub fn matches(&self, expected: &Self) -> Result<(), ModelError> {
                $(if !self.$field.same(&expected.$field) { return Err(ModelError::Invalid(format!("normalized entity closure differs: {}; {}", <$ty>::NAME,self.$field.difference(&expected.$field)))); })*
                Ok(())
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
        }
    }
}
crate::normalized_entity_outputs!(outputs);
impl EntityOutput {
    fn source(
        &mut self,
        occurrence: &Occurrence,
        module: Option<Id<Module>>,
    ) -> Result<Id<EntityRef>, ModelError> {
        let entity = if let Some(callable) = source_callable(occurrence) {
            EntityRef::Callable {
                callable: self.callables.insert(callable)?,
            }
        } else if occurrence.syntax_kind == SyntaxKind::StmtClassDef {
            EntityRef::Class {
                class: self.classes.insert(ClassEntity::Source {
                    declaration: occurrence.id(),
                })?,
            }
        } else if occurrence.syntax_kind == SyntaxKind::Parameter {
            EntityRef::Parameter {
                parameter: self.parameters.insert(ParameterEntity::Source {
                    declaration: occurrence.id(),
                })?,
            }
        } else if let Some(module) =
            module.filter(|_| occurrence.syntax_kind == SyntaxKind::ModModule)
        {
            EntityRef::Module { module }
        } else {
            EntityRef::Occurrence {
                occurrence: occurrence.id(),
            }
        };
        self.refs.insert(entity)
    }
}
/// Ordered structural ownership sweep. Retains only the active ancestor path, never a source
/// occurrence table. DataFusion supplies `(source, structural_path)` order across scan batches.
pub struct OwnershipSweep {
    frames: crate::domain::charged::ChargedVec<OwnershipFrame>,
    charge: StateCharge,
    previous: Option<(Id<SourceArtifact>, Vec<i32>)>,
}
struct OwnershipFrame {
    source: Id<SourceArtifact>,
    path: Vec<i32>,
    kind: SyntaxKind,
    occurrence: Id<Occurrence>,
    owner: Id<Occurrence>,
    entity: Id<EntityRef>,
    owner_entity: Id<EntityRef>,
}
impl HeapSize for OwnershipFrame {
    fn heap_bytes(&self) -> usize {
        self.path.heap_bytes()
    }
}
impl OwnershipSweep {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            frames: Default::default(),
            charge: StateCharge::new(budget, "entity-owner-stack"),
            previous: None,
        }
    }
    pub fn push(
        &mut self,
        row: &Occurrence,
        module: Option<Id<Module>>,
        budget: &ResourceBudget,
    ) -> Result<EntityOutput, ModelError> {
        row.validate()?;
        if self.previous.as_ref().is_some_and(|(source, path)| {
            (*source, path.as_slice()) >= (row.source, row.structural_path.as_slice())
        }) {
            return Err(ModelError::Invalid(
                "ownership stream is not strictly ordered".into(),
            ));
        }
        if let Some((_, path)) = self.previous.take() {
            self.charge.release(path.heap_bytes());
        }
        self.charge.grow(row.structural_path.heap_bytes())?;
        self.previous = Some((row.source, row.structural_path.clone()));
        while self.frames.last().is_some_and(|frame| {
            frame.source != row.source
                || !row.structural_path.starts_with(&frame.path)
                || frame.path.len() >= row.structural_path.len()
        }) {
            self.frames.take_last(&mut self.charge);
        }
        let mut output = EntityOutput::new(budget);
        let own_entity = if crate::domain::occurrence_owner::owns_body(row.syntax_kind)
            || row.syntax_kind == SyntaxKind::Parameter
            || row.structural_path.len() == 1
        {
            Some(output.source(row, module)?)
        } else {
            None
        };
        let (owner, entity) = match self.frames.last() {
            None if row.structural_path.len() == 1 => (row.id(), own_entity.expect("root entity")),
            None => return Err(missing("occurrence structural path ancestor")),
            Some(parent) => {
                if parent.path.len() + 1 != row.structural_path.len() {
                    return Err(missing("occurrence structural path ancestor"));
                }
                if crate::domain::occurrence_owner::owns_body(parent.kind)
                    && crate::domain::occurrence_owner::is_body_child(parent.kind, row.syntax_kind)
                {
                    (parent.occurrence, parent.entity)
                } else {
                    (parent.owner, parent.owner_entity)
                }
            }
        };
        output.refs.insert(EntityRef::Occurrence {
            occurrence: row.id(),
        })?;
        output.owners.insert(OccurrenceOwnership {
            occurrence: row.id(),
            owner,
            entity,
        })?;
        // The frame needs its declaration entity when it later owns a body, otherwise its owner.
        self.frames.push(
            &mut self.charge,
            OwnershipFrame {
                source: row.source,
                path: row.structural_path.clone(),
                kind: row.syntax_kind,
                occurrence: row.id(),
                owner,
                entity: own_entity.unwrap_or(entity),
                owner_entity: entity,
            },
        )?;
        Ok(output)
    }
}

fn missing(what: &str) -> ModelError {
    ModelError::Invalid(format!("normalization requires {what}"))
}
fn context(
    input: &EntityInputs<'_>,
    id: Id<AssertionQualification>,
) -> Result<Id<crate::domain::attribution::AnalysisContext>, ModelError> {
    Ok(input
        .qualifications
        .get(id)
        .ok_or_else(|| missing("assertion qualification"))?
        .context)
}
fn certain(input: &EntityInputs<'_>, id: Id<AssertionQualification>) -> Result<bool, ModelError> {
    let q = input
        .qualifications
        .get(id)
        .ok_or_else(|| missing("correspondence qualification"))?;
    Ok(q.modality == attribution::Modality::Definite
        && q.approximation == assertion::Approximation::Exact
        && q.condition == conditions::Diagram::always().id())
}

pub fn normalize(
    input: EntityInputs<'_>,
    budget: &ResourceBudget,
) -> Result<EntityOutput, ModelError> {
    let mut output = EntityOutput::new(budget);
    source_entities(&input, &mut output, budget)?;
    symbol_entities(&input, &mut output, budget)?;
    syntax_fields(&input, &mut output, budget)?;
    public_entities(&input, &mut output, budget)?;
    Ok(output)
}

/// A producer scope selects its complete dependency closure before invoking one kernel.
/// These are semantic grains, never pages of a full-source replay.
#[derive(Clone, Copy)]
pub enum EntityKernel {
    Symbol,
    SyntaxField,
    Public,
    Enumeration,
}

pub fn normalize_scope(
    input: EntityInputs<'_>,
    kernel: EntityKernel,
    budget: &ResourceBudget,
) -> Result<EntityOutput, ModelError> {
    let mut output = EntityOutput::new(budget);
    match kernel {
        EntityKernel::Symbol => symbol_entities(&input, &mut output, budget)?,
        EntityKernel::SyntaxField => syntax_fields(&input, &mut output, budget)?,
        EntityKernel::Enumeration => public_entities(&input, &mut output, budget)?,
        EntityKernel::Public => {
            // The public scope carries origin-matched symbols, including ambiguous alternatives.
            symbol_entities(&input, &mut output, budget)?;
            public_entities(&input, &mut output, budget)?;
        }
    }
    Ok(output)
}

fn source_entities(
    input: &EntityInputs<'_>,
    output: &mut EntityOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "entity-correspondence-index");
    let mut by_source: ChargedMap<Id<SourceArtifact>, Vec<&Occurrence>> = Default::default();
    let mut modules: ChargedMap<Id<SourceArtifact>, Vec<Id<Module>>> = Default::default();
    for module in input.modules.iter() {
        modules.update(&mut charge, module.source, |ids| ids.push(module.id()))?;
    }
    for occurrence in input.occurrences.iter() {
        by_source.update(&mut charge, occurrence.source, |rows| rows.push(occurrence))?;
    }
    // The ownership sweep is source-local; headers/defaults/decorators keep their evaluation owner.
    for (source, occurrences) in by_source.iter() {
        let mut source_charge = StateCharge::new(budget, "source-owner-input");
        for row in occurrences {
            source_charge.admit(*row)?;
        }
        let rows: Vec<_> = occurrences.iter().map(|row| (*row).clone()).collect();
        let owners = OwnerTable::build(&rows, budget)?;
        let module = modules
            .get(source)
            .and_then(|ids| (ids.len() == 1).then(|| ids[0]));
        for row in &rows {
            // Declaration identity exists independently of provider correspondence.
            if crate::domain::occurrence_owner::owns_body(row.syntax_kind)
                || row.syntax_kind == SyntaxKind::Parameter
            {
                output.source(row, module)?;
            }
            let owner = owners
                .owner(row.id())
                .ok_or_else(|| missing("occurrence owner"))?;
            let entity = output.source(
                input
                    .occurrences
                    .get(owner)
                    .ok_or_else(|| missing("owner occurrence"))?,
                module,
            )?;
            output.owners.insert(OccurrenceOwnership {
                occurrence: row.id(),
                owner,
                entity,
            })?;
        }
    }
    // Heterogeneous endpoints are a mechanical vocabulary over the frozen facts universe.
    // Emitting these once avoids giving downstream stages a second writer for EntityRef.
    for occurrence in input.occurrences.iter() {
        output.refs.insert(EntityRef::Occurrence {
            occurrence: occurrence.id(),
        })?;
    }
    for module in input.modules.iter() {
        output.refs.insert(EntityRef::Module {
            module: module.id(),
        })?;
    }
    for term in input.terms.iter() {
        output.refs.insert(EntityRef::Type { term: term.id() })?;
    }
    for place in input.places.iter() {
        output.refs.insert(EntityRef::Place { place: place.id() })?;
    }
    Ok(())
}
fn symbol_entities(
    input: &EntityInputs<'_>,
    output: &mut EntityOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "symbol-correspondence-index");
    let mut declarations: ChargedMap<Id<ProviderSymbol>, Vec<&SymbolDeclaration>> =
        Default::default();
    let mut supports: ChargedMap<Id<SymbolDeclaration>, Vec<&SymbolDeclarationSupport>> =
        Default::default();
    for row in input.declarations.iter() {
        declarations.update(&mut charge, row.symbol, |rows| rows.push(row))?;
    }
    for row in input.declaration_supports.iter() {
        supports.update(&mut charge, row.assertion, |rows| rows.push(row))?;
    }
    let mut resolved: ChargedMap<Id<ProviderSymbol>, Id<SymbolEntityResolution>> =
        Default::default();
    let mut function_traits: ChargedMap<Id<ProviderSymbol>, Vec<&FunctionTraitObservation>> =
        Default::default();
    let mut class_traits: ChargedMap<Id<ProviderSymbol>, Vec<&ClassTraitObservation>> =
        Default::default();
    for row in input.function_traits.iter() {
        function_traits.update(&mut charge, row.symbol, |rows| rows.push(row))?;
    }
    for row in input.class_traits.iter() {
        class_traits.update(&mut charge, row.symbol, |rows| rows.push(row))?;
    }
    for symbol in input.symbols.iter() {
        let mut candidates: ChargedMap<Id<EntityRef>, Vec<SymbolEntityPremise>> =
            Default::default();
        let mut established: ChargedSet<Id<EntityRef>> = Default::default();
        let mut candidate_charge = StateCharge::new(budget, "symbol-candidates");
        let mut unsupported = false;
        for declaration in declarations.get(&symbol.id()).into_iter().flatten() {
            if context(&input, declaration.qualification)? != symbol.context {
                return Err(missing("same-context declaration"));
            }
            let occurrence = input
                .occurrences
                .get(declaration.declaration)
                .ok_or_else(|| missing("declaration occurrence"))?;
            if !kind_accepts(symbol.kind, occurrence) {
                unsupported = true;
                continue;
            }
            let module = match input.provider_modules.get(symbol.module) {
                Some(ProviderModule::Acquired { module }) => Some(*module),
                _ => None,
            };
            let entity = output.source(occurrence, module)?;
            let evidence = supports
                .get(&declaration.id())
                .ok_or_else(|| missing("declaration support"))?;
            if !evidence.is_empty() && certain(&input, declaration.qualification)? {
                established.insert(&mut candidate_charge, entity)?;
            }
            for support in evidence {
                candidates.update(&mut candidate_charge, entity, |rows| {
                    rows.push(SymbolEntityPremise::Declaration {
                        declaration: declaration.id(),
                        support: support.id(),
                    })
                })?;
            }
        }
        let mut reason = EntityReason::DeclarationAgreement;
        if candidates.is_empty() && !unsupported {
            let module = input
                .provider_modules
                .get(symbol.module)
                .ok_or_else(|| missing("provider module"))?;
            match (symbol.kind, module) {
                (SymbolKind::Module, ProviderModule::Acquired { module }) => {
                    let entity = output.refs.insert(EntityRef::Module { module: *module })?;
                    candidates.update(&mut candidate_charge, entity, |rows| {
                        rows.push(SymbolEntityPremise::Module {
                            module: symbol.module,
                        })
                    })?;
                    established.insert(&mut candidate_charge, entity)?;
                    reason = EntityReason::AcquiredModule;
                }
                (
                    SymbolKind::Function | SymbolKind::Method | SymbolKind::Class,
                    ProviderModule::Bundled { .. },
                ) => {
                    let entity = if symbol.kind == SymbolKind::Class {
                        EntityRef::Class {
                            class: output.classes.insert(ClassEntity::External {
                                symbol: symbol.id(),
                            })?,
                        }
                    } else {
                        EntityRef::Callable {
                            callable: output.callables.insert(CallableEntity::External {
                                symbol: symbol.id(),
                            })?,
                        }
                    };
                    let entity = output.refs.insert(entity)?;
                    candidates.update(&mut candidate_charge, entity, |rows| {
                        rows.push(SymbolEntityPremise::Module {
                            module: symbol.module,
                        })
                    })?;
                    established.insert(&mut candidate_charge, entity)?;
                    reason = EntityReason::ProviderExternal;
                }
                (SymbolKind::Function | SymbolKind::Method, _) => {
                    let mut eligible = Vec::new();
                    let mut conflict = false;
                    for traits in function_traits.get(&symbol.id()).into_iter().flatten() {
                        if context(&input, traits.qualification)? != symbol.context {
                            return Err(missing("same-context traits"));
                        }
                        if traits.origin != FunctionOrigin::Synthesized {
                            conflict = true;
                        } else {
                            candidate_charge.admit(&traits.id())?;
                            eligible.push(traits.id());
                        }
                    }
                    if !eligible.is_empty() && !conflict {
                        let entity = output.refs.insert(EntityRef::Callable {
                            callable: output.callables.insert(CallableEntity::Synthetic {
                                symbol: symbol.id(),
                            })?,
                        })?;
                        for observation in eligible {
                            if certain(
                                &input,
                                input
                                    .function_traits
                                    .get(observation)
                                    .expect("indexed traits")
                                    .qualification,
                            )? {
                                established.insert(&mut candidate_charge, entity)?;
                            }
                            candidates.update(&mut candidate_charge, entity, |rows| {
                                rows.push(SymbolEntityPremise::FunctionTraits { observation })
                            })?;
                        }
                        reason = EntityReason::ProviderSynthetic;
                    }
                }
                (SymbolKind::Class, _) => {
                    let mut eligible = Vec::new();
                    let mut conflict = false;
                    for traits in class_traits.get(&symbol.id()).into_iter().flatten() {
                        if context(&input, traits.qualification)? != symbol.context {
                            return Err(missing("same-context traits"));
                        }
                        if !traits.synthesized {
                            conflict = true;
                        } else {
                            candidate_charge.admit(&traits.id())?;
                            eligible.push(traits.id());
                        }
                    }
                    if !eligible.is_empty() && !conflict {
                        let entity = output.refs.insert(EntityRef::Class {
                            class: output.classes.insert(ClassEntity::Synthetic {
                                symbol: symbol.id(),
                            })?,
                        })?;
                        for observation in eligible {
                            if certain(
                                &input,
                                input
                                    .class_traits
                                    .get(observation)
                                    .expect("indexed traits")
                                    .qualification,
                            )? {
                                established.insert(&mut candidate_charge, entity)?;
                            }
                            candidates.update(&mut candidate_charge, entity, |rows| {
                                rows.push(SymbolEntityPremise::ClassTraits { observation })
                            })?;
                        }
                        reason = EntityReason::ProviderSynthetic;
                    }
                }
                _ => {
                    reason = EntityReason::UnsupportedKind;
                }
            }
        }
        let status = if unsupported {
            ResolutionStatus::Unresolved
        } else {
            match candidates.len() {
                0 => ResolutionStatus::Unresolved,
                1 if established.len() == 1 => ResolutionStatus::Resolved,
                1 => ResolutionStatus::Unresolved,
                _ => ResolutionStatus::Ambiguous,
            }
        };
        if unsupported {
            reason = EntityReason::UnsupportedKind;
        } else if status == ResolutionStatus::Ambiguous {
            reason = EntityReason::ConflictingDeclarations;
        } else if candidates.len() == 1 && established.is_empty() {
            reason = EntityReason::QualifiedUncertainty;
        } else if candidates.is_empty() && reason != EntityReason::UnsupportedKind {
            reason = EntityReason::MissingDeclaration;
        }
        let resolution = SymbolEntityResolution {
            symbol: symbol.id(),
            context: symbol.context,
            policy: policy_revision(),
            status,
            entity: (status == ResolutionStatus::Resolved)
                .then(|| *candidates.keys().next().expect("one candidate")),
            reason,
        };
        let resolution = output.resolutions.insert(resolution)?;
        resolved.insert(&mut charge, symbol.id(), resolution)?;
        for (entity, premises) in candidates.iter() {
            let candidate = output.candidates.insert(SymbolEntityCandidate {
                resolution,
                entity: *entity,
            })?;
            for premise in premises {
                let premise = output.premises.insert(premise.clone())?;
                output
                    .evidence
                    .insert(SymbolEntityEvidence { candidate, premise })?;
            }
        }
    }
    normalize_parameters(&input, &resolved, output, budget)?;
    for field in input.fields.iter() {
        let resolution = output
            .resolutions
            .get(
                *resolved
                    .get(&field.class)
                    .ok_or_else(|| missing("field class resolution"))?,
            )
            .expect("indexed resolution");
        if context(&input, field.qualification)? != resolution.context {
            return Err(missing("same-context field"));
        }
        if let Some(EntityRef::Class { class }) =
            resolution.entity.and_then(|id| output.refs.get(id))
        {
            let field_id = output.fields.insert(FieldEntity {
                class: *class,
                name: field.name.clone(),
            })?;
            output.refs.insert(EntityRef::Field { field: field_id })?;
            output.field_links.insert(FieldEntityLink {
                field: field_id,
                observation: field.id(),
            })?;
        }
    }
    Ok(())
}
fn syntax_fields(
    input: &EntityInputs<'_>,
    output: &mut EntityOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "syntax-field-index");
    let mut bindings: ChargedMap<Id<Occurrence>, Vec<&crate::domain::lexical::BindingEvent>> =
        Default::default();
    for binding in input.bindings.iter() {
        bindings.update(&mut charge, binding.site, |rows| rows.push(binding))?;
    }
    for declaration in input.syntax_fields.iter() {
        let occurrence = input
            .occurrences
            .get(declaration.class)
            .ok_or_else(|| missing("field class occurrence"))?;
        if occurrence.syntax_kind != SyntaxKind::StmtClassDef {
            return Err(missing("class field declaration kind"));
        }
        let class = output.classes.insert(ClassEntity::Source {
            declaration: declaration.class,
        })?;
        for binding in bindings.get(&declaration.target).into_iter().flatten() {
            let field = output.fields.insert(FieldEntity {
                class,
                name: binding.name.as_str().into(),
            })?;
            output.refs.insert(EntityRef::Field { field })?;
            output.field_declarations.insert(FieldDeclarationLink {
                field,
                declaration: declaration.id(),
                binding: binding.id(),
            })?;
        }
    }
    Ok(())
}
fn public_entities(
    input: &EntityInputs<'_>,
    output: &mut EntityOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "public-resolution-index");
    let mut resolved = ChargedMap::default();
    for row in output.resolutions.iter() {
        resolved.insert(&mut charge, row.symbol, row.id())?;
    }
    normalize_exposures(input, &resolved, output, budget)
}

fn normalize_parameters(
    input: &EntityInputs<'_>,
    resolved: &ChargedMap<Id<ProviderSymbol>, Id<SymbolEntityResolution>>,
    output: &mut EntityOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "parameter-correspondence-index");
    let mut declarations: ChargedMap<Id<SignatureParameter>, Vec<&ParameterDeclaration>> =
        Default::default();
    for row in input.parameter_declarations.iter() {
        declarations.update(&mut charge, row.parameter, |rows| rows.push(row))?;
    }
    for parameter in input.parameters.iter() {
        let signature = input
            .signatures
            .get(parameter.signature)
            .ok_or_else(|| missing("parameter signature"))?;
        let resolution = output
            .resolutions
            .get(
                *resolved
                    .get(&signature.symbol)
                    .ok_or_else(|| missing("signature symbol"))?,
            )
            .expect("indexed resolution");
        let mut declared = false;
        for declaration in declarations.get(&parameter.id()).into_iter().flatten() {
            if context(input, declaration.qualification)? != resolution.context {
                return Err(missing("same-context parameter"));
            }
            declared = true;
            let entity = output.parameters.insert(ParameterEntity::Source {
                declaration: declaration.declaration,
            })?;
            output
                .refs
                .insert(EntityRef::Parameter { parameter: entity })?;
            output.parameter_links.insert(ParameterEntityLink {
                parameter: parameter.id(),
                entity,
                declaration: Some(declaration.id()),
            })?;
        }
        if !declared
            && let Some(EntityRef::Callable { callable }) =
                resolution.entity.and_then(|id| output.refs.get(id))
            && matches!(
                output.callables.get(*callable),
                Some(CallableEntity::External { .. } | CallableEntity::Synthetic { .. })
            )
        {
            let entity = output.parameters.insert(ParameterEntity::NativeSlot {
                callable: *callable,
                signature: signature.id(),
                parameter: parameter.id(),
            })?;
            output
                .refs
                .insert(EntityRef::Parameter { parameter: entity })?;
            output.parameter_links.insert(ParameterEntityLink {
                parameter: parameter.id(),
                entity,
                declaration: None,
            })?;
        }
    }
    Ok(())
}

fn normalize_exposures(
    input: &EntityInputs<'_>,
    resolved: &ChargedMap<Id<ProviderSymbol>, Id<SymbolEntityResolution>>,
    output: &mut EntityOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut index_charge = StateCharge::new(budget, "public-origin-index");
    type OriginKey = (
        Id<crate::domain::attribution::AnalysisContext>,
        Id<ProviderModule>,
        String,
    );
    let mut symbols: ChargedMap<OriginKey, Vec<&ProviderSymbol>> = Default::default();
    let mut supports: ChargedMap<Id<PublicNameObservation>, Vec<&PublicNameSupport>> =
        Default::default();
    let mut reports: ChargedMap<Id<ProviderSymbol>, Vec<&SymbolObservation>> = Default::default();
    let mut report_supports: ChargedMap<Id<SymbolObservation>, Vec<&SymbolSupport>> =
        Default::default();
    for report in input.symbol_observations.iter() {
        let symbol = input
            .symbols
            .get(report.symbol)
            .ok_or_else(|| missing("reported symbol"))?;
        if context(input, report.qualification)? != symbol.context {
            return Err(missing("same-context symbol report"));
        }
        reports.update(&mut index_charge, report.symbol, |rows| rows.push(report))?;
    }
    for support in input.symbol_supports.iter() {
        report_supports.update(&mut index_charge, support.assertion, |rows| {
            rows.push(support)
        })?;
    }
    for symbol in input.symbols.iter() {
        // Callable type metadata names original overload members which Pysa deliberately
        // does not report as exported definitions. Those identities remain available to
        // signature/origin analysis, but do not enlarge the public namespace candidate set.
        if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method)
            && reports.get(&symbol.id()).is_none()
        {
            continue;
        }
        symbols.update(
            &mut index_charge,
            (symbol.context, symbol.module, symbol.name.clone()),
            |rows| rows.push(symbol),
        )?;
    }
    for support in input.public_supports.iter() {
        supports.update(&mut index_charge, support.assertion, |rows| {
            rows.push(support)
        })?;
    }
    let mut alternatives: ChargedMap<Id<SymbolEntityResolution>, Vec<Id<EntityRef>>> =
        Default::default();
    for candidate in output.candidates.iter() {
        alternatives.update(&mut index_charge, candidate.resolution, |rows| {
            rows.push(candidate.entity)
        })?;
    }
    for enumeration in input.export_enumerations.iter() {
        let q = input
            .qualifications
            .get(enumeration.qualification)
            .ok_or_else(|| missing("export enumeration qualification"))?;
        let supported = input.export_enumeration_supports.iter().any(|s| {
            s.assertion == enumeration.id()
                && input
                    .runs
                    .get(s.run)
                    .is_some_and(|r| r.context == q.context)
                && s.fidelity == crate::domain::attribution::Fidelity::NativeStructural
                && s.origin == crate::domain::attribution::Origin::AnalyzerAssertion
                && s.mode == crate::domain::attribution::ExtractionMode::NativeTraversal
        });
        let closed = enumeration.status == ExportEnumerationStatus::Complete
            && exact_public_qualification(q)
            && supported;
        output
            .public_enumerations
            .insert(PublicEnumerationAssessment {
                observation: enumeration.id(),
                access: enumeration.access,
                context: q.context,
                closed,
            })?;
    }
    for public in input.public_names.iter() {
        let context = context(input, public.qualification)?;
        let q = input
            .qualifications
            .get(public.qualification)
            .ok_or_else(|| missing("public path qualification"))?;
        let publicity = if exact_public_qualification(q) {
            PublicPathKnowledge::Known
        } else if q.modality == crate::domain::attribution::Modality::Candidate {
            PublicPathKnowledge::Candidate
        } else {
            PublicPathKnowledge::Unknown
        };
        let enumerations: Vec<_> = input
            .export_enumerations
            .iter()
            .filter(|e| {
                e.access == public.access
                    && input
                        .qualifications
                        .get(e.qualification)
                        .is_some_and(|eq| eq.context == context && eq.scope == q.scope)
            })
            .collect();
        let enumeration = if enumerations.len() == 1 {
            Some(enumerations[0].id())
        } else {
            None
        };
        let origin = input
            .export_origins
            .get(public.origin)
            .ok_or_else(|| missing("public export origin"))?;
        let mut charge = StateCharge::new(budget, "public-exposure-candidates");
        let mut candidates: ChargedSet<Id<SymbolEntityResolution>> = Default::default();
        let mut entities: ChargedSet<Id<EntityRef>> = Default::default();
        let mut unresolved = false;
        let mut matching_supports = Vec::new();
        if let ExportOrigin::Traced { module, name, kind } = origin {
            for symbol in symbols
                .get(&(context, *module, name.clone()))
                .into_iter()
                .flatten()
            {
                if kind.is_some_and(|kind| {
                    !matches!(
                        (kind, symbol.kind),
                        (ExportKind::Function, SymbolKind::Function)
                            | (ExportKind::Method, SymbolKind::Method)
                            | (ExportKind::Class, SymbolKind::Class)
                            | (ExportKind::Module, SymbolKind::Module)
                            | (
                                ExportKind::Variable
                                    | ExportKind::Constant
                                    | ExportKind::Attribute
                                    | ExportKind::TypeAlias
                                    | ExportKind::TypeParameter
                                    | ExportKind::Parameter,
                                SymbolKind::Variable
                            )
                    )
                }) {
                    continue;
                }
                let mut supported = false;
                for support in supports.get(&public.id()).into_iter().flatten() {
                    let run = input
                        .runs
                        .get(support.run)
                        .ok_or_else(|| missing("public-name support run"))?;
                    if run.provider == symbol.provider && run.context == context {
                        if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
                            let mut reported = false;
                            let mut exact_report = false;
                            for report in reports.get(&symbol.id()).into_iter().flatten() {
                                let qualified = input
                                    .qualifications
                                    .get(report.qualification)
                                    .ok_or_else(|| missing("symbol report qualification"))?;
                                let admitted = report_supports
                                    .get(&report.id())
                                    .into_iter()
                                    .flatten()
                                    .any(|s| {
                                        // Pysa's reported definition inventory is a report
                                        // projection. It selects namespace candidates only;
                                        // source correspondence and execution proof stay separate.
                                        s.run == support.run
                                            && s.fidelity == attribution::Fidelity::ReportProjection
                                            && s.origin == attribution::Origin::AnalyzerAssertion
                                            && s.mode
                                                == attribution::ExtractionMode::NativeTraversal
                                    });
                                reported |= admitted;
                                exact_report |= admitted && exact_public_qualification(qualified);
                            }
                            if !reported {
                                unresolved = true;
                                continue;
                            }
                            unresolved |= !exact_report;
                        }
                        let resolution = *resolved
                            .get(&symbol.id())
                            .ok_or_else(|| missing("public symbol resolution"))?;
                        let pair = (resolution, support.id());
                        charge.admit(&pair)?;
                        matching_supports.push(pair);
                        supported = true;
                    }
                }
                if !supported {
                    continue;
                }
                let resolution = *resolved
                    .get(&symbol.id())
                    .ok_or_else(|| missing("public symbol resolution"))?;
                candidates.insert(&mut charge, resolution)?;
                let row = output
                    .resolutions
                    .get(resolution)
                    .expect("indexed resolution");
                if row.status == ResolutionStatus::Unresolved {
                    unresolved = true;
                }
                for entity in alternatives.get(&resolution).into_iter().flatten() {
                    entities.insert(&mut charge, *entity)?;
                }
            }
        }
        let status = if candidates.is_empty() || unresolved {
            ResolutionStatus::Unresolved
        } else if entities.len() == 1 {
            ResolutionStatus::Resolved
        } else {
            ResolutionStatus::Ambiguous
        };
        let exposure = output.exposures.insert(PublicExposure {
            access: public.access,
            context,
            observation: public.id(),
            origin: public.origin,
            enumeration,
            publicity,
            status,
            reason: match origin {
                ExportOrigin::Untraced => EntityReason::UntracedExposure,
                _ if status == ResolutionStatus::Resolved => EntityReason::DeclarationAgreement,
                _ if status == ResolutionStatus::Ambiguous => EntityReason::ConflictingDeclarations,
                _ => EntityReason::MissingCorrespondence,
            },
        })?;
        for (resolution, support) in matching_supports {
            output.exposure_candidates.insert(PublicExposureCandidate {
                exposure,
                resolution,
                support,
            })?;
        }
    }
    Ok(())
}

fn exact_public_qualification(q: &AssertionQualification) -> bool {
    q.assumptions == crate::domain::assumptions::AssumptionSet::empty_id()
        && q.modality == crate::domain::attribution::Modality::Definite
        && q.approximation == crate::domain::assertion::Approximation::Exact
        && q.condition == crate::domain::conditions::Diagram::always().id()
}
/// First namespace query consumes normalized identity and native enumeration separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicPathStatus {
    Public,
    NotPublic,
    Candidate,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicPathDecision {
    pub status: PublicPathStatus,
    pub path: Option<Id<PublicExposure>>,
    pub enumeration: Option<Id<PublicEnumerationAssessment>>,
}
pub fn public_path_decision(
    input: &EntityInputs<'_>,
    output: &EntityOutput,
    access: Id<Module>,
    context: Id<crate::domain::attribution::AnalysisContext>,
    name: &str,
) -> Result<PublicPathDecision, ModelError> {
    let mut candidate = None;
    let mut unknown = None;
    for path in output
        .exposures
        .iter()
        .filter(|e| e.access == access && e.context == context)
    {
        let row = input
            .public_names
            .get(path.observation)
            .ok_or_else(|| missing("public query observation"))?;
        if row.name != name {
            continue;
        }
        match path.publicity {
            PublicPathKnowledge::Known => {
                return Ok(PublicPathDecision {
                    status: PublicPathStatus::Public,
                    path: Some(path.id()),
                    enumeration: output
                        .public_enumerations
                        .iter()
                        .find(|e| Some(e.observation) == path.enumeration)
                        .map(Record::id),
                });
            }
            PublicPathKnowledge::Candidate => candidate = Some(path.id()),
            PublicPathKnowledge::Unknown => unknown = Some(path.id()),
        }
    }
    let basis = |path| {
        output
            .exposures
            .get(path)
            .and_then(|path| {
                output
                    .public_enumerations
                    .iter()
                    .find(|e| Some(e.observation) == path.enumeration)
            })
            .map(Record::id)
    };
    if let Some(path) = candidate {
        return Ok(PublicPathDecision {
            status: PublicPathStatus::Candidate,
            path: Some(path),
            enumeration: basis(path),
        });
    }
    if let Some(path) = unknown {
        return Ok(PublicPathDecision {
            status: PublicPathStatus::Unknown,
            path: Some(path),
            enumeration: basis(path),
        });
    }
    let enumeration = output
        .public_enumerations
        .iter()
        .filter(|e| e.access == access && e.context == context)
        .find(|e| e.closed)
        .or_else(|| {
            output
                .public_enumerations
                .iter()
                .find(|e| e.access == access && e.context == context)
        });
    Ok(PublicPathDecision {
        status: if enumeration.is_some_and(|e| e.closed) {
            PublicPathStatus::NotPublic
        } else {
            PublicPathStatus::Unknown
        },
        path: None,
        enumeration: enumeration.map(Record::id),
    })
}

/// Required admission is independent of producer replay. It retains only compact identity,
/// membership and qualification receipts; no source text, paths or rich provider inventories.
fn admission() -> Invariant {
    Invariant {
        purpose: InvariantPurpose::Admission,
        revision: 1,
        name: "normalized_entity_membership",
        inputs: super::facts_inputs(vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<Module>(&["id"]),
            ValidationInput::of::<types::TypeTerm>(&["id"]),
            ValidationInput::of::<value::Place>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<ProviderModule>(&["id"]),
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<FunctionTraitObservation>(&["id"]),
            ValidationInput::of::<ClassTraitObservation>(&["id"]),
            ValidationInput::of::<PublicNameObservation>(&["id"]),
            ValidationInput::of::<ExportEnumerationObservation>(&["id"]),
            ValidationInput::of::<ExportEnumerationSupport>(&["id"]),
            ValidationInput::of::<attribution::ProviderRun>(&["id"]),
            ValidationInput::of::<CallableEntity>(&["id"]),
            ValidationInput::of::<ClassEntity>(&["id"]),
            ValidationInput::of::<ParameterEntity>(&["id"]),
            ValidationInput::of::<EntityRef>(&["id"]),
            ValidationInput::of::<SymbolEntityResolution>(&["id"]),
            ValidationInput::of::<SymbolEntityCandidate>(&["id"]),
            ValidationInput::of::<PublicExposure>(&["id"]),
            ValidationInput::of::<PublicEnumerationAssessment>(&["id"]),
        ]),
        create: std::sync::Arc::new(|budget| Box::new(EntityAdmission::new(budget))),
    }
}
struct EntityAdmission {
    charge: StateCharge,
    occurrences: ChargedMap<Id<Occurrence>, SyntaxKind>,
    symbols: ChargedMap<
        Id<ProviderSymbol>,
        (
            SymbolKind,
            Id<attribution::AnalysisContext>,
            Id<ProviderModule>,
        ),
    >,
    modules: ChargedMap<Id<ProviderModule>, bool>,
    qualifications: ChargedMap<
        Id<AssertionQualification>,
        (Id<attribution::AnalysisContext>, PublicPathKnowledge, bool),
    >,
    functions: ChargedMap<Id<ProviderSymbol>, Vec<(Id<AssertionQualification>, FunctionOrigin)>>,
    classes: ChargedMap<Id<ProviderSymbol>, Vec<(Id<AssertionQualification>, bool)>>,
    public: ChargedMap<
        Id<PublicNameObservation>,
        (Id<AssertionQualification>, Id<Module>, Id<ExportOrigin>),
    >,
    enumerations: ChargedMap<
        Id<ExportEnumerationObservation>,
        (Id<AssertionQualification>, Id<Module>, bool),
    >,
    supports: ChargedSet<(
        Id<ExportEnumerationObservation>,
        Id<attribution::ProviderRun>,
        bool,
    )>,
    runs: ChargedMap<Id<attribution::ProviderRun>, Id<attribution::AnalysisContext>>,
    refs: ChargedSet<Id<EntityRef>>,
    required_refs: ChargedSet<Id<EntityRef>>,
    callables: Rows<CallableEntity>,
    class_entities: Rows<ClassEntity>,
    parameters: Rows<ParameterEntity>,
    resolutions: Rows<SymbolEntityResolution>,
    candidates: ChargedMap<Id<SymbolEntityResolution>, Vec<Id<EntityRef>>>,
    exposures: Rows<PublicExposure>,
    public_enumerations: Rows<PublicEnumerationAssessment>,
}
impl EntityAdmission {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            charge: StateCharge::new(budget, "normalized-entity-admission"),
            occurrences: Default::default(),
            symbols: Default::default(),
            modules: Default::default(),
            qualifications: Default::default(),
            functions: Default::default(),
            classes: Default::default(),
            public: Default::default(),
            enumerations: Default::default(),
            supports: Default::default(),
            runs: Default::default(),
            refs: Default::default(),
            required_refs: Default::default(),
            callables: Rows::new(budget),
            class_entities: Rows::new(budget),
            parameters: Rows::new(budget),
            resolutions: Rows::new(budget),
            candidates: Default::default(),
            exposures: Rows::new(budget),
            public_enumerations: Rows::new(budget),
        }
    }
    fn symbol(
        &self,
        id: Id<ProviderSymbol>,
        class: bool,
        external: bool,
    ) -> Result<(), ModelError> {
        let (kind, context, module) = self
            .symbols
            .get(&id)
            .ok_or_else(|| missing("entity symbol membership"))?;
        if (class && *kind != SymbolKind::Class)
            || (!class && !matches!(kind, SymbolKind::Function | SymbolKind::Method))
        {
            return Err(missing("entity symbol kind"));
        }
        if external {
            if self.modules.get(module) != Some(&true) {
                return Err(missing("external entity bundled module"));
            }
        } else if class {
            let traits = self
                .classes
                .get(&id)
                .ok_or_else(|| missing("synthetic class traits"))?;
            if traits.is_empty()
                || traits.iter().any(|(q, synthesized)| {
                    !*synthesized
                        || self
                            .qualifications
                            .get(q)
                            .is_none_or(|(ctx, ..)| ctx != context)
                })
            {
                return Err(missing("synthetic class fidelity"));
            }
        } else {
            let traits = self
                .functions
                .get(&id)
                .ok_or_else(|| missing("synthetic callable traits"))?;
            if traits.is_empty()
                || traits.iter().any(|(q, origin)| {
                    *origin != FunctionOrigin::Synthesized
                        || self
                            .qualifications
                            .get(q)
                            .is_none_or(|(ctx, ..)| ctx != context)
                })
            {
                return Err(missing("synthetic callable fidelity"));
            }
        }
        Ok(())
    }
}
impl InvariantCheck for EntityAdmission {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        macro_rules! each { ($ty:ty,$row:ident,$body:block) => { if relation == <$ty>::NAME { for $row in <$ty>::decode(batch)? $body return Ok(()); } }; }
        each!(Occurrence, row, {
            self.occurrences
                .insert(&mut self.charge, row.id(), row.syntax_kind)?;
            self.required_refs.insert(
                &mut self.charge,
                EntityRef::Occurrence {
                    occurrence: row.id(),
                }
                .id(),
            )?;
            let declaration = if let Some(callable) = source_callable(&row) {
                Some(EntityRef::Callable {
                    callable: callable.id(),
                })
            } else if row.syntax_kind == SyntaxKind::StmtClassDef {
                Some(EntityRef::Class {
                    class: ClassEntity::Source {
                        declaration: row.id(),
                    }
                    .id(),
                })
            } else if row.syntax_kind == SyntaxKind::Parameter {
                Some(EntityRef::Parameter {
                    parameter: ParameterEntity::Source {
                        declaration: row.id(),
                    }
                    .id(),
                })
            } else {
                None
            };
            if let Some(entity) = declaration {
                self.required_refs.insert(&mut self.charge, entity.id())?;
            }
        });
        each!(Module, row, {
            self.required_refs.insert(
                &mut self.charge,
                EntityRef::Module { module: row.id() }.id(),
            )?;
        });
        each!(types::TypeTerm, row, {
            self.required_refs
                .insert(&mut self.charge, EntityRef::Type { term: row.id() }.id())?;
        });
        each!(value::Place, row, {
            self.required_refs
                .insert(&mut self.charge, EntityRef::Place { place: row.id() }.id())?;
        });
        each!(ProviderSymbol, row, {
            self.symbols.insert(
                &mut self.charge,
                row.id(),
                (row.kind, row.context, row.module),
            )?;
        });
        each!(ProviderModule, row, {
            self.modules.insert(
                &mut self.charge,
                row.id(),
                matches!(row, ProviderModule::Bundled { .. }),
            )?;
        });
        each!(AssertionQualification, row, {
            let publicity = if exact_public_qualification(&row) {
                PublicPathKnowledge::Known
            } else if row.modality == attribution::Modality::Candidate {
                PublicPathKnowledge::Candidate
            } else {
                PublicPathKnowledge::Unknown
            };
            self.qualifications.insert(
                &mut self.charge,
                row.id(),
                (row.context, publicity, exact_public_qualification(&row)),
            )?;
        });
        each!(FunctionTraitObservation, row, {
            self.functions
                .update(&mut self.charge, row.symbol, |rows| {
                    rows.push((row.qualification, row.origin))
                })?;
        });
        each!(ClassTraitObservation, row, {
            self.classes.update(&mut self.charge, row.symbol, |rows| {
                rows.push((row.qualification, row.synthesized))
            })?;
        });
        each!(PublicNameObservation, row, {
            self.public.insert(
                &mut self.charge,
                row.id(),
                (row.qualification, row.access, row.origin),
            )?;
        });
        each!(ExportEnumerationObservation, row, {
            self.enumerations.insert(
                &mut self.charge,
                row.id(),
                (
                    row.qualification,
                    row.access,
                    row.status == ExportEnumerationStatus::Complete,
                ),
            )?;
        });
        each!(ExportEnumerationSupport, row, {
            self.supports.insert(
                &mut self.charge,
                (
                    row.assertion,
                    row.run,
                    row.fidelity == attribution::Fidelity::NativeStructural
                        && row.origin == attribution::Origin::AnalyzerAssertion
                        && row.mode == attribution::ExtractionMode::NativeTraversal,
                ),
            )?;
        });
        each!(attribution::ProviderRun, row, {
            self.runs.insert(&mut self.charge, row.id(), row.context)?;
        });
        each!(EntityRef, row, {
            self.refs.insert(&mut self.charge, row.id())?;
        });
        each!(SymbolEntityCandidate, row, {
            self.candidates
                .update(&mut self.charge, row.resolution, |entities| {
                    entities.push(row.entity)
                })?;
        });
        macro_rules! rows {
            ($ty:ty,$field:ident) => {
                if relation == <$ty>::NAME {
                    self.$field.decode(batch)?;
                    return Ok(());
                }
            };
        }
        rows!(CallableEntity, callables);
        rows!(ClassEntity, class_entities);
        rows!(ParameterEntity, parameters);
        rows!(SymbolEntityResolution, resolutions);
        rows!(PublicExposure, exposures);
        rows!(PublicEnumerationAssessment, public_enumerations);
        Err(missing("declared entity admission input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.required_refs.iter().any(|id| !self.refs.contains(id)) {
            return Err(missing("complete mechanical endpoint membership"));
        }
        if self.exposures.len() != self.public.len()
            || self.public_enumerations.len() != self.enumerations.len()
        {
            return Err(missing("total public observation membership"));
        }
        for row in self.callables.iter() {
            match row {
                CallableEntity::Source { declaration, kind } => {
                    let expected = match self.occurrences.get(declaration) {
                        Some(SyntaxKind::StmtFunctionDef) => CallableKind::Function,
                        Some(SyntaxKind::ExprLambda) => CallableKind::Lambda,
                        _ => return Err(missing("callable source kind")),
                    };
                    if *kind != expected {
                        return Err(missing("callable source fidelity"));
                    }
                }
                CallableEntity::External { symbol } => self.symbol(*symbol, false, true)?,
                CallableEntity::Synthetic { symbol } => self.symbol(*symbol, false, false)?,
            }
            if !self
                .refs
                .contains(&EntityRef::Callable { callable: row.id() }.id())
            {
                return Err(missing("callable endpoint membership"));
            }
        }
        for row in self.class_entities.iter() {
            match row {
                ClassEntity::Source { declaration } => {
                    if self.occurrences.get(declaration) != Some(&SyntaxKind::StmtClassDef) {
                        return Err(missing("class source kind"));
                    }
                }
                ClassEntity::External { symbol } => self.symbol(*symbol, true, true)?,
                ClassEntity::Synthetic { symbol } => self.symbol(*symbol, true, false)?,
            }
            if !self
                .refs
                .contains(&EntityRef::Class { class: row.id() }.id())
            {
                return Err(missing("class endpoint membership"));
            }
        }
        for row in self.parameters.iter() {
            if let ParameterEntity::Source { declaration } = row
                && self.occurrences.get(declaration) != Some(&SyntaxKind::Parameter)
            {
                return Err(missing("parameter source kind"));
            }
            if !self.refs.contains(
                &EntityRef::Parameter {
                    parameter: row.id(),
                }
                .id(),
            ) {
                return Err(missing("parameter endpoint membership"));
            }
        }
        for (resolution, entities) in self.candidates.iter() {
            if self.resolutions.get(*resolution).is_none()
                || entities.iter().any(|entity| !self.refs.contains(entity))
            {
                return Err(missing("candidate resolution/endpoint membership"));
            }
        }
        let mut total = ChargedSet::default();
        let mut charge = StateCharge::new(
            self.charge.budget().expect("admission budget"),
            "entity-resolution-totality",
        );
        for row in self.resolutions.iter() {
            let (_, context, _) = self
                .symbols
                .get(&row.symbol)
                .ok_or_else(|| missing("resolution symbol membership"))?;
            if context != &row.context
                || row.policy != policy_revision()
                || !total.insert(&mut charge, row.symbol)?
            {
                return Err(missing("resolution identity/context/policy"));
            }
            let candidates = self
                .candidates
                .get(&row.id())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if candidates.iter().any(|id| !self.refs.contains(id)) {
                return Err(missing("candidate endpoint membership"));
            }
            if row.status == ResolutionStatus::Resolved
                && (candidates.len() != 1 || row.entity != Some(candidates[0]))
            {
                return Err(missing("resolved candidate agreement"));
            }
            if row.status == ResolutionStatus::Ambiguous && candidates.len() < 2 {
                return Err(missing("ambiguous candidate membership"));
            }
        }
        if total.len() != self.symbols.len() {
            return Err(missing("total symbol correspondence"));
        }
        for row in self.exposures.iter() {
            let (qualification, access, origin) = self
                .public
                .get(&row.observation)
                .ok_or_else(|| missing("public observation membership"))?;
            let (context, publicity, _) = self
                .qualifications
                .get(qualification)
                .ok_or_else(|| missing("public qualification"))?;
            if access != &row.access
                || origin != &row.origin
                || context != &row.context
                || publicity != &row.publicity
            {
                return Err(missing("public exposure identity/fidelity"));
            }
        }
        for row in self.public_enumerations.iter() {
            let (q, access, complete) = self
                .enumerations
                .get(&row.observation)
                .ok_or_else(|| missing("enumeration observation membership"))?;
            let (context, _, exact) = self
                .qualifications
                .get(q)
                .ok_or_else(|| missing("enumeration qualification"))?;
            let supported = self.supports.iter().any(|(observation, run, exact)| {
                *observation == row.observation && *exact && self.runs.get(run) == Some(context)
            });
            if access != &row.access
                || context != &row.context
                || row.closed != (*complete && *exact && supported)
            {
                return Err(missing("enumeration identity/fidelity"));
            }
        }
        Ok(())
    }
}

/// One authoritative relation-set validator, also used at completed-stage read admission.
/// Recompute the pure correspondence contract over frozen inputs and compare every output,
/// including missing/extra rows. Independent fixtures assert source-written answers separately.
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = EntityData::validation_inputs();
    inputs.extend(EntityOutput::validation_inputs());
    vec![
        admission(),
        Invariant {
            purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
            revision: 1,
            name: "normalized_entity_closure",
            inputs,
            create: std::sync::Arc::new(|budget| {
                Box::new(EntityCheck {
                    data: EntityData::new(budget),
                    output: EntityOutput::new(budget),
                    budget: budget.clone(),
                })
            }),
        },
    ]
}
struct EntityCheck {
    data: EntityData,
    output: EntityOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for EntityCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if !self.data.visit(relation, batch)? && !self.output.visit(relation, batch)? {
            return Err(missing("declared entity check input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.output
            .matches(&normalize(self.data.inputs(), &self.budget)?)
    }
}
pub fn stage() -> stages::Stage {
    stages::Stage {
        name: "normalize_entities",
        inputs: super::facts_stage_inputs(EntityData::stage_inputs()),
        outputs: super::entities::relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: stages::Profile::ALL.to_vec(),
        effect: stages::Effect::Pure,
        code: policy_revision(),
        configuration: ContentHash::of(b"entities/v1"),
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["normalized_entity_membership", "normalized_entity_closure"]
}
