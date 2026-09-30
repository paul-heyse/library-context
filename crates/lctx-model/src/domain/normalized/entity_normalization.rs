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
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::stored::<$ty>()
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
                $(if !self.$field.same(&expected.$field) { return Err(ModelError::Invalid(format!("normalized entity closure differs: {}", <$ty>::NAME))); })*
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
    normalize_parameters(&input, &resolved, &mut output, budget)?;
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
    normalize_exposures(&input, &resolved, &mut output, budget)?;
    Ok(output)
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
    for symbol in input.symbols.iter() {
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
    for public in input.public_names.iter() {
        let context = context(input, public.qualification)?;
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

/// One authoritative relation-set validator, also used at completed-stage read admission.
/// Recompute the pure correspondence contract over frozen inputs and compare every output,
/// including missing/extra rows. Independent fixtures assert source-written answers separately.
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = EntityData::validation_inputs();
    inputs.extend(EntityOutput::validation_inputs());
    vec![Invariant {
        name: "normalized_entity_closure",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(EntityCheck {
                data: EntityData::new(budget),
                output: EntityOutput::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
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
        inputs: EntityData::stage_inputs(),
        outputs: super::entities::relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: stages::Profile::ALL.to_vec(),
        effect: stages::Effect::Pure,
        code: policy_revision(),
        configuration: ContentHash::of(b"entities/v1"),
    }
}
