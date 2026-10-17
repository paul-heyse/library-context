//! Shared deterministic C0 builder and publication closure invariant.
use super::*;
use crate::domain::{
    charged::{ChargedMap, StateCharge},
    normalized::Rows,
    resources::ResourceBudget,
    stages,
    symbols::{FunctionOrigin, Linearization},
};
macro_rules! data {($($field:ident:$ty:ty,)*)=>{
    pub struct CatalogData {$(pub $field:Rows<$ty>,)*}

impl CatalogData {
        pub fn new(budget:&ResourceBudget)->Self {Self {$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if relation==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
        pub fn validation_inputs()->Vec<ValidationInput> {crate::domain::normalized::facts_inputs(vec![$(ValidationInput::of::<$ty>(&["id"]),)*])}
        pub fn stage_inputs()->Vec<stages::RelationUse> {vec![$(stages::RelationUse::completed::<$ty>()),*]}
        /// Copy exact memberships from one hydrated union using the shared typed/prefix operation.
        pub fn selected_copy<I:Iterator<Item=[u8;16]>>(&self,inputs:&[ValidationInput],
            keys:&mut dyn FnMut(usize)->Result<I,ModelError>,budget:&ResourceBudget)->Result<Self,ModelError> {
            let mut selected=Self::new(budget);
            $(crate::domain::normalized::copy_selected_rows(&self.$field,&mut selected.$field,inputs,None,keys)?;)*
            Ok(selected)
        }
    }
};}
crate::catalog_inputs!(data);
/// C0 physical metadata acquisition. Rich kernel/receiving premises remain declared
/// by CatalogData; they are loaded only by the selected public windows that use them.
pub fn metadata_validation_inputs() -> Vec<ValidationInput> {
    let mut inputs = analysis::expected::inputs(definition().1.method);
    inputs.extend([
        ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ValidationInput::of::<analysis::MethodParameters>(&["id"]),
        ValidationInput::of::<attribution::ProviderRun>(&["id"]),
    ]);
    inputs
}
type ConstructorMatches = std::collections::BTreeMap<
    Id<FunctionTraitObservation>,
    Vec<(Option<Id<AncestryEntityMember>>, Knowledge)>,
>;
/// Fixed invocation-local correspondence: admit map nodes and vector capacity before
/// allocation. Values are borrowed rows or inline identities, not separately owned backing.
pub(super) fn append_index<K: Ord + HeapSize, V: Copy>(
    index: &mut std::collections::BTreeMap<K, Vec<V>>,
    charge: &mut StateCharge,
    key: K,
    value: V,
) -> Result<(), ModelError> {
    if !index.contains_key(&key) {
        charge.grow(size_of::<K>() + size_of::<Vec<V>>() + 32 + key.heap_bytes())?;
    }
    let values = index.entry(key).or_default();
    if values.len() == values.capacity() {
        let additional = values.capacity().max(4);
        charge.grow(
            additional
                .checked_mul(size_of::<V>())
                .ok_or_else(|| invalid("catalog correspondence allocation overflow"))?,
        )?;
        values.reserve_exact(additional);
    }
    values.push(value);
    Ok(())
}
// One immutable C0 invocation owns these borrowed correspondence buckets. Rows retain their
// canonical ID order, including competing evidence; no index selects a winner.
macro_rules! correspondence {
    ($($name:ident: $key:ty => $row:ty, $source:ident, $key_of:expr;)*) => {
        pub(super) struct CatalogPreparation<'a> {
            data: &'a CatalogData,
            $($name: std::collections::BTreeMap<$key, Vec<&'a $row>> ,)*
            constructor_traits: std::collections::BTreeMap<Id<attribution::AnalysisContext>, Vec<&'a FunctionTraitObservation>>,
            missing_trait_symbols: crate::domain::charged::ChargedSet<Id<attribution::AnalysisContext>>,
            constructor_ancestry_validation: std::cell::OnceCell<Result<(), std::sync::Arc<ModelError>>>,
            trait_validation: std::cell::OnceCell<Result<(), std::sync::Arc<ModelError>>>,
            metadata_validation: std::cell::OnceCell<Result<(), std::sync::Arc<ModelError>>>,
            _charge: StateCharge,
        }
        impl<'a> CatalogPreparation<'a> {
            fn new(data: &'a CatalogData, budget: &ResourceBudget) -> Result<Self, ModelError> {
                let mut charge = StateCharge::new(budget, "catalog-correspondence");
                $(let mut $name = std::collections::BTreeMap::new();
                  for row in data.$source.iter() {
                      append_index(&mut $name,&mut charge,($key_of)(row),row)?;
                  })*
                let mut constructor_traits = std::collections::BTreeMap::new();
                let mut missing_trait_symbols = crate::domain::charged::ChargedSet::default();
                for row in data.traits.iter() {
                    if let Some(q) = data.qualifications.get(row.qualification) {
                        match data.symbols.get(row.symbol) {
                            None => {missing_trait_symbols.insert(&mut charge,q.context)?;},
                            Some(symbol) if row.defining_class.is_some() && matches!(symbol.name.as_str(),"__init__"|"__new__") => {
                                append_index(&mut constructor_traits,&mut charge,q.context,row)?;
                            },
                            Some(_) => {},
                        }
                    }
                }
                Ok(Self {data, $($name,)* constructor_traits, missing_trait_symbols, constructor_ancestry_validation: Default::default(), trait_validation: Default::default(), metadata_validation: Default::default(), _charge: charge})
            }
            $(pub(super) fn $name(&self, key: $key) -> impl Iterator<Item=&'a $row> + '_ {
                self.$name.get(&key).into_iter().flatten().copied()
            })*
        }
        impl std::ops::Deref for CatalogPreparation<'_> {
            type Target = CatalogData;
            fn deref(&self) -> &Self::Target { self.data }
        }
    }
}
correspondence! {
    entity_candidates_for: Id<SymbolEntityResolution> => SymbolEntityCandidate, entity_candidates, |r: &SymbolEntityCandidate| r.resolution;
    exposure_candidates_for: Id<PublicExposure> => PublicExposureCandidate, exposure_candidates, |r: &PublicExposureCandidate| r.exposure;
    parameters_for_function: Id<Occurrence> => ParameterSyntaxObservation, parameter_syntax, |r: &ParameterSyntaxObservation| r.function;
    syntax_for_parameter: Id<Occurrence> => ParameterSyntaxObservation, parameter_syntax, |r: &ParameterSyntaxObservation| r.parameter;
    placements_for_parent: Option<Id<Occurrence>> => syntax::SyntaxPlacement, placements, |r: &syntax::SyntaxPlacement| r.parent;
    assessments_for: (Id<CallableEntity>, Id<attribution::AnalysisContext>) => EffectiveCallableAssessment, assessments, |r: &EffectiveCallableAssessment| (r.callable, r.context);
    aspects_for: Id<EffectiveCallableAssessment> => normalized::callable_aspects::CallableAspect, aspects, |r: &normalized::callable_aspects::CallableAspect| r.assessment;
    variants_for: (Option<Id<CallableEntity>>, Id<attribution::AnalysisContext>, Option<Id<EffectiveCallableAssessment>>) => SignatureVariant, variants, |r: &SignatureVariant| (r.callable, r.context, r.assessment);
    slots_for: Id<SignatureVariant> => SignatureSlot, slots, |r: &SignatureSlot| r.variant;
    entities_for_slot: Id<SignatureSlot> => SignatureSlotEntity, slot_entities, |r: &SignatureSlotEntity| r.slot;
    fields_for: Id<ClassEntity> => FieldEntity, fields, |r: &FieldEntity| r.class;
    declarations_for_field: Id<FieldEntity> => FieldDeclarationLink, field_declarations, |r: &FieldDeclarationLink| r.field;
    defaults_for: Id<FieldDeclarationLink> => normalized::callable_aspects::FieldDefaultAssessment, field_default_assessments, |r: &normalized::callable_aspects::FieldDefaultAssessment| r.declaration;
    links_for_field: Id<FieldEntity> => FieldEntityLink, field_links, |r: &FieldEntityLink| r.field;
    resolutions_for_symbol: (Id<calls::ProviderSymbol>, Id<attribution::AnalysisContext>) => SymbolEntityResolution, resolutions, |r: &SymbolEntityResolution| (r.symbol, r.context);
    resolutions_for_entity: (Id<attribution::AnalysisContext>, Option<Id<EntityRef>>) => SymbolEntityResolution, resolutions, |r: &SymbolEntityResolution| (r.context, r.entity);
    class_traits_for: Id<calls::ProviderSymbol> => symbols::ClassTraitObservation, class_traits, |r: &symbols::ClassTraitObservation| r.symbol;
    ancestry_for: Id<SymbolEntityResolution> => AncestryEntityAssessment, ancestry, |r: &AncestryEntityAssessment| r.class;
    ancestors_for: Id<AncestryEntityAssessment> => AncestryEntityMember, ancestors, |r: &AncestryEntityMember| r.assessment;
    children_for: Option<Id<Occurrence>> => syntax::DeclarationObservation, declarations, |r: &syntax::DeclarationObservation| r.parent;
    traits_for_defining: Option<Id<calls::ProviderSymbol>> => FunctionTraitObservation, traits, |r: &FunctionTraitObservation| r.defining_class;
    metadata_for: Id<calls::ProviderSymbol> => class_metadata::ClassMetadataObservation, class_metadata, |r: &class_metadata::ClassMetadataObservation| r.class;
    members_for: Id<calls::ProviderSymbol> => class_metadata::ClassMemberObservation, class_members, |r: &class_metadata::ClassMemberObservation| r.class;
    coverage_for: (Id<source::CoverageScope>, Id<attribution::AnalysisContext>) => attribution::ProviderCoverage, native_coverage, |r: &attribution::ProviderCoverage| (r.scope, r.context);

}

impl CatalogPreparation<'_> {
    fn validate_constructor_ancestry(&self) -> Result<(), ModelError> {
        self.constructor_ancestry_validation
            .get_or_init(|| {
                // The original inherited-constructor scan checked these references before its
                // class predicate. Delay this validation until an inherited route is consumed.
                for ancestor in self.ancestors.iter() {
                    let parent =
                        need(&self.ancestry, ancestor.assessment).map_err(std::sync::Arc::new)?;
                    need(&self.resolutions, parent.class).map_err(std::sync::Arc::new)?;
                }
                Ok(())
            })
            .clone()
            .map_err(ModelError::SharedCause)
    }
    fn validate_traits(&self) -> Result<(), ModelError> {
        self.trait_validation
            .get_or_init(|| {
                for row in self.traits.iter() {
                    need(&self.qualifications, row.qualification).map_err(std::sync::Arc::new)?;
                }
                Ok(())
            })
            .clone()
            .map_err(ModelError::SharedCause)
    }
    fn validate_metadata(&self) -> Result<(), ModelError> {
        self.metadata_validation
            .get_or_init(|| {
                for row in self.class_metadata.iter() {
                    need(&self.qualifications, row.qualification).map_err(std::sync::Arc::new)?;
                }
                for row in self.class_members.iter() {
                    need(&self.qualifications, row.qualification).map_err(std::sync::Arc::new)?;
                }
                Ok(())
            })
            .clone()
            .map_err(ModelError::SharedCause)
    }
}

impl CatalogData {
    /// Exact companions consumed by C0's scoped reader; later vocabulary cannot enlarge facts.
    pub fn scoped_field_bindings(
        model: &ValidatedModel,
        inputs: &[ValidationInput],
    ) -> Result<Vec<Vec<Option<usize>>>, ModelError> {
        scoped_field_bindings(
            model,
            inputs,
            &Self::validation_inputs(),
            |source, field| {
                // C0 reads qualification context/scope properties, not ConditionsData or assumptions.
                source.type_id() == std::any::TypeId::of::<assertion::AssertionQualification>()
                    && matches!(field.name(), "condition" | "assumptions")
            },
        )
    }
}

/// Bind the finite property inventory once, before any grain reads. None means the owning
/// reader does not consume this field's companion, never that an available epoch may substitute.
pub(crate) fn scoped_field_bindings(
    model: &ValidatedModel,
    inputs: &[ValidationInput],
    declared: &[ValidationInput],
    omit: impl Fn(&ValidationInput, &Field) -> bool,
) -> Result<Vec<Vec<Option<usize>>>, ModelError> {
    use stages::{PublicationBoundary, is_epoch_shared, is_vocabulary};
    if inputs.len() != declared.len()
        || declared.iter().any(|required| {
            inputs
                .iter()
                .filter(|input| {
                    input.type_id() == required.type_id() && input.prefix() == required.prefix()
                })
                .count()
                != 1
        })
    {
        return Err(ModelError::Conflict(
            "scoped declared immutable reader absent or ambiguous",
        ));
    }
    inputs
        .iter()
        .map(|source| {
            if !declared.iter().any(|input| {
                input.type_id() == source.type_id() && input.prefix() == source.prefix()
            }) {
                return Err(ModelError::Conflict("scoped source immutable reader"));
            }
            let relation = model
                .relation(source.name())
                .filter(|relation| relation.type_id() == source.type_id())
                .ok_or(ModelError::Schema("scoped source model relation"))?;
            relation
                .fields()
                .iter()
                .map(|field| {
                    let Some((kind, name)) = field.target() else {
                        return Ok(None);
                    };
                    if omit(source, field) {
                        return Ok(None);
                    }
                    // Ordinary companions absent from the owner's declared property inventory are
                    // outside this read grain. Shared companions use the same explicit inventory rule.
                    if !declared.iter().any(|input| input.type_id() == kind) {
                        return Ok(None);
                    }
                    let prefix = if is_vocabulary(name) {
                        Some(if is_epoch_shared(source.name()) {
                            source
                                .prefix()
                                .ok_or(ModelError::Conflict("scoped vocabulary source epoch"))?
                        } else if source.type_id()
                            == std::any::TypeId::of::<local_fields::FieldLocation>()
                            && field.name() == "qualification"
                        {
                            PublicationBoundary::Local
                        } else {
                            PublicationBoundary::Facts
                        })
                    } else if is_epoch_shared(name) {
                        return Err(ModelError::Conflict(
                            "scoped canonical value binding undeclared",
                        ));
                    } else {
                        None
                    };
                    if !declared
                        .iter()
                        .any(|input| input.type_id() == kind && input.prefix() == prefix)
                    {
                        return Err(ModelError::Conflict("scoped companion owner epoch"));
                    }
                    let mut matching = inputs
                        .iter()
                        .enumerate()
                        .filter(|(_, input)| input.type_id() == kind && input.prefix() == prefix);
                    let selected =
                        matching
                            .next()
                            .map(|(index, _)| index)
                            .ok_or(ModelError::Conflict(
                                "scoped companion immutable reader absent",
                            ))?;
                    if matching.next().is_some() {
                        return Err(ModelError::Conflict(
                            "scoped companion immutable reader ambiguous",
                        ));
                    }
                    Ok(Some(selected))
                })
                .collect()
        })
        .collect()
}

macro_rules! output {($($field:ident:$ty:ty,)*)=>{
    pub struct CatalogOutput {$(pub $field:Rows<$ty>,)*}
    impl CatalogOutput {
        pub fn new(budget:&ResourceBudget)->Self {Self {$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if relation==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
        pub fn validation_inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
        pub fn matches(&self,other:&Self)->Result<(),ModelError> {$(if !self.$field.same(&other.$field) {return Err(invalid(format!("catalog closure differs: {}",<$ty>::NAME)));})*Ok(())}
    }
};}
crate::catalog_outputs!(output);
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("catalog input absent: {}", R::NAME)))
}
fn option(
    out: &mut CatalogOutput,
    member: Id<CatalogMember>,
    subject: CatalogOptionSubject,
    evidence: CatalogOptionEvidence,
    default: CatalogDefault,
) -> Result<(), ModelError> {
    let subject = out.subjects.insert(subject)?;
    let evidence = out.evidence.insert(evidence)?;
    let default = out.defaults.insert(default)?;
    out.options.insert(CatalogOption {
        member,
        subject,
        evidence,
        default,
    })?;
    Ok(())
}
pub(super) fn callable(
    data: &CatalogPreparation<'_>,
    out: &mut CatalogOutput,
    member: Id<CatalogMember>,
    candidate: Id<CatalogCandidate>,
    id: Id<CallableEntity>,
    context: Id<attribution::AnalysisContext>,
) -> Result<(), ModelError> {
    if let Some(CallableEntity::Source { declaration, .. }) = data.source_callables.get(id) {
        for syntax in data.parameters_for_function(*declaration) {
            let q = need(&data.qualifications, syntax.qualification)?;
            if q.context != context
                || q.approximation != assertion::Approximation::Exact
                || q.modality != attribution::Modality::Definite
                || q.condition != conditions::Diagram::always().id()
            {
                continue;
            }
            let direct = ParameterEntity::Source {
                declaration: syntax.parameter,
            }
            .id();
            let mut parameter = data.parameters.get(direct).map(|_| (direct, None));
            if parameter.is_none() {
                for placement in data.placements_for_parent(Some(syntax.parameter)) {
                    let q = need(&data.qualifications, placement.qualification)?;
                    if q.context != context
                        || q.approximation != assertion::Approximation::Exact
                        || q.modality != attribution::Modality::Definite
                        || q.condition != conditions::Diagram::always().id()
                    {
                        continue;
                    }
                    if need(&data.occurrences, placement.occurrence)?.syntax_kind
                        != source::SyntaxKind::Parameter
                    {
                        continue;
                    }
                    let formal = ParameterEntity::Source {
                        declaration: placement.occurrence,
                    }
                    .id();
                    if data.parameters.get(formal).is_none() {
                        continue;
                    }
                    if parameter.is_some_and(|(old, _)| old != formal) {
                        return Err(invalid(
                            "source parameter has conflicting formal placements",
                        ));
                    }
                    parameter = Some((formal, Some(placement.id())));
                }
            }
            let Some((parameter, placement)) = parameter else {
                continue;
            };
            let default = match (syntax.default_literal, syntax.default) {
                (Some(literal), _) => CatalogDefault::Literal { literal },
                (_, Some(expression)) => CatalogDefault::Expression { expression },
                _ => CatalogDefault::Absent {},
            };
            option(
                out,
                member,
                CatalogOptionSubject::SourceParameter { parameter },
                CatalogOptionEvidence::SourceParameter {
                    syntax: syntax.id(),
                    placement,
                },
                default,
            )?;
        }
    }
    for assessment in data.assessments_for((id, context)) {
        let owner = out.callables.insert(CatalogCallable {
            member,
            candidate,
            assessment: assessment.id(),
            basis: if need(&out.candidates, candidate)?.alias.is_some() {
                CatalogContractBasis::SourceOnlyAlias
            } else {
                CatalogContractBasis::PublicCandidate
            },
        })?;
        for aspect in data.aspects_for(assessment.id()) {
            out.aspects.insert(CatalogCallableAspect {
                callable: owner,
                aspect: aspect.id(),
            })?;
        }
        for variant in data.variants_for((Some(id), context, Some(assessment.id()))) {
            out.invocations.insert(CatalogInvocation {
                callable: owner,
                variant: variant.id(),
            })?;
            for slot in data.slots_for(variant.id()) {
                let mut found = false;
                for entity in data.entities_for_slot(slot.id()) {
                    let link = need(&data.parameter_links, entity.link)?;
                    let ParameterEntity::Source { declaration } =
                        need(&data.parameters, link.entity)?
                    else {
                        continue;
                    };
                    for syntax in data.syntax_for_parameter(*declaration) {
                        if need(&data.qualifications, syntax.qualification)?.context != context {
                            continue;
                        }
                        found = true;
                        let default = match (syntax.default_literal, syntax.default) {
                            (Some(literal), _) => CatalogDefault::Literal { literal },
                            (_, Some(expression)) => CatalogDefault::Expression { expression },
                            _ => CatalogDefault::Absent {},
                        };
                        option(
                            out,
                            member,
                            CatalogOptionSubject::Parameter { slot: slot.id() },
                            CatalogOptionEvidence::Parameter {
                                entity: entity.id(),
                                syntax: syntax.id(),
                            },
                            default,
                        )?;
                    }
                }
                if !found {
                    let default = match slot.default {
                        DefaultSlot::Required | DefaultSlot::Collector => CatalogDefault::Absent {},
                        DefaultSlot::DefinitionTime | DefaultSlot::NativeUnknown => {
                            CatalogDefault::Unknown {}
                        }
                    };
                    option(
                        out,
                        member,
                        CatalogOptionSubject::Parameter { slot: slot.id() },
                        CatalogOptionEvidence::NativeParameter { slot: slot.id() },
                        default,
                    )?;
                }
            }
        }
    }
    Ok(())
}
/// Public slot keys depend on access ownership only. Every exposure and candidate is retained.
pub fn build(data: &CatalogData, budget: &ResourceBudget) -> Result<CatalogOutput, ModelError> {
    let preparation = CatalogPreparation::new(data, budget)?;
    let data = &preparation;
    let mut out = CatalogOutput::new(budget);
    for exposure in data.exposures.iter() {
        let module = need(&data.modules, exposure.access)?;
        let artifact = need(&data.artifacts, module.source)?;
        let name = need(&data.names, exposure.observation)?;
        if name.access != exposure.access {
            return Err(invalid("catalog exposure crosses access owner"));
        }
        let member = out.members.insert(CatalogMember {
            input: artifact.input,
            access: exposure.access,
            path: vec![name.name.clone()],
            name: name.name.clone(),
        })?;
        let link = out.exposures.insert(CatalogExposure {
            member,
            exposure: exposure.id(),
        })?;
        for candidate in data.exposure_candidates_for(exposure.id()) {
            let resolution = need(&data.resolutions, candidate.resolution)?;
            if resolution.context != exposure.context {
                return Err(invalid("catalog candidate crosses context"));
            }
            let mut known = data.entity_candidates_for(candidate.resolution).peekable();
            if known.peek().is_none() {
                out.candidates.insert(CatalogCandidate {
                    exposure: link,
                    candidate: Some(candidate.id()),
                    entity: None,
                    path: None,
                    alias: None,
                })?;
            }
            for entity_candidate in known {
                let catalog_candidate = out.candidates.insert(CatalogCandidate {
                    exposure: link,
                    candidate: Some(candidate.id()),
                    entity: Some(entity_candidate.id()),
                    path: None,
                    alias: None,
                })?;
                match need(&data.refs, entity_candidate.entity)? {
                    EntityRef::Callable { callable: id } => callable(
                        data,
                        &mut out,
                        member,
                        catalog_candidate,
                        *id,
                        exposure.context,
                    )?,
                    EntityRef::Class { class } => {
                        let catalog_class = out.classes.insert(CatalogClass {
                            member,
                            candidate: catalog_candidate,
                            class: *class,
                        })?;
                        data.validate_metadata()?;
                        for resolution in data.resolutions_for_entity((
                            exposure.context,
                            Some(EntityRef::Class { class: *class }.id()),
                        )) {
                            if resolution.status != ResolutionStatus::Resolved
                                || resolution.entity.and_then(|e| data.refs.get(e))
                                    != Some(&EntityRef::Class { class: *class })
                            {
                                continue;
                            }
                            for metadata in data.metadata_for(resolution.symbol) {
                                if need(&data.qualifications, metadata.qualification)?.context
                                    == exposure.context
                                {
                                    out.class_metadata.insert(CatalogClassMetadata {
                                        class: catalog_class,
                                        observation: metadata.id(),
                                    })?;
                                }
                            }
                            for metadata in data.members_for(resolution.symbol) {
                                if need(&data.qualifications, metadata.qualification)?.context
                                    == exposure.context
                                {
                                    out.class_members.insert(CatalogClassMember {
                                        class: catalog_class,
                                        observation: metadata.id(),
                                    })?;
                                }
                            }
                        }
                        constructors(data, &mut out, catalog_class, exposure.context)?;
                        fields(data, &mut out, member, *class, exposure.context)?;
                    }
                    _ => {}
                }
            }
        }
    }
    super::aliases::expand(data, &mut out, budget)?;
    super::paths::expand(data, &mut out, budget)?;
    classify_constructors(data, &mut out, budget)?;
    Ok(out)
}
pub(super) fn constructors(
    data: &CatalogPreparation<'_>,
    out: &mut CatalogOutput,
    class: Id<CatalogClass>,
    context: Id<attribution::AnalysisContext>,
) -> Result<(), ModelError> {
    let row = need(&out.classes, class)?.clone();
    data.validate_traits()?;
    if data.missing_trait_symbols.contains(&context) {
        return Err(invalid("catalog input absent: provider_symbols"));
    }
    if data
        .constructor_traits
        .get(&context)
        .is_none_or(|v| v.is_empty())
    {
        return Ok(());
    }
    // A source constructor belongs to an own or admitted ancestral defining symbol. Retain
    // every trait support and every ancestry edge; this is correspondence, not selection.
    let mut charge = StateCharge::new(
        data._charge.budget().expect("bound preparation"),
        "catalog-constructor-correspondence",
    );
    let mut own_symbols = crate::domain::charged::ChargedSet::default();
    let mut applicable: ConstructorMatches = Default::default();
    for resolution in
        data.resolutions_for_entity((context, Some(EntityRef::Class { class: row.class }.id())))
    {
        if resolution.status == ResolutionStatus::Resolved
            && resolution.entity.and_then(|e| data.refs.get(e))
                == Some(&EntityRef::Class { class: row.class })
        {
            own_symbols.insert(&mut charge, resolution.symbol)?;
        }
    }
    for symbol in own_symbols.iter() {
        for traits in data.traits_for_defining(Some(*symbol)) {
            append_index(
                &mut applicable,
                &mut charge,
                traits.id(),
                (None, Knowledge::Known),
            )?;
        }
    }
    let inherited = data
        .constructor_traits
        .get(&context)
        .into_iter()
        .flatten()
        .any(|traits| {
            traits
                .defining_class
                .is_some_and(|symbol| !own_symbols.contains(&symbol))
                && data
                    .resolutions_for_symbol((traits.symbol, context))
                    .any(|r| {
                        r.status == ResolutionStatus::Resolved
                            && matches!(
                                r.entity.and_then(|id| data.refs.get(id)),
                                Some(EntityRef::Callable { .. })
                            )
                    })
        });
    if inherited {
        data.validate_constructor_ancestry()?;
        for owner in
            data.resolutions_for_entity((context, Some(EntityRef::Class { class: row.class }.id())))
        {
            if owner.entity.and_then(|e| data.refs.get(e))
                != Some(&EntityRef::Class { class: row.class })
            {
                continue;
            }
            for parent in data.ancestry_for(owner.id()) {
                for ancestor in data.ancestors_for(parent.id()) {
                    let base = need(&data.resolutions, ancestor.resolution)?;
                    if base.context != context || own_symbols.contains(&base.symbol) {
                        continue;
                    }
                    for traits in data.traits_for_defining(Some(base.symbol)) {
                        if need(&data.qualifications, traits.qualification)?.context != context
                            || !matches!(
                                need(&data.symbols, traits.symbol)?.name.as_str(),
                                "__init__" | "__new__"
                            )
                            || !data
                                .resolutions_for_symbol((traits.symbol, context))
                                .any(|r| {
                                    r.status == ResolutionStatus::Resolved
                                        && matches!(
                                            r.entity.and_then(|id| data.refs.get(id)),
                                            Some(EntityRef::Callable { .. })
                                        )
                                })
                        {
                            continue;
                        }
                        let observation = need(&data.ancestry_observations, parent.observation)?;
                        if observation.relation != symbols::AncestryRelation::Mro {
                            continue;
                        }
                        let applicability = if parent.status == ResolutionStatus::Resolved
                            && observation.linearization == Some(Linearization::Complete)
                        {
                            Knowledge::Known
                        } else {
                            Knowledge::Unknown
                        };
                        append_index(
                            &mut applicable,
                            &mut charge,
                            traits.id(),
                            (Some(ancestor.id()), applicability),
                        )?;
                    }
                }
            }
        }
    }
    for (id, ancestry) in applicable.iter() {
        let traits = need(&data.traits, *id)?;
        if need(&data.qualifications, traits.qualification)?.context != context {
            continue;
        }
        let symbol = need(&data.symbols, traits.symbol)?;
        let kind = match symbol.name.as_str() {
            "__init__" => ConstructorKind::Init,
            "__new__" => ConstructorKind::New,
            _ => continue,
        };
        for resolution in data
            .resolutions_for_symbol((traits.symbol, context))
            .filter(|r| r.status == ResolutionStatus::Resolved)
        {
            let Some(EntityRef::Callable { callable: id }) =
                resolution.entity.and_then(|r| data.refs.get(r))
            else {
                continue;
            };
            for (ancestor, applicability) in ancestry {
                constructor(
                    data,
                    out,
                    &row,
                    class,
                    traits,
                    *id,
                    kind,
                    *ancestor,
                    *applicability,
                    context,
                )?;
            }
        }
    }
    Ok(())
}
#[expect(
    clippy::too_many_arguments,
    reason = "A constructor row binds the class, traits, callable, kind, ancestry, applicability and context"
)]
fn constructor(
    data: &CatalogPreparation<'_>,
    out: &mut CatalogOutput,
    row: &CatalogClass,
    class: Id<CatalogClass>,
    traits: &FunctionTraitObservation,
    id: Id<CallableEntity>,
    kind: ConstructorKind,
    ancestry: Option<Id<AncestryEntityMember>>,
    applicability: Knowledge,
    context: Id<attribution::AnalysisContext>,
) -> Result<(), ModelError> {
    callable(data, out, row.member, row.candidate, id, context)?;
    let candidate = need(&out.candidates, row.candidate)?;
    let exposure = need(
        &data.exposures,
        need(&out.exposures, candidate.exposure)?.exposure,
    )?;
    let public_known = candidate.alias.is_none()
        && exposure.status == ResolutionStatus::Resolved
        && exposure.publicity == crate::domain::normalized::entities::PublicPathKnowledge::Known;
    for assessment in data.assessments_for((id, context)) {
        let callable = CatalogCallable {
            member: row.member,
            candidate: row.candidate,
            assessment: assessment.id(),
            basis: if need(&out.candidates, row.candidate)?.alias.is_some() {
                CatalogContractBasis::SourceOnlyAlias
            } else {
                CatalogContractBasis::PublicCandidate
            },
        }
        .id();
        let origin = if traits.origin == FunctionOrigin::Synthesized {
            ConstructorOrigin::Synthetic
        } else if ancestry.is_some() {
            ConstructorOrigin::Inherited
        } else {
            ConstructorOrigin::Own
        };
        out.constructors.insert(CatalogConstructor {
            class,
            callable,
            traits: traits.id(),
            ancestry,
            origin,
            kind,
            disposition: ConstructorDisposition::Candidate,
            applicability: if !public_known {
                Knowledge::Unknown
            } else if assessment.identity == Knowledge::Known {
                applicability
            } else {
                assessment.identity
            },
        })?;
    }
    Ok(())
}
pub(super) fn fields(
    data: &CatalogPreparation<'_>,
    out: &mut CatalogOutput,
    member: Id<CatalogMember>,
    class: Id<ClassEntity>,
    context: Id<attribution::AnalysisContext>,
) -> Result<(), ModelError> {
    for field in data.fields_for(class) {
        for link in data.declarations_for_field(field.id()) {
            let syntax = need(&data.field_syntax, link.declaration)?;
            if need(&data.qualifications, syntax.qualification)?.context != context {
                continue;
            }
            for assessment in data.defaults_for(link.id()) {
                use normalized::callable_aspects::FieldDefault;
                let default = match need(&data.field_defaults, assessment.default)? {
                    FieldDefault::Absent {} => CatalogDefault::Absent {},
                    FieldDefault::Unknown {} => CatalogDefault::Unknown {},
                    FieldDefault::Unavailable {} => CatalogDefault::Unavailable {},
                    FieldDefault::Literal { literal, .. } => {
                        CatalogDefault::Literal { literal: *literal }
                    }
                    FieldDefault::Expression { expression } => CatalogDefault::Expression {
                        expression: *expression,
                    },
                    FieldDefault::Factory { expression, .. } => CatalogDefault::Factory {
                        expression: *expression,
                    },
                };
                option(
                    out,
                    member,
                    CatalogOptionSubject::Field { field: field.id() },
                    CatalogOptionEvidence::DeclaredField {
                        declaration: link.id(),
                        assessment: assessment.id(),
                    },
                    default,
                )?;
            }
            if data.defaults_for(link.id()).next().is_none() {
                return Err(invalid(
                    "catalog field requires completed normalized default assessment",
                ));
            }
        }
        for link in data.links_for_field(field.id()) {
            let observation = need(&data.field_observations, link.observation)?;
            if need(&data.qualifications, observation.qualification)?.context != context {
                continue;
            }
            let default = match observation.has_default {
                Some(false) => CatalogDefault::Absent {},
                Some(true) => CatalogDefault::Unknown {},
                None => CatalogDefault::Unavailable {},
            };
            option(
                out,
                member,
                CatalogOptionSubject::Field { field: field.id() },
                CatalogOptionEvidence::NativeField {
                    link: link.id(),
                    observation: observation.id(),
                },
                default,
            )?;
        }
    }
    Ok(())
}
fn constructor_rank(data: &CatalogData, row: &CatalogConstructor) -> Result<i64, ModelError> {
    match row.ancestry {
        None => Ok(-1),
        Some(id) => Ok(need(&data.sequence_members, need(&data.ancestors, id)?.member)?.ordinal),
    }
}
fn complete_signature_scope(
    data: &CatalogPreparation<'_>,
    qualification: Id<assertion::AssertionQualification>,
) -> Result<bool, ModelError> {
    let q = need(&data.qualifications, qualification)?;
    Ok(q.modality == attribution::Modality::Definite
        && q.approximation == assertion::Approximation::Exact
        && q.condition == conditions::Diagram::always().id()
        && data.coverage_for((q.scope, q.context)).any(|r| {
            r.family == attribution::FactFamily::Signatures
                && r.status == attribution::CoverageStatus::CompleteUnderStatedModel
        }))
}
fn classify_constructors(
    data: &CatalogPreparation<'_>,
    out: &mut CatalogOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut classified = Rows::new(budget);
    let mut charge = StateCharge::new(budget, "catalog-constructor-classification");
    let mut groups: std::collections::BTreeMap<(Id<CatalogClass>, i16), Vec<&CatalogConstructor>> =
        Default::default();
    for row in out.constructors.iter() {
        append_index(&mut groups, &mut charge, (row.class, row.kind as i16), row)?;
    }
    for row in out.constructors.iter() {
        let rank = constructor_rank(data, row)?;
        let class = need(&out.classes, row.class)?;
        let context = need(
            &data.qualifications,
            need(&data.traits, row.traits)?.qualification,
        )?
        .context;
        let own_complete = data
            .resolutions_for_entity((context, Some(EntityRef::Class { class: class.class }.id())))
            .filter(|r| {
                r.status == ResolutionStatus::Resolved
                    && r.entity.and_then(|e| data.refs.get(e))
                        == Some(&EntityRef::Class { class: class.class })
            })
            .any(|r| {
                data.class_traits_for(r.symbol)
                    .any(|t| complete_signature_scope(data, t.qualification).unwrap_or(false))
            });
        let mut closest = rank;
        let mut complete = own_complete;
        if let Some(row_ancestry) = row.ancestry {
            let ancestry = need(
                &data.ancestry,
                need(&data.ancestors, row_ancestry)?.assessment,
            )?;
            let observation = need(&data.ancestry_observations, ancestry.observation)?;
            complete &= ancestry.status == ResolutionStatus::Resolved
                && observation.linearization == Some(Linearization::Complete);
            for ancestor in data.ancestors_for(ancestry.id()) {
                let ordinal = need(&data.sequence_members, ancestor.member)?.ordinal;
                if ordinal >= rank {
                    continue;
                }
                let resolution = need(&data.resolutions, ancestor.resolution)?;
                complete &= resolution.status == ResolutionStatus::Resolved
                    && data
                        .class_traits_for(resolution.symbol)
                        .any(|t| complete_signature_scope(data, t.qualification).unwrap_or(false));
            }
        }
        for candidate in groups
            .get(&(row.class, row.kind as i16))
            .into_iter()
            .flatten()
            .copied()
        {
            closest = closest.min(constructor_rank(data, candidate)?);
            complete &= candidate.applicability == Knowledge::Known
                && complete_signature_scope(
                    data,
                    need(&data.traits, candidate.traits)?.qualification,
                )?;
        }
        // A single known nearest callable is required; traits from one callable can have multiple supports.
        let mut winner = None;
        let mut unique = true;
        for candidate in groups
            .get(&(row.class, row.kind as i16))
            .into_iter()
            .flatten()
            .copied()
        {
            if constructor_rank(data, candidate)? == closest {
                if winner.is_some_and(|id| id != candidate.callable) {
                    unique = false;
                }
                winner = Some(candidate.callable);
            }
        }
        let mut result = row.clone();
        if complete && unique {
            result.disposition = if rank == closest {
                ConstructorDisposition::Effective
            } else {
                ConstructorDisposition::Shadowed
            };
        }
        classified.insert(result)?;
    }
    out.constructors = classified;
    Ok(())
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = CatalogData::validation_inputs();
    inputs.extend(CatalogOutput::validation_inputs());
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 2,
        name: "catalog_core_closure",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(Check {
                data: CatalogData::new(budget),
                output: CatalogOutput::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct Check {
    data: CatalogData,
    output: CatalogOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if !self.data.visit(relation, batch)? && !self.output.visit(relation, batch)? {
            return Err(invalid("undeclared catalog validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.output.matches(&build(&self.data, &self.budget)?)
    }
}
pub fn stage(
    profile: stages::Profile,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    let mut inputs = crate::domain::normalized::coverage::stage(profile).inputs;
    inputs.extend(crate::domain::normalized::callable_aspects::stage(profile).inputs);
    inputs.extend(
        crate::domain::normalized::callable_aspects::relations()
            .iter()
            .map(|r| stages::RelationUse::of_relation(r).completed_input()),
    );
    inputs.extend(CatalogData::stage_inputs());
    inputs.extend(metadata_inputs());
    let outputs = stage_outputs();
    let roots = dependency_closure::DependencyClosure::roots_from_uses(model, &inputs)?;
    let inputs = dependency_closure::DependencyClosure::grants(
        model,
        roots,
        inputs,
        &outputs,
        stages::PublicationBoundary::Facts,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    Ok(stages::Stage {
        captured_binding: None,
        name: "catalog_core",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: ContentHash::of(include_bytes!("build.rs")),
        configuration: ContentHash::of(b"mandatory-catalog/v1"),
    })
}
/// Complete this definition once in the compilation's early authored configuration stage.
pub fn definition() -> (analysis::MethodParameters, analysis::AnalysisDefinition) {
    let parameters = analysis::MethodParameters {
        depth: None,
        proof_steps: None,
        work: None,
        members: None,
        seed: None,
        iterations: None,
        threshold: None,
        resolution: None,
        damping: None,
        model_catalog: None,
    };
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::Catalog,
        semantic_version: ContentHash::of(b"catalog-core/v1"),
        parameters: parameters.id(),
        interpretation: analysis::Interpretation::Structural,
    };
    (parameters, definition)
}
pub fn metadata_inputs() -> Vec<stages::RelationUse> {
    use crate::domain::{
        attribution::{AnalysisContext, ProviderRun},
        input::{ArtifactUse, InputRevision},
        normalized::coverage::{NormalizationComputation, NormalizationCoverage},
        source::CoverageScope,
    };
    vec![
        stages::RelationUse::completed::<InputRevision>(),
        stages::RelationUse::completed::<ArtifactUse>(),
        stages::RelationUse::completed::<CoverageScope>(),
        stages::RelationUse::completed::<NormalizationComputation>(),
        stages::RelationUse::completed::<NormalizationCoverage>(),
        stages::RelationUse::completed::<analysis::AnalysisDefinition>(),
        stages::RelationUse::completed::<analysis::MethodParameters>(),
        stages::RelationUse::completed::<AnalysisContext>(),
        stages::RelationUse::completed::<ProviderRun>(),
    ]
}
fn stage_outputs() -> Vec<stages::RelationUse> {
    let mut outputs = super::core_relations()
        .iter()
        .map(stages::RelationUse::of_relation)
        .collect::<Vec<_>>();
    outputs.extend(
        analysis::catalog_core::publication_relations()
            .iter()
            .map(stages::RelationUse::of_relation),
    );
    outputs
}
/// Map every retained slot/context to its own admitted computation, even without signatures.
pub fn invocation_links(
    data: &CatalogData,
    output: &CatalogOutput,
    invocations: &Rows<analysis::catalog_core::Invocation>,
    budget: &ResourceBudget,
) -> Result<Rows<CatalogMemberInvocation>, ModelError> {
    let mut links = Rows::new(budget);
    let mut charge = StateCharge::new(budget, "catalog-invocation-correspondence");
    let mut frames: std::collections::BTreeMap<
        CatalogInvocationFrame,
        Vec<&analysis::catalog_core::Invocation>,
    > = Default::default();
    for invocation in invocations.iter() {
        append_index(
            &mut frames,
            &mut charge,
            (invocation.input, invocation.context),
            invocation,
        )?;
    }
    for exposure in output.exposures.iter() {
        let context = need(&data.exposures, exposure.exposure)?.context;
        let member = need(&output.members, exposure.member)?;
        let mut count = 0;
        for invocation in frames.get(&(member.input, context)).into_iter().flatten() {
            links.insert(CatalogMemberInvocation {
                member: member.id(),
                invocation: invocation.id(),
            })?;
            count += 1;
        }
        if count != 1 {
            return Err(invalid(
                "catalog slot/context requires exactly one admitted computation",
            ));
        }
    }
    Ok(links)
}
pub fn invocation_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: InvariantPurpose::Admission,
        revision: 2,
        name: "catalog_member_invocation_closure",
        inputs: vec![
            ValidationInput::of::<normalized::entities::PublicExposure>(&["id"]),
            ValidationInput::of::<CatalogMember>(&["id"]),
            ValidationInput::of::<CatalogExposure>(&["id"]),
            ValidationInput::of::<analysis::catalog_core::Invocation>(&["id"]),
            ValidationInput::of::<CatalogMemberInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(InvocationCheck {
                public: Default::default(),
                members: Default::default(),
                exposures: Default::default(),
                invocations: Default::default(),
                links: Default::default(),
                charge: StateCharge::new(budget, "catalog-invocation-membership"),
            })
        }),
    }]
}
type CatalogInvocationFrame = (Id<input::InputRevision>, Id<attribution::AnalysisContext>);
struct InvocationCheck {
    public: ChargedMap<Id<normalized::entities::PublicExposure>, Id<attribution::AnalysisContext>>,
    members: ChargedMap<Id<CatalogMember>, Id<input::InputRevision>>,
    exposures: ChargedMap<
        Id<CatalogExposure>,
        (Id<CatalogMember>, Id<normalized::entities::PublicExposure>),
    >,
    invocations: ChargedMap<CatalogInvocationFrame, Vec<Id<analysis::catalog_core::Invocation>>>,
    links: crate::domain::charged::ChargedSet<(
        Id<CatalogMember>,
        Id<analysis::catalog_core::Invocation>,
    )>,
    charge: StateCharge,
}
impl InvariantCheck for InvocationCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == normalized::entities::PublicExposure::NAME {
            for row in normalized::entities::PublicExposure::decode(batch)? {
                self.public
                    .insert(&mut self.charge, row.id(), row.context)?;
            }
        } else if relation == CatalogMember::NAME {
            for row in CatalogMember::decode(batch)? {
                self.members.insert(&mut self.charge, row.id(), row.input)?;
            }
        } else if relation == CatalogExposure::NAME {
            for row in CatalogExposure::decode(batch)? {
                self.exposures
                    .insert(&mut self.charge, row.id(), (row.member, row.exposure))?;
            }
        } else if relation == analysis::catalog_core::Invocation::NAME {
            for row in analysis::catalog_core::Invocation::decode(batch)? {
                self.invocations.push_unique(
                    &mut self.charge,
                    (row.input, row.context),
                    row.id(),
                )?;
            }
        } else if relation == CatalogMemberInvocation::NAME {
            for row in CatalogMemberInvocation::decode(batch)? {
                self.links
                    .insert(&mut self.charge, (row.member, row.invocation))?;
            }
        } else {
            return Err(invalid("undeclared catalog invocation input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        let mut expected = crate::domain::charged::ChargedSet::default();
        for (member, exposure) in self.exposures.values() {
            let input = *self
                .members
                .get(member)
                .ok_or_else(|| invalid("catalog invocation member absent"))?;
            let context = *self
                .public
                .get(exposure)
                .ok_or_else(|| invalid("catalog invocation public exposure absent"))?;
            let invocations = self.invocations.get(&(input, context)).ok_or_else(|| {
                invalid("catalog slot/context requires exactly one admitted computation")
            })?;
            if invocations.len() != 1 {
                return Err(invalid(
                    "catalog slot/context requires exactly one admitted computation",
                ));
            }
            expected.insert(&mut self.charge, (*member, invocations[0]))?;
        }
        if *expected != *self.links {
            return Err(invalid("catalog invocation closure differs"));
        }
        Ok(())
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["catalog_core_closure"]
}
pub(crate) fn invocation_invariants_refs() -> Vec<&'static str> {
    vec!["catalog_member_invocation_closure"]
}

#[cfg(test)]
mod compact_invocation_controls {
    use super::*;
    fn nominal<R>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    fn invocation(
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> analysis::catalog_core::Invocation {
        // Decode a real generated invocation shape rather than inventing a parallel DTO.
        let (parameters, definition) = definition();
        let sources = analysis::sources::CapturedSources::capture(
            stages::Profile::Catalog,
            [],
            &ResourceBudget::fixed(1 << 20).unwrap(),
        )
        .unwrap();
        let _ = parameters;
        analysis::catalog_core::Invocation::admitted(
            input,
            context,
            definition.id(),
            None,
            [],
            &sources,
            [],
            &ResourceBudget::fixed(1 << 20).unwrap(),
        )
        .unwrap()
        .0
    }
    fn feed<R: Record>(check: &mut dyn InvariantCheck, rows: &[R]) {
        check.visit(R::NAME, &R::encode(rows).unwrap()).unwrap();
    }
    #[test]
    fn exact_catalog_frame_membership_drops_member_labels_and_refuses_foreign_link() {
        let budget = ResourceBudget::fixed(64 << 10).unwrap();
        let public = PublicExposure {
            access: nominal(1),
            context: nominal(2),
            observation: nominal(3),
            origin: nominal(4),
            enumeration: None,
            publicity: PublicPathKnowledge::Unknown,
            status: ResolutionStatus::Unresolved,
            reason: EntityReason::MissingDeclaration,
        };
        let member = CatalogMember {
            input: nominal(5),
            access: public.access,
            path: vec!["x".repeat(1 << 20)],
            name: "x".repeat(1 << 20),
        };
        let exposure = CatalogExposure {
            member: member.id(),
            exposure: public.id(),
        };
        let valid = invocation(member.input, public.context);
        let foreign = invocation(nominal(6), public.context);
        let invariant = invocation_invariants().pop().unwrap();
        let mut check = (invariant.create)(&budget);
        feed(check.as_mut(), std::slice::from_ref(&public));
        feed(check.as_mut(), std::slice::from_ref(&member));
        feed(check.as_mut(), std::slice::from_ref(&exposure));
        feed(check.as_mut(), &[valid.clone(), foreign.clone()]);
        feed(
            check.as_mut(),
            &[CatalogMemberInvocation {
                member: member.id(),
                invocation: valid.id(),
            }],
        );
        check.finish().unwrap();
        assert_eq!(budget.reserved(), 0);
        let mut check = (invariant.create)(&budget);
        feed(check.as_mut(), &[public]);
        feed(check.as_mut(), std::slice::from_ref(&member));
        feed(check.as_mut(), &[exposure]);
        feed(check.as_mut(), &[valid, foreign.clone()]);
        feed(
            check.as_mut(),
            &[CatalogMemberInvocation {
                member: member.id(),
                invocation: foreign.id(),
            }],
        );
        assert!(
            check
                .finish()
                .unwrap_err()
                .to_string()
                .contains("closure differs")
        );
        assert_eq!(budget.reserved(), 0);
    }
}

#[cfg(test)]
mod scoped_binding_controls {
    use super::*;
    use stages::PublicationBoundary::{Facts, Local};
    use std::any::TypeId;

    #[test]
    fn local_location_binds_qualification_and_placement_to_different_owners() {
        let model = crate::domain::model().unwrap();
        let inputs = vec![
            ValidationInput::of::<local_fields::FieldLocation>(&["id"]),
            ValidationInput::of::<assertion::AssertionQualification>(&["id"]).at_epoch(Facts),
            ValidationInput::of::<assertion::AssertionQualification>(&["id"]).at_epoch(Local),
            ValidationInput::of::<syntax::SyntaxPlacement>(&["id"]),
        ];
        let bindings = scoped_field_bindings(&model, &inputs, &inputs, |_, _| false).unwrap();
        let relation = model.relation(local_fields::FieldLocation::NAME).unwrap();
        let field = |name| {
            relation
                .fields()
                .iter()
                .position(|f| f.name() == name)
                .unwrap()
        };
        assert_eq!(bindings[0][field("qualification")], Some(2));
        assert_eq!(bindings[0][field("placement")], Some(3));
        let missing = vec![inputs[0].clone(), inputs[1].clone(), inputs[3].clone()];
        assert!(scoped_field_bindings(&model, &missing, &inputs, |_, _| false).is_err());
    }

    #[test]
    fn consumed_vocabulary_dependencies_inherit_the_exact_source_view() {
        let model = crate::domain::model().unwrap();
        let inputs = vec![
            ValidationInput::of::<assertion::AssertionQualification>(&["id"]).at_epoch(Local),
            ValidationInput::of::<conditions::Condition>(&["id"]).at_epoch(Facts),
            ValidationInput::of::<conditions::Condition>(&["id"]).at_epoch(Local),
        ];
        let bindings = scoped_field_bindings(&model, &inputs, &inputs, |_, _| false).unwrap();
        let relation = model
            .relation(assertion::AssertionQualification::NAME)
            .unwrap();
        let field = relation
            .fields()
            .iter()
            .position(|f| {
                f.target()
                    .is_some_and(|(kind, _)| kind == TypeId::of::<conditions::Condition>())
            })
            .unwrap();
        assert_eq!(bindings[0][field], Some(2));
        let missing = vec![inputs[0].clone(), inputs[1].clone()];
        assert!(scoped_field_bindings(&model, &missing, &inputs, |_, _| false).is_err());
        let mut unbound = inputs.clone();
        unbound[0] = ValidationInput::of::<assertion::AssertionQualification>(&["id"]);
        assert!(scoped_field_bindings(&model, &unbound, &inputs, |_, _| false).is_err());
    }
}

impl CatalogData {
    /// C0's value contract names every selected typed input, including explicit empty rows.
    /// Its sole attribution-only projection is native coverage: signature completeness reads
    /// scope/context/family/status, and never the reporting provider, run or diagnostics.
    pub fn value_dependencies(
        &self,
        domain: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<Vec<crate::domain::compilation_product::DependencyToken>, ModelError> {
        use crate::domain::compilation_product::{RelationValue, RelationValueOutcome};
        let mut values = Vec::new();
        macro_rules! rows {($($field:ident:$ty:ty,)*)=>{$(
            if <$ty>::NAME!=attribution::ProviderCoverage::NAME {
                values.push(RelationValue::of_rows::<$ty>(domain,RelationValueOutcome::Complete,self.$field.iter())?
                    .dependency(stringify!($field),None)?);
            }
        )*};}
        crate::catalog_inputs!(rows);
        let _sort = budget.reserve(
            "catalog-native-coverage-value",
            self.native_coverage.len().saturating_mul(128),
        )?;
        let mut coverage = self
            .native_coverage
            .iter()
            .map(|row| (row.scope, row.context, row.family, row.status))
            .collect::<Vec<_>>();
        coverage.sort_by_key(|(scope, context, family, status)| {
            (*scope, *context, *family as i16, *status as i16)
        });
        let mut content = KeySink::new("catalog-signature-completeness-projection/v1");
        for (scope, context, family, status) in &coverage {
            scope.encode(&mut content);
            context.encode(&mut content);
            content.part(b"family", &(*family as i16).to_le_bytes());
            content.part(b"status", &(*status as i16).to_le_bytes());
        }
        values.push(
            RelationValue {
                relation: attribution::ProviderCoverage::NAME.into(),
                contract: ContentHash::of(
                    b"catalog-signature-completeness:scope/context/family/status/v1",
                ),
                domain,
                outcome: RelationValueOutcome::Complete,
                rows: coverage.len() as u64,
                content: content.finish(),
            }
            .dependency("native_coverage", None)?,
        );
        Ok(values)
    }
}

#[cfg(test)]
mod correspondence_controls {
    use super::*;
    fn nominal<R>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    #[test]
    fn constructor_correspondence_does_not_read_an_unmatched_mro_observation() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut data = CatalogData::new(&budget);
        let q = assertion::AssertionQualification {
            context: nominal(1),
            scope: nominal(2),
            assumptions: assumptions::AssumptionSet::empty_id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
        };
        data.qualifications.insert(q.clone()).unwrap();
        let class = nominal(3);
        let class_ref = data.refs.insert(EntityRef::Class { class }).unwrap();
        let base_ref = data
            .refs
            .insert(EntityRef::Class { class: nominal(4) })
            .unwrap();
        let callable_ref = data
            .refs
            .insert(EntityRef::Callable {
                callable: nominal(5),
            })
            .unwrap();
        let method = calls::ProviderSymbol {
            provider: nominal(6),
            context: q.context,
            module: nominal(7),
            native_key: "Other.__init__".into(),
            name: "__init__".into(),
            kind: calls::SymbolKind::Method,
        };
        data.symbols.insert(method.clone()).unwrap();
        let resolution = |symbol, entity| SymbolEntityResolution {
            symbol,
            entity: Some(entity),
            context: q.context,
            policy: ContentHash::of(b"test"),
            status: ResolutionStatus::Resolved,
            reason: EntityReason::DeclarationAgreement,
        };
        let owner = resolution(nominal(8), class_ref);
        let base = resolution(nominal(9), base_ref);
        data.resolutions.insert(owner.clone()).unwrap();
        data.resolutions.insert(base.clone()).unwrap();
        data.resolutions
            .insert(resolution(method.id(), callable_ref))
            .unwrap();
        data.traits
            .insert(FunctionTraitObservation {
                qualification: q.id(),
                symbol: method.id(),
                overload: false,
                staticmethod: false,
                classmethod: false,
                property_getter: false,
                property_setter: false,
                stub: false,
                origin: FunctionOrigin::DefStatement,
                defining_class: Some(nominal(10)),
                overrides: None,
            })
            .unwrap();
        let ancestry = AncestryEntityAssessment {
            observation: nominal(11),
            class: owner.id(),
            status: ResolutionStatus::Ambiguous,
            reason: normalized::links::LinkReason::IncompleteInput,
        };
        data.ancestry.insert(ancestry.clone()).unwrap();
        data.ancestors
            .insert(AncestryEntityMember {
                assessment: ancestry.id(),
                member: nominal(12),
                resolution: base.id(),
            })
            .unwrap();
        let prepared = CatalogPreparation::new(&data, &budget).unwrap();
        let mut output = CatalogOutput::new(&budget);
        let row = output
            .classes
            .insert(CatalogClass {
                member: nominal(13),
                candidate: nominal(14),
                class,
            })
            .unwrap();
        // The admitted base has no trait for the requested constructor's defining symbol.
        // No matching route consumes either the absent observation or sequence member.
        constructors(&prepared, &mut output, row, q.context).unwrap();
        assert_eq!(output.constructors.len(), 0);
        drop(output);
        drop(prepared);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
}

#[cfg(test)]
mod metadata_demand_controls {
    use super::*;
    #[test]
    fn catalog_metadata_declares_only_complete_expected_domains_and_authored_metadata() {
        let declared = metadata_validation_inputs()
            .into_iter()
            .map(|input| input.name())
            .collect::<std::collections::BTreeSet<_>>();
        let mut expected = analysis::expected::inputs(analysis::AnalysisMethod::Catalog)
            .into_iter()
            .map(|input| input.name())
            .collect::<std::collections::BTreeSet<_>>();
        expected.extend([
            analysis::AnalysisDefinition::NAME,
            analysis::MethodParameters::NAME,
            attribution::ProviderRun::NAME,
        ]);
        assert_eq!(declared, expected);
        assert!(!declared.contains(calls::ProviderSymbol::NAME));
        assert!(
            CatalogData::validation_inputs()
                .iter()
                .any(|input| input.name() == calls::ProviderSymbol::NAME),
            "selected rich catalog kernels retain their premises"
        );
        assert!(declared.contains(source::SourceArtifact::NAME));
        assert!(declared.contains(source::CoverageScope::NAME));
        assert!(
            !declared.contains(attribution::ProviderCoverage::NAME),
            "C0's expected domain is a normalized contract; native receipt checks remain with its rich premises"
        );
        assert!(
            CatalogData::validation_inputs()
                .iter()
                .any(|input| input.name() == attribution::ProviderCoverage::NAME),
            "native qualification obligations remain independently declared for rich C0 admission"
        );
        assert!(declared.contains(normalized::coverage::NormalizationCoverage::NAME));
    }
}
