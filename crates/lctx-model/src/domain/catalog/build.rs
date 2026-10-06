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
    }
};}
crate::catalog_inputs!(data);
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
    data: &CatalogData,
    out: &mut CatalogOutput,
    member: Id<CatalogMember>,
    candidate: Id<CatalogCandidate>,
    id: Id<CallableEntity>,
    context: Id<attribution::AnalysisContext>,
) -> Result<(), ModelError> {
    if let Some(CallableEntity::Source { declaration, .. }) = data.source_callables.get(id) {
        for syntax in data
            .parameter_syntax
            .iter()
            .filter(|r| r.function == *declaration)
        {
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
                for placement in data
                    .placements
                    .iter()
                    .filter(|p| p.parent == Some(syntax.parameter))
                {
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
    for assessment in data
        .assessments
        .iter()
        .filter(|r| r.callable == id && r.context == context)
    {
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
        for aspect in data
            .aspects
            .iter()
            .filter(|r| r.assessment == assessment.id())
        {
            out.aspects.insert(CatalogCallableAspect {
                callable: owner,
                aspect: aspect.id(),
            })?;
        }
        for variant in data.variants.iter().filter(|r| {
            r.callable == Some(id) && r.context == context && r.assessment == Some(assessment.id())
        }) {
            out.invocations.insert(CatalogInvocation {
                callable: owner,
                variant: variant.id(),
            })?;
            for slot in data.slots.iter().filter(|r| r.variant == variant.id()) {
                let mut found = false;
                for entity in data.slot_entities.iter().filter(|r| r.slot == slot.id()) {
                    let link = need(&data.parameter_links, entity.link)?;
                    let ParameterEntity::Source { declaration } =
                        need(&data.parameters, link.entity)?
                    else {
                        continue;
                    };
                    for syntax in data
                        .parameter_syntax
                        .iter()
                        .filter(|r| r.parameter == *declaration)
                    {
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
    let mut out = CatalogOutput::new(budget);
    let mut charge = StateCharge::new(budget, "catalog-index");
    let mut candidates: ChargedMap<Id<SymbolEntityResolution>, Vec<&SymbolEntityCandidate>> =
        Default::default();
    for candidate in data.entity_candidates.iter() {
        candidates.update(&mut charge, candidate.resolution, |v| v.push(candidate))?;
    }
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
        for candidate in data
            .exposure_candidates
            .iter()
            .filter(|r| r.exposure == exposure.id())
        {
            let resolution = need(&data.resolutions, candidate.resolution)?;
            if resolution.context != exposure.context {
                return Err(invalid("catalog candidate crosses context"));
            }
            let known = candidates.get(&candidate.resolution);
            if known.is_none_or(|r| r.is_empty()) {
                out.candidates.insert(CatalogCandidate {
                    exposure: link,
                    candidate: Some(candidate.id()),
                    entity: None,
                    path: None,
                    alias: None,
                })?;
            }
            for entity_candidate in known.into_iter().flatten() {
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
                        for metadata in data.class_metadata.iter() {
                            let q = need(&data.qualifications, metadata.qualification)?;
                            if q.context != exposure.context {
                                continue;
                            }
                            if data.resolutions.iter().any(|r| {
                                r.symbol == metadata.class
                                    && r.context == exposure.context
                                    && r.status == ResolutionStatus::Resolved
                                    && r.entity.and_then(|e| data.refs.get(e))
                                        == Some(&EntityRef::Class { class: *class })
                            }) {
                                out.class_metadata.insert(CatalogClassMetadata {
                                    class: catalog_class,
                                    observation: metadata.id(),
                                })?;
                            }
                        }
                        for metadata in data.class_members.iter() {
                            let q = need(&data.qualifications, metadata.qualification)?;
                            if q.context != exposure.context {
                                continue;
                            }
                            if data.resolutions.iter().any(|r| {
                                r.symbol == metadata.class
                                    && r.context == exposure.context
                                    && r.status == ResolutionStatus::Resolved
                                    && r.entity.and_then(|e| data.refs.get(e))
                                        == Some(&EntityRef::Class { class: *class })
                            }) {
                                out.class_members.insert(CatalogClassMember {
                                    class: catalog_class,
                                    observation: metadata.id(),
                                })?;
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
    data: &CatalogData,
    out: &mut CatalogOutput,
    class: Id<CatalogClass>,
    context: Id<attribution::AnalysisContext>,
) -> Result<(), ModelError> {
    let row = need(&out.classes, class)?.clone();
    for traits in data.traits.iter() {
        if need(&data.qualifications, traits.qualification)?.context != context {
            continue;
        }
        let symbol = need(&data.symbols, traits.symbol)?;
        let kind = match symbol.name.as_str() {
            "__init__" => ConstructorKind::Init,
            "__new__" => ConstructorKind::New,
            _ => continue,
        };
        let Some(defining) = traits.defining_class else {
            continue;
        };
        let own = data.resolutions.iter().any(|r| {
            r.symbol == defining
                && r.context == context
                && r.status == ResolutionStatus::Resolved
                && r.entity.and_then(|e| data.refs.get(e))
                    == Some(&EntityRef::Class { class: row.class })
        });
        for resolution in data.resolutions.iter().filter(|r| {
            r.symbol == traits.symbol
                && r.context == context
                && r.status == ResolutionStatus::Resolved
        }) {
            let Some(EntityRef::Callable { callable: id }) =
                resolution.entity.and_then(|r| data.refs.get(r))
            else {
                continue;
            };
            if own {
                constructor(
                    data,
                    out,
                    &row,
                    class,
                    traits,
                    *id,
                    kind,
                    None,
                    Knowledge::Known,
                    context,
                )?;
            } else {
                for ancestor in data.ancestors.iter() {
                    let parent = need(&data.ancestry, ancestor.assessment)?;
                    let owner = need(&data.resolutions, parent.class)?;
                    if owner.context != context
                        || owner.entity.and_then(|e| data.refs.get(e))
                            != Some(&EntityRef::Class { class: row.class })
                    {
                        continue;
                    }
                    let base = need(&data.resolutions, ancestor.resolution)?;
                    if base.symbol != defining || base.context != context {
                        continue;
                    }
                    let observation = need(&data.ancestry_observations, parent.observation)?;
                    if observation.relation != symbols::AncestryRelation::Mro {
                        continue;
                    }
                    // Even a complete MRO does not independently establish constructor selection.
                    let applicability = if parent.status == ResolutionStatus::Resolved
                        && observation.linearization == Some(Linearization::Complete)
                    {
                        Knowledge::Known
                    } else {
                        Knowledge::Unknown
                    };
                    constructor(
                        data,
                        out,
                        &row,
                        class,
                        traits,
                        *id,
                        kind,
                        Some(ancestor.id()),
                        applicability,
                        context,
                    )?;
                }
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
    data: &CatalogData,
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
    for assessment in data
        .assessments
        .iter()
        .filter(|r| r.callable == id && r.context == context)
    {
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
    data: &CatalogData,
    out: &mut CatalogOutput,
    member: Id<CatalogMember>,
    class: Id<ClassEntity>,
    context: Id<attribution::AnalysisContext>,
) -> Result<(), ModelError> {
    for field in data.fields.iter().filter(|r| r.class == class) {
        for link in data
            .field_declarations
            .iter()
            .filter(|r| r.field == field.id())
        {
            let syntax = need(&data.field_syntax, link.declaration)?;
            if need(&data.qualifications, syntax.qualification)?.context != context {
                continue;
            }
            for assessment in data
                .field_default_assessments
                .iter()
                .filter(|r| r.declaration == link.id())
            {
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
            if !data
                .field_default_assessments
                .iter()
                .any(|r| r.declaration == link.id())
            {
                return Err(invalid(
                    "catalog field requires completed normalized default assessment",
                ));
            }
        }
        for link in data.field_links.iter().filter(|r| r.field == field.id()) {
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
    data: &CatalogData,
    qualification: Id<assertion::AssertionQualification>,
) -> Result<bool, ModelError> {
    let q = need(&data.qualifications, qualification)?;
    Ok(q.modality == attribution::Modality::Definite
        && q.approximation == assertion::Approximation::Exact
        && q.condition == conditions::Diagram::always().id()
        && data.native_coverage.iter().any(|r| {
            r.scope == q.scope
                && r.context == q.context
                && r.family == attribution::FactFamily::Signatures
                && r.status == attribution::CoverageStatus::CompleteUnderStatedModel
        }))
}
fn classify_constructors(
    data: &CatalogData,
    out: &mut CatalogOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut classified = Rows::new(budget);
    for row in out.constructors.iter() {
        let rank = constructor_rank(data, row)?;
        let class = need(&out.classes, row.class)?;
        let context = need(
            &data.qualifications,
            need(&data.traits, row.traits)?.qualification,
        )?
        .context;
        let own_complete = data
            .resolutions
            .iter()
            .filter(|r| {
                r.context == context
                    && r.status == ResolutionStatus::Resolved
                    && r.entity.and_then(|e| data.refs.get(e))
                        == Some(&EntityRef::Class { class: class.class })
            })
            .any(|r| {
                data.class_traits.iter().any(|t| {
                    t.symbol == r.symbol
                        && complete_signature_scope(data, t.qualification).unwrap_or(false)
                })
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
            for ancestor in data
                .ancestors
                .iter()
                .filter(|a| a.assessment == ancestry.id())
            {
                let ordinal = need(&data.sequence_members, ancestor.member)?.ordinal;
                if ordinal >= rank {
                    continue;
                }
                let resolution = need(&data.resolutions, ancestor.resolution)?;
                complete &= resolution.status == ResolutionStatus::Resolved
                    && data.class_traits.iter().any(|t| {
                        t.symbol == resolution.symbol
                            && complete_signature_scope(data, t.qualification).unwrap_or(false)
                    });
            }
        }
        for candidate in out
            .constructors
            .iter()
            .filter(|r| r.class == row.class && r.kind == row.kind)
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
        for candidate in out
            .constructors
            .iter()
            .filter(|r| r.class == row.class && r.kind == row.kind)
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
    inputs.extend(
        crate::domain::analysis::preparation::configuration_relations()
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
    for exposure in output.exposures.iter() {
        let context = need(&data.exposures, exposure.exposure)?.context;
        let member = need(&output.members, exposure.member)?;
        let mut count = 0;
        for invocation in invocations
            .iter()
            .filter(|r| r.input == member.input && r.context == context)
        {
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
struct InvocationCheck {
    public: ChargedMap<Id<normalized::entities::PublicExposure>, Id<attribution::AnalysisContext>>,
    members: ChargedMap<Id<CatalogMember>, Id<input::InputRevision>>,
    exposures: ChargedMap<
        Id<CatalogExposure>,
        (Id<CatalogMember>, Id<normalized::entities::PublicExposure>),
    >,
    invocations: ChargedMap<
        (Id<input::InputRevision>, Id<attribution::AnalysisContext>),
        Vec<Id<analysis::catalog_core::Invocation>>,
    >,
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
                self.invocations
                    .update(&mut self.charge, (row.input, row.context), |ids| {
                        if !ids.contains(&row.id()) {
                            ids.push(row.id());
                        }
                    })?;
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
        feed(check.as_mut(), &[public.clone()]);
        feed(check.as_mut(), &[member.clone()]);
        feed(check.as_mut(), &[exposure.clone()]);
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
        feed(check.as_mut(), &[member.clone()]);
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
