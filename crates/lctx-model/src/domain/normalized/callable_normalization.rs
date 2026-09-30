//! N3's single semantic operation, used both by materialization and publication validation.
use super::{Rows, callables::*, entities::*, links::*, policy_revision};
use crate::domain::{*, assertion::AssertionQualification, attribution::*, calls::*, source::*, syntax::*,
    symbols::FunctionTraitObservation, types::FunctionBodyObservation, lexical::*,
    charged::{ChargedMap, ChargedSet, StateCharge}, resources::ResourceBudget};
macro_rules! inputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct CallableData { $(pub $field: Rows<$ty>,)* }
        impl CallableData {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::stored::<$ty>()),*] }
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
fn invalid(message: impl Into<String>) -> ModelError { ModelError::Invalid(message.into()) }
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> { rows.get(id).ok_or_else(|| invalid(format!("normalized callable requires {}", R::NAME))) }
fn context(data: &CallableData, qualification: Id<AssertionQualification>) -> Result<Id<AnalysisContext>, ModelError> { Ok(need(&data.qualifications, qualification)?.context) }
fn exact(data: &CallableData, qualification: Id<AssertionQualification>) -> Result<bool, ModelError> {
    let q = need(&data.qualifications, qualification)?;
    Ok(q.modality == Modality::Definite && q.approximation == assertion::Approximation::Exact)
}
type CallableContext = (Id<CallableEntity>, Id<AnalysisContext>);
struct Index<'a> {
    universe: ChargedSet<CallableContext>,
    resolutions: ChargedMap<Id<ProviderSymbol>, &'a SymbolEntityResolution>,
    mappings: ChargedMap<CallableContext, Vec<&'a SymbolEntityResolution>>,
    traits: ChargedMap<CallableContext, Vec<&'a FunctionTraitObservation>>,
    signatures: ChargedMap<CallableContext, Vec<&'a Signature>>,
    coverage: ChargedMap<(Id<SourceArtifact>, Id<AnalysisContext>), Vec<&'a ProviderCoverage>>,
    declarations: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a DeclarationObservation>>,
    decorators: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a DeclarationDecorator>>,
    bodies: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a FunctionBodyObservation>>,
    children: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a SyntaxPlacement>>,
    references: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a ReferenceEntityAssessment>>,
    targets: ChargedMap<Id<ReferenceEntityAssessment>, Vec<&'a ReferenceEntityCandidate>>,
    generators: ChargedSet<Id<Occurrence>>,
    _charge: StateCharge,
}
fn callable(data: &CallableData, resolution: &SymbolEntityResolution) -> Option<Id<CallableEntity>> {
    if resolution.status != ResolutionStatus::Resolved { return None; }
    match resolution.entity.and_then(|entity| data.refs.get(entity)) { Some(EntityRef::Callable { callable }) => Some(*callable), _ => None }
}
impl<'a> Index<'a> {
    fn new(data: &'a CallableData, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut index = Self { universe: Default::default(), resolutions: Default::default(), mappings: Default::default(), traits: Default::default(), signatures: Default::default(), coverage: Default::default(), declarations: Default::default(), decorators: Default::default(), bodies: Default::default(), children: Default::default(), references: Default::default(), targets: Default::default(), generators: Default::default(), _charge: StateCharge::new(budget, "effective-callable-index") };
        let charge = &mut index._charge;
        for row in data.resolutions.iter() {
            index.resolutions.insert(charge, row.symbol, row)?;
            if let Some(callable) = callable(data, row) {
                index.universe.insert(charge, (callable, row.context))?;
                index.mappings.update(charge, (callable, row.context), |v| v.push(row))?;
            }
        }
        for row in data.coverage.iter().filter(|r| r.family == FactFamily::Syntax) {
            if let CoverageScope::Artifact { artifact } = need(&data.scopes, row.scope)? {
                index.coverage.update(charge, (*artifact, row.context), |v| v.push(row))?;
            }
        }
        let mut sources: ChargedMap<Id<SourceArtifact>, Vec<Id<AnalysisContext>>> = Default::default();
        for (source, ctx) in index.coverage.keys() { sources.update(charge, *source, |v| v.push(*ctx))?; }
        for row in data.callables.iter() {
            if let CallableEntity::Source { declaration, .. } = row {
                for ctx in sources.get(&need(&data.occurrences, *declaration)?.source).into_iter().flatten() { index.universe.insert(charge, (row.id(), *ctx))?; }
            }
        }
        for row in data.declarations.iter() {
            let ctx = context(data, row.qualification)?;
            index.declarations.update(charge, (row.declaration, ctx), |v| v.push(row))?;
            if let Some(entity) = super::entities::source_callable(need(&data.occurrences, row.declaration)?) {
                need(&data.callables, entity.id())?; index.universe.insert(charge, (entity.id(), ctx))?;
            }
        }
        for row in data.decorators.iter() { index.decorators.update(charge, (row.declaration, context(data, row.qualification)?), |v| v.push(row))?; }
        for row in data.bodies.iter() { index.bodies.update(charge, (row.declaration, context(data, row.qualification)?), |v| v.push(row))?; }
        for row in data.placements.iter() { if let Some(parent) = row.parent { index.children.update(charge, (parent, context(data, row.qualification)?), |v| v.push(row))?; } }
        for row in data.traits.iter() {
            if let Some(resolution) = index.resolutions.get(&row.symbol) && let Some(callable) = callable(data, resolution) {
                let ctx = context(data, row.qualification)?;
                if ctx == resolution.context { index.traits.update(charge, (callable, ctx), |v| v.push(row))?; }
            }
        }
        for row in data.signatures.iter() {
            if let Some(resolution) = index.resolutions.get(&row.symbol) && let Some(callable) = callable(data, resolution) {
                let ctx = context(data, row.qualification)?;
                if ctx == resolution.context { index.signatures.update(charge, (callable, ctx), |v| v.push(row))?; }
            }
        }
        for row in data.reference_assessments.iter() {
            let reference = need(&data.references, row.reference)?;
            index.references.update(charge, (reference.read, context(data, reference.qualification)?), |v| v.push(row))?;
        }
        for row in data.reference_candidates.iter() { index.targets.update(charge, row.assessment, |v| v.push(row))?; }
        for row in data.owners.iter() {
            if matches!(need(&data.occurrences, row.occurrence)?.syntax_kind, SyntaxKind::ExprYield | SyntaxKind::ExprYieldFrom) { index.generators.insert(charge, row.owner)?; }
        }
        Ok(index)
    }
}
fn evidence(output: &mut CallableOutput, assessment: Id<EffectiveCallableAssessment>, premise: EffectiveCallablePremise) -> Result<(), ModelError> {
    let premise = output.premises.insert(premise)?;
    output.evidence.insert(EffectiveCallableEvidence { assessment, premise })?; Ok(())
}
/// Descriptor recognition only accepts an exact bare builtin reference, with every retained
/// lexical alternative definite and identical. Shadowed names, aliases and attributes refuse.
fn descriptor(data: &CallableData, index: &Index<'_>, decorator: &DeclarationDecorator, ctx: Id<AnalysisContext>, premises: &mut Vec<EffectiveCallablePremise>, charge: &mut StateCharge) -> Result<Option<DescriptorKind>, ModelError> {
    let occurrence = need(&data.occurrences, decorator.decorator)?;
    let bare = if occurrence.syntax_kind == SyntaxKind::ExprName { Some(occurrence.id()) }
        else if occurrence.syntax_kind == SyntaxKind::Decorator {
            let children = index.children.get(&(occurrence.id(), ctx));
            let mut names = children.into_iter().flatten().map(|p| p.occurrence);
            let first = names.next();
            if names.any(|next| Some(next) != first) { None } else { first.filter(|id| data.occurrences.get(*id).is_some_and(|o| o.syntax_kind == SyntaxKind::ExprName)) }
        } else { None };
    let Some(bare) = bare else { return Ok(None); };
    let Some(references) = index.references.get(&(bare, ctx)) else { return Ok(None); };
    let mut result = None;
    for assessment in references {
        let premise = EffectiveCallablePremise::Lexical { assessment: assessment.id() }; charge.admit(&premise)?; premises.push(premise);
        if assessment.status != ResolutionStatus::Resolved { return Ok(None); }
        let Some(candidates) = index.targets.get(&assessment.id()).filter(|v| !v.is_empty()) else { return Ok(None); };
        for candidate in candidates {
            let raw = need(&data.lexical_resolutions, candidate.resolution)?;
            if !exact(data, raw.qualification)? { return Ok(None); }
            let ReferenceEntityTarget::Builtin { target } = need(&data.reference_targets, candidate.target)? else { return Ok(None); };
            let kind = match need(&data.lexical_targets, *target)? {
                LexicalTarget::Builtin { name, variable: false } => match name.as_str() {
                    "staticmethod" => DescriptorKind::StaticMethod, "classmethod" => DescriptorKind::ClassMethod, "property" => DescriptorKind::Property, _ => return Ok(None),
                }, _ => return Ok(None),
            };
            if result.is_some_and(|old| old != kind) { return Ok(None); } result = Some(kind);
        }
    }
    Ok(result)
}
fn trait_descriptor(row: &FunctionTraitObservation) -> Option<DescriptorKind> {
    match (row.staticmethod, row.classmethod, row.property_getter, row.property_setter) {
        (true, false, false, false) => Some(DescriptorKind::StaticMethod),
        (false, true, false, false) => Some(DescriptorKind::ClassMethod),
        (false, false, true, false) => Some(DescriptorKind::Property),
        (false, false, false, false) => Some(if row.defining_class.is_some() { DescriptorKind::InstanceMethod } else { DescriptorKind::Function }),
        _ => None,
    }
}
fn signature_knowledge(rows: &[&Signature], budget: &ResourceBudget) -> Result<(Knowledge, CallableReason), ModelError> {
    if rows.is_empty() { return Ok((Knowledge::Unknown, CallableReason::MissingSignature)); }
    let mut charge = StateCharge::new(budget, "callable-signature-contracts");
    let mut providers: ChargedMap<Id<ProviderSymbol>, Vec<&Signature>> = Default::default();
    for row in rows { providers.update(&mut charge, row.symbol, |v| v.push(*row))?; }
    let mut contracts = ChargedSet::default();
    for rows in providers.values() {
        let mut variants: ChargedMap<i64, (i16, ContentHash)> = Default::default();
        for row in rows {
            let value = (row.form.code(), row.parameters);
            if variants.insert(&mut charge, row.variant, value)?.is_some_and(|old| old != value) { return Ok((Knowledge::Conflicting, CallableReason::ConflictingEvidence)); }
        }
        let mut key = KeySink::new("effective-signature-contract");
        for (ordinal, (form, parameters)) in variants.iter() { ordinal.encode(&mut key); form.encode(&mut key); parameters.encode(&mut key); }
        contracts.insert(&mut charge, key.finish().0)?;
    }
    if contracts.len() > 1 { Ok((Knowledge::Conflicting, CallableReason::ConflictingEvidence)) }
    else if rows.iter().any(|r| r.form != SignatureForm::List) { Ok((Knowledge::Unknown, CallableReason::IncompleteSignature)) }
    else { Ok((Knowledge::Known, CallableReason::EvidenceAgreement)) }
}
pub fn normalize(data: &CallableData, budget: &ResourceBudget) -> Result<CallableOutput, ModelError> {
    let index = Index::new(data, budget)?;
    let mut output = CallableOutput::new(budget);
    let mut assessments: ChargedMap<CallableContext, Id<EffectiveCallableAssessment>> = Default::default();
    let mut charge = StateCharge::new(budget, "callable-assessment-map");
    for &(callable, ctx) in index.universe.iter() {
        let mut held = StateCharge::new(budget, "effective-callable-premises");
        let mut premises = Vec::new();
        let traits = index.traits.get(&(callable, ctx)).map(Vec::as_slice).unwrap_or(&[]);
        let signatures = index.signatures.get(&(callable, ctx)).map(Vec::as_slice).unwrap_or(&[]);
        for row in traits { let p = EffectiveCallablePremise::Traits { observation: row.id() }; held.admit(&p)?; premises.push(p); }
        for row in signatures { let p = EffectiveCallablePremise::Signature { signature: row.id() }; held.admit(&p)?; premises.push(p); }
        for row in index.mappings.get(&(callable, ctx)).into_iter().flatten() { let p = EffectiveCallablePremise::Resolution { resolution: row.id() }; held.admit(&p)?; premises.push(p); }
        let (signature_state, signature_reason) = signature_knowledge(signatures, budget)?;
        let mut row = EffectiveCallableAssessment { callable, context: ctx, decorators: ContentHash::of(b""), policy: policy_revision(),
            identity: Knowledge::Unknown, identity_reason: CallableReason::NoSourceBody, signatures: signature_state, signature_reason,
            descriptor: Knowledge::Unknown, descriptor_kind: None, descriptor_reason: CallableReason::NoSourceBody,
            body: Knowledge::Unknown, body_admitted: false, body_reason: CallableReason::NoSourceBody, asynchronous: None, generator: None };
        let mut decorators: ChargedMap<i64, Id<Occurrence>> = Default::default();
        let mut members = Vec::new();
        if let CallableEntity::Source { declaration, .. } = need(&data.callables, callable)? {
            let source = need(&data.occurrences, *declaration)?.source;
            let coverage = index.coverage.get(&(source, ctx)).map(Vec::as_slice).unwrap_or(&[]);
            let syntax_complete = !coverage.is_empty() && coverage.iter().all(|c| c.status == CoverageStatus::CompleteUnderStatedModel);
            for c in coverage { let p = EffectiveCallablePremise::Coverage { coverage: c.id() }; held.admit(&p)?; premises.push(p); }
            let declarations = index.declarations.get(&(*declaration, ctx)).map(Vec::as_slice).unwrap_or(&[]);
            let mut async_values = (false, false);
            for d in declarations {
                let p = EffectiveCallablePremise::Syntax { declaration: d.id() }; held.admit(&p)?; premises.push(p);
                match d.kind { DeclarationKind::AsyncFunction => async_values.0 = true, DeclarationKind::Function => async_values.1 = true, DeclarationKind::Class => {} }
            }
            row.asynchronous = match async_values { (true, false) => Some(true), (false, true) => Some(false), _ => None };
            row.generator = if index.generators.contains(declaration) { Some(true) } else if syntax_complete { Some(false) } else { None };
            let mut conflicting_chain = false;
            for d in index.decorators.get(&(*declaration, ctx)).into_iter().flatten() {
                if decorators.insert(&mut held, d.ordinal, d.decorator)?.is_some_and(|old| old != d.decorator) { conflicting_chain = true; }
                held.admit(d)?; members.push(*d);
            }
            if decorators.keys().copied().ne(0..decorators.len() as i64) { conflicting_chain = true; }
            let mut key = KeySink::new("effective-decorator-chain");
            for (ordinal, occurrence) in decorators.iter() { ordinal.encode(&mut key); occurrence.encode(&mut key); }
            row.decorators = key.finish();
            let recognized = if decorators.len() == 1 && !conflicting_chain { descriptor(data, &index, members[0], ctx, &mut premises, &mut held)? } else { None };
            let native = traits.first().and_then(|t| trait_descriptor(t));
            let trait_conflict = traits.iter().any(|t| trait_descriptor(t) != native);
            let mut certain = true;
            for q in traits.iter().map(|t| t.qualification).chain(signatures.iter().map(|s| s.qualification)).chain(declarations.iter().map(|d| d.qualification)).chain(members.iter().map(|d| d.qualification)) { certain &= exact(data, q)?; }
            let (state, reason, kind) = if !syntax_complete { (Knowledge::Unknown, CallableReason::IncompleteSyntax, None) }
                else if conflicting_chain || trait_conflict { (Knowledge::Conflicting, CallableReason::ConflictingEvidence, None) }
                else if !certain { (Knowledge::Unknown, CallableReason::QualifiedUncertainty, None) }
                else if decorators.len() > 1 { (Knowledge::Unknown, CallableReason::UnsupportedDecorator, None) }
                else if decorators.len() == 1 && recognized.is_none() { (Knowledge::Unknown, CallableReason::ShadowedOrUnresolved, None) }
                else if traits.is_empty() { (Knowledge::Unknown, CallableReason::MissingTraits, None) }
                else if traits.iter().any(|t| t.origin != symbols::FunctionOrigin::DefStatement) { (Knowledge::Unknown, CallableReason::UnsupportedNativeOrigin, None) }
                else if (decorators.is_empty() && !matches!(native, Some(DescriptorKind::Function | DescriptorKind::InstanceMethod))) || (recognized.is_some() && recognized != native) {
                    (Knowledge::Conflicting, CallableReason::ConflictingEvidence, None)
                } else { (Knowledge::Known, CallableReason::EvidenceAgreement, native) };
            row.descriptor = state; row.descriptor_reason = reason; row.descriptor_kind = kind;
            row.identity = state; row.identity_reason = reason;
            if state == Knowledge::Known && signature_state != Knowledge::Known { row.identity = signature_state; row.identity_reason = signature_reason; }
            let bodies = index.bodies.get(&(*declaration, ctx)).map(Vec::as_slice).unwrap_or(&[]);
            let excluded = |b: &FunctionBodyObservation| b.abstract_method || b.in_protocol_class || b.in_type_checking_block || b.overload || matches!(b.body, crate::domain::types::FunctionBodyKind::Ellipsis);
            for b in bodies { let p = EffectiveCallablePremise::Body { observation: b.id() }; held.admit(&p)?; premises.push(p); }
            let native_excluded = traits.iter().any(|t| t.stub || t.overload);
            let native_body_conflict = traits.first().is_some_and(|first| traits.iter().any(|t| (t.stub, t.overload) != (first.stub, first.overload)));
            let mut body_certain = true; for b in bodies { body_certain &= exact(data, b.qualification)?; }
            if row.identity != Knowledge::Known { row.body = row.identity; row.body_reason = row.identity_reason; }
            else if bodies.is_empty() { row.body_reason = CallableReason::MissingBodyEvidence; }
            else if !body_certain { row.body_reason = CallableReason::QualifiedUncertainty; }
            else if native_body_conflict || bodies.iter().any(|b| excluded(b)) != bodies.iter().all(|b| excluded(b)) { row.body = Knowledge::Conflicting; row.body_reason = CallableReason::ConflictingEvidence; }
            else { row.body = Knowledge::Known; row.body_admitted = !native_excluded && !bodies.iter().any(|b| excluded(b)); row.body_reason = if row.body_admitted { CallableReason::EvidenceAgreement } else { CallableReason::BodyExcluded }; }
        }
        let assessment = output.assessments.insert(row)?;
        assessments.insert(&mut charge, (callable, ctx), assessment)?;
        for member in members { output.decorators.insert(EffectiveDecoratorMember { assessment, observation: member.id(), source_ordinal: member.ordinal, application_ordinal: decorators.len() as i64 - 1 - member.ordinal })?; }
        for premise in premises { evidence(&mut output, assessment, premise)?; }
    }
    for signature in data.signatures.iter() {
        let resolution = index.resolutions.get(&signature.symbol).ok_or_else(|| invalid("signature has no total symbol resolution"))?;
        let ctx = context(data, signature.qualification)?;
        let target = if ctx == resolution.context { callable(data, resolution) } else { None };
        let assessment = target.and_then(|id| assessments.get(&(id, ctx)).copied());
        let kind = assessment.and_then(|a| output.assessments.get(a)).and_then(|a| a.descriptor_kind);
        let adjustment = match kind { Some(DescriptorKind::Function | DescriptorKind::StaticMethod) => SignatureAdjustment::None,
            Some(DescriptorKind::InstanceMethod) => SignatureAdjustment::BindInstanceReceiver, Some(DescriptorKind::ClassMethod) => SignatureAdjustment::BindClassReceiver,
            Some(DescriptorKind::Property) => SignatureAdjustment::PropertyAccess, None => SignatureAdjustment::Unknown };
        output.variants.insert(SignatureVariant { signature: signature.id(), context: ctx, resolution: resolution.id(), callable: target, assessment, adjustment })?;
    }
    let mut variants: ChargedMap<Id<Signature>, Id<SignatureVariant>> = Default::default();
    for row in output.variants.iter() { variants.insert(&mut charge, row.signature, row.id())?; }
    let mut slots: ChargedMap<Id<SignatureParameter>, Id<SignatureSlot>> = Default::default();
    for parameter in data.parameters.iter() {
        let shape = need(&data.shapes, parameter.shape)?;
        let default = if matches!(shape.kind, ParameterKind::VarPositional | ParameterKind::VarKeyword) { DefaultSlot::Collector } else if shape.required { DefaultSlot::Required } else { DefaultSlot::DefinitionTime };
        let variant = *variants.get(&parameter.signature).ok_or_else(|| invalid("parameter has no total signature variant"))?;
        let slot = output.slots.insert(SignatureSlot { parameter: parameter.id(), variant, ordinal: parameter.ordinal, default })?;
        slots.insert(&mut charge, parameter.id(), slot)?;
    }
    for link in data.parameter_links.iter() { output.slot_entities.insert(SignatureSlotEntity { slot: *slots.get(&link.parameter).ok_or_else(|| invalid("parameter link has no signature slot"))?, link: link.id() })?; }
    Ok(output)
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = CallableData::validation_inputs(); inputs.extend(CallableOutput::validation_inputs());
    vec![Invariant { name: "normalized_callable_closure", inputs, create: std::sync::Arc::new(|budget| Box::new(CallableCheck { data: CallableData::new(budget), output: CallableOutput::new(budget), budget: budget.clone() })) }]
}
struct CallableCheck { data: CallableData, output: CallableOutput, budget: ResourceBudget }
impl InvariantCheck for CallableCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if !self.data.visit(relation, batch)? && !self.output.visit(relation, batch)? { return Err(invalid("undeclared callable validation input")); } Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { self.output.matches(&normalize(&self.data, &self.budget)?) }
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    let mut inputs = super::relation_normalization::stage(profile).inputs;
    macro_rules! prior { ($($field:ident: $ty:ty,)*) => { $(inputs.push(stages::RelationUse::stored::<$ty>());)* }; }
    crate::normalized_relation_outputs!(prior);
    inputs.extend(CallableData::stage_inputs()); inputs.sort_by_key(|r| r.name()); inputs.dedup_by_key(|r| r.name());
    stages::Stage { name: "normalize_callables", inputs, outputs: super::callables::relations().iter().map(stages::RelationUse::of_relation).collect(), contributes: vec![], coverage: vec![], provider: None, profiles: vec![profile], effect: stages::Effect::Pure, code: policy_revision(), configuration: ContentHash::of(b"callables/v1") }
}
