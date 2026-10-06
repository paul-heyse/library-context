//! N3's single semantic operation, used both by materialization and publication validation.
use super::{Rows, callables::*, entities::*, links::*, policy_revision};
use crate::domain::{
    assertion::AssertionQualification,
    attribution::*,
    calls::*,
    charged::{ChargedMap, ChargedSet, StateCharge},
    lexical::*,
    resources::ResourceBudget,
    source::*,
    symbols::FunctionTraitObservation,
    syntax::*,
    types::{FunctionBodyObservation, NativeReceiver, SignatureTypeSubject},
    *,
};
macro_rules! inputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct CallableData { $(pub $field: Rows<$ty>,)* }
        impl CallableData {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { super::facts_inputs(vec![$(ValidationInput::of::<$ty>(&["id"]),)*]) }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::completed::<$ty>()),*] }
        }
    }
}
crate::normalized_callable_inputs!(inputs);
macro_rules! outputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct CallableOutput { $(pub $field: Rows<$ty>,)* }
        impl CallableOutput {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn matches(&self, expected: &Self) -> Result<(), ModelError> {
                $(if !self.$field.same(&expected.$field) { return Err(invalid(format!("normalized callable closure differs: {}", <$ty>::NAME))); })* Ok(())
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
        }
    }
}
crate::normalized_callable_outputs!(outputs);
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("normalized callable requires {}", R::NAME)))
}
fn context(
    data: &CallableData,
    qualification: Id<AssertionQualification>,
) -> Result<Id<AnalysisContext>, ModelError> {
    Ok(need(&data.qualifications, qualification)?.context)
}
fn exact(
    data: &CallableData,
    qualification: Id<AssertionQualification>,
) -> Result<bool, ModelError> {
    let q = need(&data.qualifications, qualification)?;
    Ok(q.modality == Modality::Definite
        && q.approximation == assertion::Approximation::Exact
        && q.condition == conditions::Diagram::always().id())
}
type CallableContext = (Id<CallableEntity>, Id<AnalysisContext>);
struct Index<'a> {
    universe: ChargedSet<CallableContext>,
    resolutions: ChargedMap<Id<ProviderSymbol>, &'a SymbolEntityResolution>,
    mappings: ChargedMap<CallableContext, Vec<&'a SymbolEntityResolution>>,
    traits: ChargedMap<CallableContext, Vec<&'a FunctionTraitObservation>>,
    signatures: ChargedMap<CallableContext, Vec<&'a Signature>>,
    coverage: ChargedMap<(Id<SourceArtifact>, Id<AnalysisContext>), Vec<&'a ProviderCoverage>>,
    declarations:
        ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a DeclarationObservation>>,
    decorators: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a DeclarationDecorator>>,
    bodies: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a FunctionBodyObservation>>,
    children: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a SyntaxPlacement>>,
    references:
        ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a ReferenceEntityAssessment>>,
    targets: ChargedMap<Id<ReferenceEntityAssessment>, Vec<&'a ReferenceEntityCandidate>>,
    generators: ChargedSet<Id<Occurrence>>,
    _charge: StateCharge,
}
fn callable(
    data: &CallableData,
    resolution: &SymbolEntityResolution,
) -> Option<Id<CallableEntity>> {
    if resolution.status != ResolutionStatus::Resolved {
        return None;
    }
    match resolution.entity.and_then(|entity| data.refs.get(entity)) {
        Some(EntityRef::Callable { callable }) => Some(*callable),
        _ => None,
    }
}
impl<'a> Index<'a> {
    fn new(data: &'a CallableData, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut index = Self {
            universe: Default::default(),
            resolutions: Default::default(),
            mappings: Default::default(),
            traits: Default::default(),
            signatures: Default::default(),
            coverage: Default::default(),
            declarations: Default::default(),
            decorators: Default::default(),
            bodies: Default::default(),
            children: Default::default(),
            references: Default::default(),
            targets: Default::default(),
            generators: Default::default(),
            _charge: StateCharge::new(budget, "effective-callable-index"),
        };
        let charge = &mut index._charge;
        for row in data.resolutions.iter() {
            index.resolutions.insert(charge, row.symbol, row)?;
            if let Some(callable) = callable(data, row) {
                index.universe.insert(charge, (callable, row.context))?;
                index
                    .mappings
                    .update(charge, (callable, row.context), |v| v.push(row))?;
            }
        }
        for row in data
            .coverage
            .iter()
            .filter(|r| r.family == FactFamily::Syntax)
        {
            if let CoverageScope::Artifact { artifact } = need(&data.scopes, row.scope)? {
                index
                    .coverage
                    .update(charge, (*artifact, row.context), |v| v.push(row))?;
            }
        }
        let mut sources: ChargedMap<Id<SourceArtifact>, Vec<Id<AnalysisContext>>> =
            Default::default();
        for (source, ctx) in index.coverage.keys() {
            sources.update(charge, *source, |v| v.push(*ctx))?;
        }
        for row in data.callables.iter() {
            if let CallableEntity::Source { declaration, .. } = row {
                for ctx in sources
                    .get(&need(&data.occurrences, *declaration)?.source)
                    .into_iter()
                    .flatten()
                {
                    index.universe.insert(charge, (row.id(), *ctx))?;
                }
            }
        }
        for row in data.declarations.iter() {
            let ctx = context(data, row.qualification)?;
            index
                .declarations
                .update(charge, (row.declaration, ctx), |v| v.push(row))?;
            if let Some(entity) =
                super::entities::source_callable(need(&data.occurrences, row.declaration)?)
            {
                need(&data.callables, entity.id())?;
                index.universe.insert(charge, (entity.id(), ctx))?;
            }
        }
        for row in data.decorators.iter() {
            index.decorators.update(
                charge,
                (row.declaration, context(data, row.qualification)?),
                |v| v.push(row),
            )?;
        }
        for row in data.bodies.iter() {
            index.bodies.update(
                charge,
                (row.declaration, context(data, row.qualification)?),
                |v| v.push(row),
            )?;
        }
        for row in data.placements.iter() {
            if let Some(parent) = row.parent {
                index.children.update(
                    charge,
                    (parent, context(data, row.qualification)?),
                    |v| v.push(row),
                )?;
            }
        }
        for row in data.traits.iter() {
            if let Some(resolution) = index.resolutions.get(&row.symbol)
                && let Some(callable) = callable(data, resolution)
            {
                let ctx = context(data, row.qualification)?;
                if ctx == resolution.context {
                    index
                        .traits
                        .update(charge, (callable, ctx), |v| v.push(row))?;
                }
            }
        }
        for row in data.signatures.iter().filter(|s| s.role.runtime_source()) {
            if let Some(resolution) = index.resolutions.get(&row.symbol)
                && let Some(callable) = callable(data, resolution)
            {
                let ctx = context(data, row.qualification)?;
                if ctx == resolution.context {
                    index
                        .signatures
                        .update(charge, (callable, ctx), |v| v.push(row))?;
                }
            }
        }
        for row in data.reference_assessments.iter() {
            let reference = need(&data.references, row.reference)?;
            index.references.update(
                charge,
                (reference.read, context(data, reference.qualification)?),
                |v| v.push(row),
            )?;
        }
        for row in data.reference_candidates.iter() {
            index
                .targets
                .update(charge, row.assessment, |v| v.push(row))?;
        }
        for row in data.owners.iter() {
            if matches!(
                need(&data.occurrences, row.occurrence)?.syntax_kind,
                SyntaxKind::ExprYield | SyntaxKind::ExprYieldFrom
            ) {
                index.generators.insert(charge, row.owner)?;
            }
        }
        Ok(index)
    }
}
fn evidence(
    output: &mut CallableOutput,
    assessment: Id<EffectiveCallableAssessment>,
    premise: EffectiveCallablePremise,
) -> Result<(), ModelError> {
    let premise = output.premises.insert(premise)?;
    output.evidence.insert(EffectiveCallableEvidence {
        assessment,
        premise,
    })?;
    Ok(())
}
/// Descriptor recognition only accepts an exact bare builtin reference, with every retained
/// lexical alternative definite and identical. Shadowed names, aliases and attributes refuse.
fn descriptor(
    data: &CallableData,
    index: &Index<'_>,
    decorator: &DeclarationDecorator,
    ctx: Id<AnalysisContext>,
    premises: &mut Vec<EffectiveCallablePremise>,
    charge: &mut StateCharge,
) -> Result<Option<DescriptorKind>, ModelError> {
    let occurrence = need(&data.occurrences, decorator.decorator)?;
    let bare = if occurrence.syntax_kind == SyntaxKind::ExprName {
        Some(occurrence.id())
    } else if occurrence.syntax_kind == SyntaxKind::Decorator {
        let children = index.children.get(&(occurrence.id(), ctx));
        let mut names = children.into_iter().flatten().map(|p| p.occurrence);
        let first = names.next();
        if names.any(|next| Some(next) != first) {
            None
        } else {
            first.filter(|id| {
                data.occurrences
                    .get(*id)
                    .is_some_and(|o| o.syntax_kind == SyntaxKind::ExprName)
            })
        }
    } else {
        None
    };
    let Some(bare) = bare else {
        return Ok(None);
    };
    let Some(references) = index.references.get(&(bare, ctx)) else {
        return Ok(None);
    };
    let mut result = None;
    for assessment in references {
        let premise = EffectiveCallablePremise::Lexical {
            assessment: assessment.id(),
        };
        charge.admit(&premise)?;
        premises.push(premise);
        if assessment.status != ResolutionStatus::Resolved {
            return Ok(None);
        }
        let Some(candidates) = index
            .targets
            .get(&assessment.id())
            .filter(|v| !v.is_empty())
        else {
            return Ok(None);
        };
        for candidate in candidates {
            let raw = need(&data.lexical_resolutions, candidate.resolution)?;
            if !exact(data, raw.qualification)? {
                return Ok(None);
            }
            let ReferenceEntityTarget::Builtin { target } =
                need(&data.reference_targets, candidate.target)?
            else {
                return Ok(None);
            };
            let kind = match need(&data.lexical_targets, *target)? {
                LexicalTarget::Builtin {
                    name,
                    variable: false,
                } => match name.as_str() {
                    "staticmethod" => DescriptorKind::StaticMethod,
                    "classmethod" => DescriptorKind::ClassMethod,
                    "property" => DescriptorKind::Property,
                    _ => return Ok(None),
                },
                _ => return Ok(None),
            };
            if result.is_some_and(|old| old != kind) {
                return Ok(None);
            }
            result = Some(kind);
        }
    }
    Ok(result)
}
fn trait_descriptor(row: &FunctionTraitObservation) -> Option<DescriptorKind> {
    match (
        row.staticmethod,
        row.classmethod,
        row.property_getter,
        row.property_setter,
    ) {
        (true, false, false, false) => Some(DescriptorKind::StaticMethod),
        (false, true, false, false) => Some(DescriptorKind::ClassMethod),
        (false, false, true, false) => Some(DescriptorKind::Property),
        (false, false, false, false) => Some(if row.defining_class.is_some() {
            DescriptorKind::InstanceMethod
        } else {
            DescriptorKind::Function
        }),
        _ => None,
    }
}
fn signature_knowledge(
    data: &CallableData,
    rows: &[&Signature],
    budget: &ResourceBudget,
) -> Result<(Knowledge, CallableReason), ModelError> {
    if rows.is_empty() {
        return Ok((Knowledge::Unknown, CallableReason::MissingSignature));
    }
    for row in rows {
        if !exact(data, row.qualification)? {
            return Ok((Knowledge::Unknown, CallableReason::QualifiedUncertainty));
        }
    }
    let mut charge = StateCharge::new(budget, "callable-signature-contracts");
    let mut providers: ChargedMap<Id<ProviderSymbol>, Vec<&Signature>> = Default::default();
    for row in rows {
        providers.update(&mut charge, row.symbol, |v| v.push(*row))?;
    }
    let mut contracts = ChargedSet::default();
    for rows in providers.values() {
        let mut variants: ChargedMap<i64, (i16, ContentHash)> = Default::default();
        for row in rows {
            let value = (row.form.code(), row.parameters);
            if variants
                .insert(&mut charge, row.variant, value)?
                .is_some_and(|old| old != value)
            {
                return Ok((Knowledge::Conflicting, CallableReason::ConflictingEvidence));
            }
        }
        let mut key = KeySink::new("effective-signature-contract");
        for (ordinal, (form, parameters)) in variants.iter() {
            ordinal.encode(&mut key);
            form.encode(&mut key);
            parameters.encode(&mut key);
        }
        contracts.insert(&mut charge, key.finish().0)?;
    }
    if contracts.len() > 1 {
        Ok((Knowledge::Conflicting, CallableReason::ConflictingEvidence))
    } else if rows.iter().any(|r| r.form != SignatureForm::List) {
        Ok((Knowledge::Unknown, CallableReason::IncompleteSignature))
    } else {
        Ok((Knowledge::Known, CallableReason::EvidenceAgreement))
    }
}
fn derive_assessments(
    data: &CallableData,
    index: &Index<'_>,
    output: &mut CallableOutput,
    assessments: &mut ChargedMap<CallableContext, Id<EffectiveCallableAssessment>>,
    charge: &mut StateCharge,
    selected: Option<CallableContext>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    for &(callable, ctx) in index
        .universe
        .iter()
        .filter(|key| selected.is_none_or(|selected| selected == **key))
    {
        let mut held = StateCharge::new(budget, "effective-callable-premises");
        let mut premises = Vec::new();
        let traits = index
            .traits
            .get(&(callable, ctx))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let signatures = index
            .signatures
            .get(&(callable, ctx))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for row in traits {
            let p = EffectiveCallablePremise::Traits {
                observation: row.id(),
            };
            held.admit(&p)?;
            premises.push(p);
        }
        for row in signatures {
            let p = EffectiveCallablePremise::Signature {
                signature: row.id(),
            };
            held.admit(&p)?;
            premises.push(p);
        }
        for row in index.mappings.get(&(callable, ctx)).into_iter().flatten() {
            let p = EffectiveCallablePremise::Resolution {
                resolution: row.id(),
            };
            held.admit(&p)?;
            premises.push(p);
        }
        let (signature_state, signature_reason) = signature_knowledge(data, signatures, budget)?;
        let mut row = EffectiveCallableAssessment {
            callable,
            context: ctx,
            decorators: ContentHash::of(b""),
            policy: policy_revision(),
            identity: Knowledge::Unknown,
            identity_reason: CallableReason::NoSourceBody,
            signatures: signature_state,
            signature_reason,
            descriptor: Knowledge::Unknown,
            descriptor_kind: None,
            descriptor_reason: CallableReason::NoSourceBody,
            body: Knowledge::Unknown,
            body_admitted: false,
            body_reason: CallableReason::NoSourceBody,
            asynchronous: None,
            generator: None,
        };
        let mut decorators: ChargedMap<i64, Id<Occurrence>> = Default::default();
        let mut members = Vec::new();
        if let CallableEntity::Source { declaration, .. } = need(&data.callables, callable)? {
            let source = need(&data.occurrences, *declaration)?.source;
            let coverage = index
                .coverage
                .get(&(source, ctx))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let syntax_complete = !coverage.is_empty()
                && coverage
                    .iter()
                    .all(|c| c.status == CoverageStatus::CompleteUnderStatedModel);
            for c in coverage {
                let p = EffectiveCallablePremise::Coverage { coverage: c.id() };
                held.admit(&p)?;
                premises.push(p);
            }
            let declarations = index
                .declarations
                .get(&(*declaration, ctx))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut async_values = (false, false);
            let mut async_certain = true;
            for d in declarations {
                let p = EffectiveCallablePremise::Syntax {
                    declaration: d.id(),
                };
                held.admit(&p)?;
                premises.push(p);
                async_certain &= exact(data, d.qualification)?;
                match d.kind {
                    DeclarationKind::AsyncFunction => async_values.0 = true,
                    DeclarationKind::Function => async_values.1 = true,
                    DeclarationKind::Class => {}
                }
            }
            row.asynchronous = if async_certain {
                match async_values {
                    (true, false) => Some(true),
                    (false, true) => Some(false),
                    _ => None,
                }
            } else {
                None
            };
            row.generator = if index.generators.contains(declaration) {
                Some(true)
            } else if syntax_complete {
                Some(false)
            } else {
                None
            };
            let mut conflicting_chain = false;
            for d in index
                .decorators
                .get(&(*declaration, ctx))
                .into_iter()
                .flatten()
            {
                if decorators
                    .insert(&mut held, d.ordinal, d.decorator)?
                    .is_some_and(|old| old != d.decorator)
                {
                    conflicting_chain = true;
                }
                held.admit(d)?;
                members.push(*d);
            }
            if decorators.keys().copied().ne(0..decorators.len() as i64) {
                conflicting_chain = true;
            }
            let mut key = KeySink::new("effective-decorator-chain");
            for (ordinal, occurrence) in decorators.iter() {
                ordinal.encode(&mut key);
                occurrence.encode(&mut key);
            }
            row.decorators = key.finish();
            let recognized = if decorators.len() == 1 && !conflicting_chain {
                descriptor(data, &index, members[0], ctx, &mut premises, &mut held)?
            } else {
                None
            };
            let native = traits.first().and_then(|t| trait_descriptor(t));
            let trait_conflict = traits.iter().any(|t| trait_descriptor(t) != native);
            let mut certain = true;
            for q in traits
                .iter()
                .map(|t| t.qualification)
                .chain(signatures.iter().map(|s| s.qualification))
                .chain(declarations.iter().map(|d| d.qualification))
                .chain(members.iter().map(|d| d.qualification))
            {
                certain &= exact(data, q)?;
            }
            let (state, reason, kind) = if !syntax_complete {
                (Knowledge::Unknown, CallableReason::IncompleteSyntax, None)
            } else if conflicting_chain || trait_conflict {
                (
                    Knowledge::Conflicting,
                    CallableReason::ConflictingEvidence,
                    None,
                )
            } else if !certain {
                (
                    Knowledge::Unknown,
                    CallableReason::QualifiedUncertainty,
                    None,
                )
            } else if decorators.len() > 1 {
                (
                    Knowledge::Unknown,
                    CallableReason::UnsupportedDecorator,
                    None,
                )
            } else if decorators.len() == 1 && recognized.is_none() {
                (
                    Knowledge::Unknown,
                    CallableReason::ShadowedOrUnresolved,
                    None,
                )
            } else if traits.is_empty() {
                (Knowledge::Unknown, CallableReason::MissingTraits, None)
            } else if traits
                .iter()
                .any(|t| t.origin != symbols::FunctionOrigin::DefStatement)
            {
                (
                    Knowledge::Unknown,
                    CallableReason::UnsupportedNativeOrigin,
                    None,
                )
            } else if (decorators.is_empty()
                && !matches!(
                    native,
                    Some(DescriptorKind::Function | DescriptorKind::InstanceMethod)
                ))
                || (recognized.is_some() && recognized != native)
            {
                (
                    Knowledge::Conflicting,
                    CallableReason::ConflictingEvidence,
                    None,
                )
            } else {
                (Knowledge::Known, CallableReason::EvidenceAgreement, native)
            };
            row.descriptor = state;
            row.descriptor_reason = reason;
            row.descriptor_kind = kind;
            row.identity = state;
            row.identity_reason = reason;
            if state == Knowledge::Known && signature_state != Knowledge::Known {
                row.identity = signature_state;
                row.identity_reason = signature_reason;
            }
            let bodies = index
                .bodies
                .get(&(*declaration, ctx))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let excluded = |b: &FunctionBodyObservation| {
                b.abstract_method
                    || b.in_protocol_class
                    || b.in_type_checking_block
                    || b.overload
                    || matches!(b.body, crate::domain::types::FunctionBodyKind::Ellipsis)
            };
            for b in bodies {
                let p = EffectiveCallablePremise::Body {
                    observation: b.id(),
                };
                held.admit(&p)?;
                premises.push(p);
            }
            let native_excluded = traits.iter().any(|t| t.stub || t.overload);
            let native_body_conflict = traits.first().is_some_and(|first| {
                traits
                    .iter()
                    .any(|t| (t.stub, t.overload) != (first.stub, first.overload))
            });
            let mut body_certain = true;
            for b in bodies {
                body_certain &= exact(data, b.qualification)?;
            }
            if row.identity != Knowledge::Known {
                row.body = row.identity;
                row.body_reason = row.identity_reason;
            } else if bodies.is_empty() {
                row.body_reason = CallableReason::MissingBodyEvidence;
            } else if !body_certain {
                row.body_reason = CallableReason::QualifiedUncertainty;
            } else if native_body_conflict
                || bodies.iter().any(|b| excluded(b)) != bodies.iter().all(|b| excluded(b))
            {
                row.body = Knowledge::Conflicting;
                row.body_reason = CallableReason::ConflictingEvidence;
            } else {
                row.body = Knowledge::Known;
                row.body_admitted = !native_excluded && !bodies.iter().any(|b| excluded(b));
                row.body_reason = if row.body_admitted {
                    CallableReason::EvidenceAgreement
                } else {
                    CallableReason::BodyExcluded
                };
            }
        }
        let assessment = output.assessments.insert(row)?;
        assessments.insert(charge, (callable, ctx), assessment)?;
        for member in members {
            output.decorators.insert(EffectiveDecoratorMember {
                assessment,
                observation: member.id(),
                source_ordinal: member.ordinal,
                application_ordinal: decorators.len() as i64 - 1 - member.ordinal,
            })?;
        }
        for premise in premises {
            evidence(output, assessment, premise)?;
        }
    }
    Ok(())
}
/// The owner kernel for one callable and context. Its premises must include every resolution,
/// trait/signature alternative, decorator reference and syntax/body observation for this grain.
/// The compiler selects those candidate families before calling this operation.
pub fn derive_assessment(
    data: &CallableData,
    callable: Id<CallableEntity>,
    context: Id<AnalysisContext>,
    budget: &ResourceBudget,
) -> Result<CallableOutput, ModelError> {
    let index = Index::new(data, budget)?;
    if !index.universe.contains(&(callable, context)) {
        return Err(invalid(
            "callable assessment grain has no admitted source universe",
        ));
    }
    let mut output = CallableOutput::new(budget);
    let mut assessments = Default::default();
    let mut charge = StateCharge::new(budget, "callable-assessment-grain");
    derive_assessments(
        data,
        &index,
        &mut output,
        &mut assessments,
        &mut charge,
        Some((callable, context)),
        budget,
    )?;
    Ok(output)
}
/// Check the actual source-owned descriptor/body/signature claims with the same local kernel
/// that constructs them, without replaying any upstream entity, link or callable producer.
pub fn admit_assessment(
    data: &CallableData,
    stored: &CallableOutput,
    callable: Id<CallableEntity>,
    context: Id<AnalysisContext>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let expected = derive_assessment(data, callable, context, budget)?;
    stored
        .assessments
        .same(&expected.assessments)
        .then_some(())
        .ok_or_else(|| invalid("callable owner claims differ from selected source premises"))?;
    if !stored.decorators.same(&expected.decorators)
        || !stored.premises.same(&expected.premises)
        || !stored.evidence.same(&expected.evidence)
    {
        return Err(invalid(
            "callable owner evidence differs from selected source premises",
        ));
    }
    Ok(())
}
fn derive_variants(
    data: &CallableData,
    index: &Index<'_>,
    output: &mut CallableOutput,
    assessments: &ChargedMap<CallableContext, Id<EffectiveCallableAssessment>>,
    selected: Option<&ChargedSet<Id<Signature>>>,
) -> Result<(), ModelError> {
    for signature in data
        .signatures
        .iter()
        .filter(|row| selected.is_none_or(|selected| selected.contains(&row.id())))
    {
        let resolution = index
            .resolutions
            .get(&signature.symbol)
            .ok_or_else(|| invalid("signature has no total symbol resolution"))?;
        let ctx = context(data, signature.qualification)?;
        let target = if ctx == resolution.context {
            callable(data, resolution)
        } else {
            None
        };
        let assessment = target.and_then(|id| assessments.get(&(id, ctx)).copied());
        let kind = assessment
            .and_then(|a| output.assessments.get(a))
            .and_then(|a| a.descriptor_kind);
        let adjustment = match kind {
            Some(DescriptorKind::Function | DescriptorKind::StaticMethod) => {
                SignatureAdjustment::None
            }
            Some(DescriptorKind::InstanceMethod) => SignatureAdjustment::BindInstanceReceiver,
            Some(DescriptorKind::ClassMethod) => SignatureAdjustment::BindClassReceiver,
            Some(DescriptorKind::Property) => SignatureAdjustment::PropertyAccess,
            None => SignatureAdjustment::Unknown,
        };
        let native = data
            .native_signatures
            .iter()
            .find(|n| n.signature == signature.id() && n.qualification == signature.qualification);
        let adjustment = if let Some(native) = native {
            match native.receiver {
                NativeReceiver::Unbound => SignatureAdjustment::None,
                NativeReceiver::Instance => SignatureAdjustment::BindInstanceReceiver,
                NativeReceiver::Class => SignatureAdjustment::BindClassReceiver,
                NativeReceiver::Property => SignatureAdjustment::PropertyAccess,
                NativeReceiver::Unknown => SignatureAdjustment::Unknown,
            }
        } else {
            adjustment
        };
        output.variants.insert(SignatureVariant {
            signature: signature.id(),
            role: signature.role,
            native: native.map(Record::id),
            context: ctx,
            resolution: resolution.id(),
            callable: target,
            assessment,
            adjustment,
        })?;
    }
    Ok(())
}
fn derive_slots(
    data: &CallableData,
    output: &mut CallableOutput,
    selected: Option<&ChargedSet<Id<Signature>>>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "callable-signature-slots");
    let mut variants: ChargedMap<Id<Signature>, Id<SignatureVariant>> = Default::default();
    for row in output.variants.iter() {
        variants.insert(&mut charge, row.signature, row.id())?;
    }
    let mut slots: ChargedMap<Id<SignatureParameter>, Id<SignatureSlot>> = Default::default();
    for parameter in data
        .parameters
        .iter()
        .filter(|row| selected.is_none_or(|selected| selected.contains(&row.signature)))
    {
        let shape = need(&data.shapes, parameter.shape)?;
        let signature = need(&data.signatures, parameter.signature)?;
        let default = if matches!(
            shape.kind,
            ParameterKind::VarPositional | ParameterKind::VarKeyword
        ) {
            DefaultSlot::Collector
        } else if shape.required {
            DefaultSlot::Required
        } else if signature.role.runtime_source() {
            DefaultSlot::DefinitionTime
        } else {
            DefaultSlot::NativeUnknown
        };
        let variant = *variants
            .get(&parameter.signature)
            .ok_or_else(|| invalid("parameter has no total signature variant"))?;
        let slot = output.slots.insert(SignatureSlot {
            parameter: parameter.id(),
            variant,
            ordinal: parameter.ordinal,
            default,
        })?;
        slots.insert(&mut charge, parameter.id(), slot)?;
    }
    for link in data
        .parameter_links
        .iter()
        .filter(|row| selected.is_none() || slots.contains_key(&row.parameter))
    {
        output.slot_entities.insert(SignatureSlotEntity {
            slot: *slots
                .get(&link.parameter)
                .ok_or_else(|| invalid("parameter link has no signature slot"))?,
            link: link.id(),
        })?;
    }
    for observation in data.signature_types.iter() {
        match need(&data.signature_type_subjects, observation.subject)? {
            SignatureTypeSubject::Parameter { parameter } => {
                if selected.is_some() && !slots.contains_key(parameter) {
                    continue;
                }
                output.slot_types.insert(SignatureSlotType {
                    slot: *slots
                        .get(parameter)
                        .ok_or_else(|| invalid("typed parameter slot absent"))?,
                    observation: observation.id(),
                })?;
            }
            SignatureTypeSubject::Return { signature } => {
                if selected.is_some() && !variants.contains_key(signature) {
                    continue;
                }
                output.return_types.insert(SignatureReturnType {
                    variant: *variants
                        .get(signature)
                        .ok_or_else(|| invalid("typed return variant absent"))?,
                    observation: observation.id(),
                })?;
            }
        }
    }
    Ok(())
}
/// Complete alternatives for one source callable, across its admitted context frames. This
/// constructs only the assessment/evidence family; unrelated signatures and slots are outputs
/// of their own selected kernel.
pub fn normalize_callable(
    data: &CallableData,
    selected: Id<CallableEntity>,
    budget: &ResourceBudget,
) -> Result<CallableOutput, ModelError> {
    let index = Index::new(data, budget)?;
    let mut output = CallableOutput::new(budget);
    let mut assessments = Default::default();
    let mut charge = StateCharge::new(budget, "callable-family-assessments");
    for key in index
        .universe
        .iter()
        .filter(|(callable, _)| *callable == selected)
    {
        derive_assessments(
            data,
            &index,
            &mut output,
            &mut assessments,
            &mut charge,
            Some(*key),
            budget,
        )?;
    }
    Ok(output)
}
/// Compare only one actual callable's owner claims across its complete context candidate domain.
/// The stored family must be selected by actual callable membership, including missing/extra rows.
pub fn admit_callable(
    data: &CallableData,
    stored: &CallableOutput,
    selected: Id<CallableEntity>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let expected = normalize_callable(data, selected, budget)?;
    if !stored.assessments.same(&expected.assessments)
        || !stored.decorators.same(&expected.decorators)
        || !stored.premises.same(&expected.premises)
        || !stored.evidence.same(&expected.evidence)
    {
        return Err(invalid(
            "selected callable owner claims differ from actual source premises",
        ));
    }
    Ok(())
}
fn signature_assessment(
    data: &CallableData,
    index: &Index<'_>,
    signature: &Signature,
    output: &mut CallableOutput,
    assessments: &mut ChargedMap<CallableContext, Id<EffectiveCallableAssessment>>,
    charge: &mut StateCharge,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let resolution = index
        .resolutions
        .get(&signature.symbol)
        .ok_or_else(|| invalid("signature has no total symbol resolution"))?;
    let ctx = context(data, signature.qualification)?;
    if ctx == resolution.context
        && let Some(callable) = callable(data, resolution)
        && !assessments.contains_key(&(callable, ctx))
    {
        derive_assessments(
            data,
            index,
            output,
            assessments,
            charge,
            Some((callable, ctx)),
            budget,
        )?;
    }
    Ok(())
}
/// A signature's variant, defaults, native slots and attributed types. Its target callable
/// assessment is derived from complete alternatives solely to obtain the canonical reference
/// and descriptor adjustment; that assessment is published by the callable kernel.
pub fn normalize_signature(
    data: &CallableData,
    selected: Id<Signature>,
    budget: &ResourceBudget,
) -> Result<CallableOutput, ModelError> {
    let index = Index::new(data, budget)?;
    let signature = need(&data.signatures, selected)?;
    let mut output = CallableOutput::new(budget);
    let mut assessments = Default::default();
    let mut charge = StateCharge::new(budget, "callable-signature-grain");
    signature_assessment(
        data,
        &index,
        signature,
        &mut output,
        &mut assessments,
        &mut charge,
        budget,
    )?;
    let mut roots = ChargedSet::default();
    roots.insert(&mut charge, selected)?;
    derive_variants(data, &index, &mut output, &assessments, Some(&roots))?;
    derive_slots(data, &mut output, Some(&roots), budget)?;
    output.assessments = Rows::new(budget);
    output.decorators = Rows::new(budget);
    output.premises = Rows::new(budget);
    output.evidence = Rows::new(budget);
    Ok(output)
}
/// One complete native overload trace and every original origin/context/run/surface alternative.
/// Scope selection must not supply another trace; candidate count/digest verification remains
/// the model owner's actual association kernel.
pub fn normalize_overload(
    data: &CallableData,
    selected: Id<types::NativeOverloadObservation>,
    budget: &ResourceBudget,
) -> Result<CallableOutput, ModelError> {
    if data.overload_traces.len() != 1 || data.overload_traces.get(selected).is_none() {
        return Err(invalid(
            "overload kernel needs exactly its selected complete trace",
        ));
    }
    let index = Index::new(data, budget)?;
    let mut output = CallableOutput::new(budget);
    let mut assessments = Default::default();
    let mut charge = StateCharge::new(budget, "callable-overload-grain");
    let mut roots = ChargedSet::default();
    for signature in data
        .signatures
        .iter()
        .filter(|row| row.role == SignatureRole::EffectiveTyped)
    {
        roots.insert(&mut charge, signature.id())?;
        signature_assessment(
            data,
            &index,
            signature,
            &mut output,
            &mut assessments,
            &mut charge,
            budget,
        )?;
    }
    derive_variants(data, &index, &mut output, &assessments, Some(&roots))?;
    super::overload_association::associate(data, &mut output, budget)?;
    output.assessments = Rows::new(budget);
    output.decorators = Rows::new(budget);
    output.premises = Rows::new(budget);
    output.evidence = Rows::new(budget);
    output.variants = Rows::new(budget);
    Ok(output)
}
pub fn normalize(
    data: &CallableData,
    budget: &ResourceBudget,
) -> Result<CallableOutput, ModelError> {
    let index = Index::new(data, budget)?;
    let mut output = CallableOutput::new(budget);
    let mut assessments = Default::default();
    let mut charge = StateCharge::new(budget, "callable-assessment-map");
    derive_assessments(
        data,
        &index,
        &mut output,
        &mut assessments,
        &mut charge,
        None,
        budget,
    )?;
    derive_variants(data, &index, &mut output, &assessments, None)?;
    derive_slots(data, &mut output, None, budget)?;
    super::overload_association::associate(data, &mut output, budget)?;
    Ok(output)
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = CallableData::validation_inputs();
    inputs.extend(CallableOutput::validation_inputs());
    let mut owner_inputs = CallableData::validation_inputs();
    owner_inputs.extend([
        ValidationInput::of::<EffectiveCallableAssessment>(&["id"]),
        ValidationInput::of::<EffectiveDecoratorMember>(&["id"]),
        ValidationInput::of::<EffectiveCallablePremise>(&["id"]),
        ValidationInput::of::<EffectiveCallableEvidence>(&["id"]),
    ]);
    vec![
        Invariant {
            purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
            revision: 1,
            name: "normalized_callable_closure",
            inputs,
            create: std::sync::Arc::new(|budget| {
                Box::new(CallableCheck {
                    data: CallableData::new(budget),
                    output: CallableOutput::new(budget),
                    budget: budget.clone(),
                    admission: false,
                })
            }),
        },
        Invariant {
            purpose: InvariantPurpose::Admission,
            revision: 1,
            name: "normalized_callable_admission",
            inputs: owner_inputs,
            create: std::sync::Arc::new(|budget| {
                Box::new(CallableCheck {
                    data: CallableData::new(budget),
                    output: CallableOutput::new(budget),
                    budget: budget.clone(),
                    admission: true,
                })
            }),
        },
    ]
}
struct CallableCheck {
    admission: bool,
    data: CallableData,
    output: CallableOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for CallableCheck {
    fn normalization_scope(&self) -> Option<super::admission::Scope> {
        self.admission.then_some(super::admission::Scope::Callables)
    }
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if !self.data.visit(relation, batch)? && !self.output.visit(relation, batch)? {
            return Err(invalid("undeclared callable validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.admission {
            let index = Index::new(&self.data, &self.budget)?;
            let mut expected = CallableOutput::new(&self.budget);
            let mut assessments = Default::default();
            let mut charge = StateCharge::new(&self.budget, "callable-owner-admission");
            derive_assessments(
                &self.data,
                &index,
                &mut expected,
                &mut assessments,
                &mut charge,
                None,
                &self.budget,
            )?;
            if !self.output.assessments.same(&expected.assessments)
                || !self.output.decorators.same(&expected.decorators)
                || !self.output.premises.same(&expected.premises)
                || !self.output.evidence.same(&expected.evidence)
            {
                return Err(invalid(
                    "callable owner admission differs from actual source premises",
                ));
            }
            Ok(())
        } else {
            self.output.matches(&normalize(&self.data, &self.budget)?)
        }
    }
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    let mut inputs = super::relation_normalization::stage(profile).inputs;
    macro_rules! prior { ($($field:ident: $ty:ty,)*) => { $(inputs.push(stages::RelationUse::completed::<$ty>());)* }; }
    crate::normalized_relation_outputs!(prior);
    inputs.extend(CallableData::stage_inputs());
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    stages::Stage {
        name: "normalize_callables",
        inputs: super::facts_stage_inputs(inputs),
        outputs: super::callables::relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: policy_revision(),
        configuration: ContentHash::of(b"callables/v1"),
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec![
        "normalized_callable_closure",
        "normalized_callable_admission",
    ]
}

#[cfg(test)]
mod callable_grain_controls {
    use super::*;
    fn id<R: Record>(byte: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([byte; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn selected_claims_require_actual_local_premises_and_omit_other_callable() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut data = CallableData::new(&budget);
        let context = id(1);
        let first = CallableEntity::External { symbol: id(2) };
        let other = CallableEntity::External { symbol: id(3) };
        for row in [&first, &other] {
            data.callables.insert(row.clone()).unwrap();
            let entity = data
                .refs
                .insert(EntityRef::Callable { callable: row.id() })
                .unwrap();
            let CallableEntity::External { symbol } = row else {
                unreachable!()
            };
            data.resolutions
                .insert(SymbolEntityResolution {
                    symbol: *symbol,
                    context,
                    policy: policy_revision(),
                    status: ResolutionStatus::Resolved,
                    entity: Some(entity),
                    reason: EntityReason::ProviderExternal,
                })
                .unwrap();
        }
        let actual = derive_assessment(&data, first.id(), context, &budget).unwrap();
        assert_eq!(actual.assessments.len(), 1);
        assert_eq!(
            actual.assessments.iter().next().unwrap().callable,
            first.id()
        );
        admit_assessment(&data, &actual, first.id(), context, &budget).unwrap();
        let mut false_claim = CallableOutput::new(&budget);
        let mut row = actual.assessments.iter().next().unwrap().clone();
        row.body = Knowledge::Known;
        row.body_reason = CallableReason::EvidenceAgreement;
        false_claim.assessments.insert(row).unwrap();
        assert!(admit_assessment(&data, &false_claim, first.id(), context, &budget).is_err());
        assert!(derive_assessment(&data, first.id(), id(4), &budget).is_err());
    }
}
