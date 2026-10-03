//! Native metadata enriches declared type ports; no class fact closes runtime dispatch.
use super::{attributes::{Fact, classes}, build::{Data, need}, *};
use crate::domain::{charged::{ChargedVec, StateCharge}, normalized::{entities::{EntityRef, ResolutionStatus}, links::ReferenceEntityTarget}, resources::ResourceBudget};

pub(super) enum Selection {
    Metadata {
    entity: Id<EntityRef>, observation: Id<types::TypeObservation>, class: Id<EntityRef>, role: TypePortRole,
    metadata: Option<Id<class_metadata::ClassMetadataObservation>>, status: MetadataSelectionStatus,
    abstract_absence_known: Option<bool>,
    },
    Decorator { entity: Id<EntityRef>, observation: Id<syntax::DeclarationDecorator>,
        assessment: Option<Id<normalized::links::ReferenceEntityAssessment>>, status: DecoratorSelectionStatus },
}
impl HeapSize for Selection { fn heap_bytes(&self) -> usize { 0 } }
impl Selection {
    pub fn emit(&self, scope: Id<ConceptScope>, out: &mut Output) -> Result<(), ModelError> {
        match self {
            Self::Metadata { entity, observation, class, role, metadata, status, abstract_absence_known } => {
                out.metadata_selections.insert(TypeMetadataSelection { scope, entity: *entity, observation: *observation, class: *class, role: *role,
                    metadata: *metadata, status: *status, abstract_absence_known: *abstract_absence_known })?;
            }
            Self::Decorator { entity, observation, assessment, status } => {
                out.decorator_selections.insert(DecoratorSelection { scope, entity: *entity, observation: *observation, assessment: *assessment, status: *status })?;
            }
        }
        Ok(())
    }
}
fn exact(d: &Data, qualification: Id<assertion::AssertionQualification>, ctx: Id<attribution::AnalysisContext>) -> Result<bool, ModelError> {
    let q = need(&d.native.qualifications, qualification)?;
    Ok(q.context == ctx && q.modality == attribution::Modality::Definite && q.approximation == assertion::Approximation::Exact
        && q.condition == conditions::Diagram::always().id())
}
fn resolves(d: &Data, symbol: Id<calls::ProviderSymbol>, entity: Id<EntityRef>, ctx: Id<attribution::AnalysisContext>) -> bool {
    d.native.symbol_resolutions.iter().any(|r| r.symbol == symbol && r.context == ctx && r.status == ResolutionStatus::Resolved && r.entity == Some(entity))
}
pub(super) fn metadata(
    d: &Data, entity: Id<EntityRef>, observation: &types::TypeObservation, role: TypePortRole,
    ctx: Id<attribution::AnalysisContext>, budget: &ResourceBudget, facts: &mut ChargedVec<Fact>, selections: &mut ChargedVec<Selection>, charge: &mut StateCharge,
) -> Result<(), ModelError> {
    let Some(type_classes) = classes(d, observation.term, ctx, budget)? else { return Ok(()); };
    for class in type_classes {
        let mut found = false;
        for m in d.class_metadata.iter().filter(|m| resolves(d, m.class, class, ctx)) {
            if need(&d.native.qualifications, m.qualification)?.context != ctx { continue; }
            found = true;
            let supports = d.metadata_supports.iter().filter(|s| s.assertion == m.id()).collect::<Vec<_>>();
            let status = if !exact(d, observation.qualification, ctx)? || !exact(d, m.qualification, ctx)? { MetadataSelectionStatus::QualifiedUncertainty }
                else if supports.is_empty() { MetadataSelectionStatus::MissingSupport } else { MetadataSelectionStatus::Available };
            selections.push(charge, Selection::Metadata { entity, observation: observation.id(), class, role, metadata: Some(m.id()), status,
                abstract_absence_known: Some(m.abstract_absence_known) })?;
            if status != MetadataSelectionStatus::Available { continue; }
            let mut attributes = vec![];
            for (present, trait_kind) in [
                (m.final_declaration, TypeClassTrait::FinalDeclaration), (m.protocol, TypeClassTrait::Protocol),
                (m.runtime_checkable, TypeClassTrait::RuntimeCheckableProtocol), (m.new_type, TypeClassTrait::NewType),
                (m.enumeration, TypeClassTrait::Enumeration), (m.explicitly_abstract, TypeClassTrait::ExplicitlyAbstract),
                (!m.abstract_members.is_empty(), TypeClassTrait::AbstractMember), (m.explicit_slots, TypeClassTrait::ExplicitSlots),
            ] {
                if present { attributes.push(Attribute::TypeClassTrait { role, class, basis: m.basis, trait_kind }); }
            }
            if let Some(record) = m.record {
                if let Some(options) = m.record_options { need(&d.record_options, options)?; }
                attributes.push(Attribute::TypeClassRecord { role, class, basis: m.basis, record, options: m.record_options });
            }
            if m.deprecated { attributes.push(Attribute::TypeClassDeprecation { role, class, basis: m.basis, message: m.deprecation_message.clone() }); }
            for support in supports {
                for attribute in &attributes {
                    facts.push(charge, Fact { entity, attribute: attribute.clone(), source: IncidenceSource::TypeClassMetadata {
                        observation: observation.id(), metadata: m.id(), support: support.id() } })?;
                }
            }
        }
        if !found { selections.push(charge, Selection::Metadata { entity, observation: observation.id(), class, role, metadata: None,
            status: MetadataSelectionStatus::MissingMetadata, abstract_absence_known: None })?; }
        // Member typing is independently supported; a final/protocol/record flag grants no member authority.
        if !exact(d, observation.qualification, ctx)? { continue; }
        for member in d.class_members.iter().filter(|m| resolves(d, m.class, class, ctx)) {
            if !exact(d, member.qualification, ctx)? || classes(d, member.term, ctx, budget)?.is_none() { continue; }
            for support in d.member_supports.iter().filter(|s| s.assertion == member.id()) {
                facts.push(charge, Fact { entity, attribute: Attribute::TypeClassMember { role, class, basis: member.basis,
                    origin: member.origin, member_kind: member.kind, name: member.name.clone(), term: member.term },
                    source: IncidenceSource::TypeClassMember { observation: observation.id(), member: member.id(), support: support.id() } })?;
            }
        }
    }
    Ok(())
}
/// Only the exact decorator head is selected. References in decorator-call arguments do not
/// describe the decorator, and spelling is never a shared attribute identity.
pub(super) fn decorator(d: &Data, entity: Id<EntityRef>, decorator: &syntax::DeclarationDecorator,
    ctx: Id<attribution::AnalysisContext>, facts: &mut ChargedVec<Fact>, selections: &mut ChargedVec<Selection>, charge: &mut StateCharge) -> Result<(), ModelError> {
    let retain = |selections: &mut ChargedVec<Selection>, charge: &mut StateCharge, assessment, status| {
        selections.push(charge, Selection::Decorator { entity, observation: decorator.id(), assessment, status })
    };
    if !exact(d, decorator.qualification, ctx)? {
        return retain(selections, charge, None, DecoratorSelectionStatus::QualifiedUncertainty);
    }
    let mut head = decorator.decorator;
    // DeclarationDecorator names Ruff's wrapper, whose expression is its unique Child.
    // Resolve that explicit structural edge before selecting a decorator-call's callee.
    if need(&d.native.occurrences, head)?.syntax_kind == source::SyntaxKind::Decorator {
        let expressions = d.native.placements.iter().filter(|p| p.parent == Some(head) && p.field == lexical::SyntaxField::Child
            && d.native.qualifications.get(p.qualification).is_some_and(|q| q.context == ctx)).collect::<Vec<_>>();
        let expression = match expressions.as_slice() {
            [expression] => expression,
            [] => return retain(selections, charge, None, DecoratorSelectionStatus::MissingCorrespondence),
            _ => return retain(selections, charge, None, DecoratorSelectionStatus::Ambiguous),
        };
        if !exact(d, expression.qualification, ctx)? { return retain(selections, charge, None, DecoratorSelectionStatus::QualifiedUncertainty); }
        head = expression.occurrence;
    }
    let heads: Vec<_> = d.native.placements.iter().filter(|p| p.parent == Some(head) && p.field == lexical::SyntaxField::Callee && d.native.qualifications.get(p.qualification).is_some_and(|q| q.context == ctx)).collect();
    if heads.len() > 1 { return retain(selections, charge, None, DecoratorSelectionStatus::Ambiguous); }
    if let Some(placement) = heads.first() {
        if !exact(d, placement.qualification, ctx)? { return retain(selections, charge, None, DecoratorSelectionStatus::QualifiedUncertainty); }
        head = placement.occurrence;
    }
    let mut found = false;
    let mut qualified = false;
    for reference in d.native.references.iter().filter(|r| r.read == head && d.native.qualifications.get(r.qualification).is_some_and(|q| q.context == ctx)) {
        if !exact(d, reference.qualification, ctx)? { qualified = true; continue; }
        for assessment in d.native.reference_assessments.iter().filter(|a| a.reference == reference.id()) {
            found = true;
            let candidates = d.native.reference_candidates.iter().filter(|c| c.assessment == assessment.id()).collect::<Vec<_>>();
            let mut candidates_exact = true;
            for candidate in &candidates {
                candidates_exact &= exact(d, need(&d.native.lexical_resolutions, candidate.resolution)?.qualification, ctx)?;
            }
            let targets = candidates.iter().filter_map(|c| match d.native.reference_targets.get(c.target) {
                Some(ReferenceEntityTarget::Binding { entity, .. }) => Some(*entity), _ => None,
            }).collect::<std::collections::BTreeSet<_>>();
            let status = if !candidates_exact { DecoratorSelectionStatus::QualifiedUncertainty } else { match assessment.status {
                ResolutionStatus::Ambiguous => DecoratorSelectionStatus::Ambiguous,
                ResolutionStatus::Unresolved => DecoratorSelectionStatus::Unresolved,
                ResolutionStatus::Resolved if targets.len() > 1 => DecoratorSelectionStatus::Ambiguous,
                ResolutionStatus::Resolved if targets.is_empty() => DecoratorSelectionStatus::UnsupportedTarget,
                ResolutionStatus::Resolved => DecoratorSelectionStatus::Resolved,
            } };
            retain(selections, charge, Some(assessment.id()), status)?;
            if status != DecoratorSelectionStatus::Resolved { continue; }
            for candidate in candidates {
                if let ReferenceEntityTarget::Binding { entity: target, .. } = need(&d.native.reference_targets, candidate.target)? {
                    facts.push(charge, Fact { entity, attribute: Attribute::ResolvedDecorator { entity: *target }, source: IncidenceSource::ResolvedDecorator {
                        observation: decorator.id(), assessment: assessment.id(), candidate: candidate.id() } })?;
                }
            }
        }
    }
    if qualified { retain(selections, charge, None, DecoratorSelectionStatus::QualifiedUncertainty)?; }
    if !found && !qualified { retain(selections, charge, None, DecoratorSelectionStatus::MissingCorrespondence)?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{class_metadata::*, normalized::entities::*};
    fn id<T>(byte: u8) -> Id<T> { serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap() }
    fn fixture() -> (ResourceBudget, Data, types::TypeObservation, ClassMetadataObservation, Id<EntityRef>, Id<EntityRef>) {
        let b = ResourceBudget::fixed(16 << 20).unwrap();
        let mut d = Data::new(&b);
        let q = assertion::AssertionQualification { context: id(1), scope: id(2), condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite, approximation: assertion::Approximation::Exact, assumptions: id(3) };
        d.native.qualifications.insert(q.clone()).unwrap();
        let class = d.native.refs.insert(EntityRef::Class { class: id(4) }).unwrap();
        let entity = d.native.refs.insert(EntityRef::Callable { callable: id(5) }).unwrap();
        let symbol = id(6);
        d.native.symbol_resolutions.insert(SymbolEntityResolution { symbol, context: q.context, policy: ContentHash::of(b"fixture"),
            status: ResolutionStatus::Resolved, entity: Some(class), reason: EntityReason::DeclarationAgreement }).unwrap();
        let term = d.native.terms.insert(types::TypeTerm::ClassObject { class: symbol }).unwrap();
        let o = types::TypeObservation { qualification: q.id(), subject: id(7), role: types::TypeRole::Parameter, declared: true, term };
        let m = ClassMetadataObservation { qualification: q.id(), class: symbol, basis: MetadataBasis::NativeEffective, metaclass: id(8), custom_metaclass: None,
            final_declaration: true, protocol: false, runtime_checkable: false, new_type: false, enumeration: false, explicitly_abstract: false,
            abstract_members: vec![], abstract_absence_known: false, protocol_members: vec![], explicit_slots: false, slots: None,
            record: None, record_options: None, transform: None, deprecated: true, deprecation_message: Some("use successor".into()) };
        (b, d, o, m, entity, class)
    }
    fn support(m: &ClassMetadataObservation, byte: u8) -> ClassMetadataSupport {
        ClassMetadataSupport { assertion: m.id(), run: id(byte), surface: id(20), evidence: id(21), origin: attribution::Origin::AnalyzerAssertion,
            mode: attribution::ExtractionMode::NativeTraversal, fidelity: attribution::Fidelity::NativeStructural }
    }
    fn collect(d: &Data, b: &ResourceBudget, entity: Id<EntityRef>, o: &types::TypeObservation, role: TypePortRole) -> (Vec<Attribute>, Vec<MetadataSelectionStatus>, usize) {
        let mut f = ChargedVec::default(); let mut selections = ChargedVec::default(); let mut charge = StateCharge::new(b, "test");
        metadata(d, entity, o, role, id(1), b, &mut f, &mut selections, &mut charge).unwrap();
        let count = f.len();
        let attrs = f.iter().map(|f| (f.attribute.id(), f.attribute.clone())).collect::<std::collections::BTreeMap<_,_>>().into_values().collect();
        (attrs, selections.iter().filter_map(|s| match s { Selection::Metadata { status, .. } => Some(*status), _ => None }).collect(), count)
    }
    #[test]
    fn declared_port_metadata_has_exact_positive_set_distinct_roles_and_one_boolean_vote() {
        let (b, mut d, o, m, entity, class) = fixture();
        d.class_metadata.insert(m.clone()).unwrap();
        d.metadata_supports.insert(support(&m, 22)).unwrap(); d.metadata_supports.insert(support(&m, 23)).unwrap();
        let (attrs, states, count) = collect(&d, &b, entity, &o, TypePortRole::Parameter);
        let expected = [Attribute::TypeClassTrait { role: TypePortRole::Parameter, class, basis: MetadataBasis::NativeEffective, trait_kind: TypeClassTrait::FinalDeclaration },
            Attribute::TypeClassDeprecation { role: TypePortRole::Parameter, class, basis: MetadataBasis::NativeEffective, message: Some("use successor".into()) }];
        assert_eq!(attrs.iter().map(Record::id).collect::<std::collections::BTreeSet<_>>(), expected.iter().map(Record::id).collect());
        assert_eq!(states, [MetadataSelectionStatus::Available]); assert_eq!(count, 4, "two support paths retain provenance but only two Boolean attributes");
        let (returns, _, _) = collect(&d, &b, entity, &o, TypePortRole::Return);
        assert!(attrs.iter().all(|a| returns.iter().all(|r| a.id() != r.id())));
    }
    #[test]
    fn unavailable_unsupported_and_qualified_metadata_do_not_become_negative_or_positive_traits() {
        let (b, mut d, mut o, m, entity, _) = fixture();
        let (attrs, states, _) = collect(&d, &b, entity, &o, TypePortRole::Parameter);
        assert!(attrs.is_empty()); assert_eq!(states, [MetadataSelectionStatus::MissingMetadata]);
        d.class_metadata.insert(m.clone()).unwrap();
        let (attrs, states, _) = collect(&d, &b, entity, &o, TypePortRole::Parameter);
        assert!(attrs.is_empty()); assert_eq!(states, [MetadataSelectionStatus::MissingSupport]);
        d.metadata_supports.insert(support(&m, 22)).unwrap();
        let mut q = d.native.qualifications.get(o.qualification).unwrap().clone(); q.approximation = assertion::Approximation::Over;
        o.qualification = d.native.qualifications.insert(q).unwrap();
        let (attrs, states, _) = collect(&d, &b, entity, &o, TypePortRole::Parameter);
        assert!(attrs.is_empty()); assert_eq!(states, [MetadataSelectionStatus::QualifiedUncertainty]);
        o.term = d.native.terms.insert(types::TypeTerm::Any { flavor: types::AnyFlavor::Error }).unwrap();
        let (attrs, states, _) = collect(&d, &b, entity, &o, TypePortRole::Parameter);
        assert!(attrs.is_empty() && states.is_empty(), "Any cannot invent a class identity");
    }
    #[test]
    fn resolved_decorator_identity_retains_paths_and_missing_or_qualified_twins() {
        let (b, mut d, o, _, entity, target) = fixture();
        let wrapper = d.native.occurrences.insert(source::Occurrence { source: id(31), start: 0, end: 15, syntax_kind: source::SyntaxKind::Decorator,
            role: source::OccurrenceRole::Decorator, structural_path: vec![0] }).unwrap();
        let expression = d.native.occurrences.insert(source::Occurrence { source: id(31), start: 1, end: 15, syntax_kind: source::SyntaxKind::ExprName,
            role: source::OccurrenceRole::Read, structural_path: vec![0, 0] }).unwrap();
        d.native.placements.insert(syntax::SyntaxPlacement { qualification: o.qualification, occurrence: expression, parent: Some(wrapper), field: lexical::SyntaxField::Child, ordinal: 0 }).unwrap();
        let mut dec = syntax::DeclarationDecorator { qualification: o.qualification, declaration: id(30), decorator: wrapper, ordinal: 0 };
        let collect = |d: &Data, dec: &syntax::DeclarationDecorator| {
            let mut facts = ChargedVec::default(); let mut selections = ChargedVec::default(); let mut charge = StateCharge::new(&b, "decorator-test");
            decorator(d, entity, dec, id(1), &mut facts, &mut selections, &mut charge).unwrap();
            (facts.iter().map(|f| f.attribute.clone()).collect::<Vec<_>>(), selections.iter().filter_map(|s| match s {
                Selection::Decorator { status, .. } => Some(*status), _ => None,
            }).collect::<Vec<_>>())
        };
        assert_eq!(collect(&d, &dec), (vec![], vec![DecoratorSelectionStatus::MissingCorrespondence]));
        let r = lexical::ReferenceObservation { qualification: o.qualification, read: expression, scope: id(32), parent: wrapper,
            field: lexical::SyntaxField::Decorator, name: "alias_spelling".into() };
        d.native.references.insert(r.clone()).unwrap();
        let a = normalized::links::ReferenceEntityAssessment { reference: r.id(), status: ResolutionStatus::Resolved, reason: normalized::links::LinkReason::ExplicitIdentity };
        d.native.reference_assessments.insert(a.clone()).unwrap();
        let event = id(33);
        let lexical_target = d.native.lexical_targets.insert(lexical::LexicalTarget::Binding { event }).unwrap();
        let reference_target = d.native.reference_targets.insert(ReferenceEntityTarget::Binding { event, entity: target }).unwrap();
        for captured in [false, true] {
            let resolution = d.native.lexical_resolutions.insert(lexical::LexicalResolution { qualification: o.qualification, read: r.read, target: lexical_target, captured }).unwrap();
            d.native.reference_candidates.insert(normalized::links::ReferenceEntityCandidate { assessment: a.id(), resolution, target: reference_target }).unwrap();
        }
        assert_eq!(collect(&d, &dec), (vec![Attribute::ResolvedDecorator { entity: target }; 2], vec![DecoratorSelectionStatus::Resolved]),
            "two resolution paths retain one qualified target identity regardless of spelling");
        let mut q = d.native.qualifications.get(o.qualification).unwrap().clone(); q.approximation = assertion::Approximation::Over;
        dec.qualification = d.native.qualifications.insert(q).unwrap();
        assert_eq!(collect(&d, &dec), (vec![], vec![DecoratorSelectionStatus::QualifiedUncertainty]));
    }
    #[test]
    fn supported_capture_and_protocol_incidence_preserves_unknowns_and_diagnostics() {
        let (b, mut d, o, _, entity, _) = fixture();
        let function = id(40);
        d.native.symbol_resolutions.insert(SymbolEntityResolution { symbol: function, context: id(1), policy: ContentHash::of(b"capture"),
            status: ResolutionStatus::Resolved, entity: Some(entity), reason: EntityReason::DeclarationAgreement }).unwrap();
        d.native.owners.insert(OccurrenceOwnership { occurrence: id(41), owner: id(42), entity }).unwrap();
        let capture = captures::CaptureObservation { qualification: o.qualification, function, name: "outer".into(), origin: captures::CaptureOrigin::OuterFunction,
            declaring: Some(id(43)), mutable: None, timing: captures::CaptureTiming::Unknown };
        let exit = protocols::NativeExitObservation { qualification: o.qualification, subject: id(41), asynchronous: true, receiver: o.term, member: o.term,
            member_name: "__aexit__".into(), normal_result: o.term, exceptional_result: o.term, normal_status: protocols::NativeCallStatus::HardDiagnostics,
            exceptional_status: protocols::NativeCallStatus::NoHardDiagnostics, normal_awaitability: protocols::ExitAwaitability::NotAwaitable,
            exceptional_awaitability: protocols::ExitAwaitability::Awaitable };
        let terminal = protocols::NativeTerminalObservation { qualification: o.qualification, subject: id(41), callee: o.term, return_type: Some(o.term),
            return_is_inferred: Some(true), is_bound_method: false, decision: protocols::TerminalDecision::NonDivergentCallable };
        d.captures.insert(capture.clone()).unwrap(); d.exits.insert(exit.clone()).unwrap(); d.terminals.insert(terminal.clone()).unwrap();
        let collect = |d: &Data, ctx| {
            let mut facts = ChargedVec::default(); let mut charge = StateCharge::new(&b, "dependence-test");
            dependence(d, entity, ctx, &mut facts, &mut charge).unwrap();
            facts.iter().map(|f| f.attribute.id()).collect::<std::collections::BTreeSet<_>>()
        };
        assert!(collect(&d, id(1)).is_empty(), "unsupported observations supply no incidence");
        macro_rules! support {($rows:ident,$ty:ident,$observation:ident)=>{d.$rows.insert($ty {
            assertion: $observation.id(), run: id(44), surface: id(45), evidence: id(46), origin: attribution::Origin::AnalyzerAssertion,
            mode: attribution::ExtractionMode::NativeTraversal, fidelity: attribution::Fidelity::NativeStructural,
        }).unwrap();};}
        use captures::CaptureSupport; use protocols::{NativeExitSupport, NativeTerminalSupport};
        support!(capture_supports,CaptureSupport,capture); support!(exit_supports,NativeExitSupport,exit); support!(terminal_supports,NativeTerminalSupport,terminal);
        let expected = [Attribute::CaptureDependence { name: "outer".into(), origin: captures::CaptureOrigin::OuterFunction, declaring: Some(id(43)), mutable: None, timing: captures::CaptureTiming::Unknown },
            Attribute::ExitProtocolTyping { asynchronous: true, normal: o.term, exceptional: o.term, normal_status: protocols::NativeCallStatus::HardDiagnostics,
                exceptional_status: protocols::NativeCallStatus::NoHardDiagnostics, normal_awaitability: protocols::ExitAwaitability::NotAwaitable, exceptional_awaitability: protocols::ExitAwaitability::Awaitable },
            Attribute::TerminalTyping { decision: protocols::TerminalDecision::NonDivergentCallable, returns: Some(o.term), inferred: Some(true), bound: false }];
        assert_eq!(collect(&d, id(1)), expected.iter().map(Record::id).collect());
        assert!(collect(&d, id(2)).is_empty(), "another context cannot borrow characterization");
    }
}

/// Native contracts are distinct kinds from declared source ports. Native receiver slots remain
/// present and labelled, because bound native terms may have already transformed the receiver.
pub(super) fn variants(d: &Data, entity: Id<EntityRef>, ctx: Id<attribution::AnalysisContext>, b: &ResourceBudget,
    facts: &mut ChargedVec<Fact>, charge: &mut StateCharge, policy: policy::AttributePolicy) -> Result<(), ModelError> {
    let EntityRef::Callable { callable } = need(&d.native.refs, entity)? else { return Ok(()); };
    for variant in d.native.callable_variants.iter().filter(|v| v.callable == Some(*callable) && v.context == ctx && policy.signature_roles.native(v.role)) {
        let Some(native) = variant.native else { continue; };
        let native = need(&d.native.native_signatures, native)?;
        if !exact(d, native.qualification, ctx)? { continue; }
        for support in d.native_signature_supports.iter().filter(|s| s.assertion == native.id()) {
            facts.push(charge, Fact { entity, attribute: Attribute::NativeSignature { role: variant.role, adjustment: variant.adjustment, receiver: native.receiver,
                complete: native.complete, term: native.term }, source: IncidenceSource::NativeSignature {
                observation: native.id(), support: support.id(), variant: variant.id() } })?;
            // Availability and the unmodified message are native metadata. Its exact origin
            // remains on the cited observation; decorator spelling supplies no substitute.
            facts.push(charge, Fact { entity, attribute: Attribute::NativeDeprecation { role: variant.role, basis: class_metadata::MetadataBasis::NativeEffective,
                availability: native.deprecation, message: native.deprecation_message.clone() }, source: IncidenceSource::NativeSignature {
                observation: native.id(), support: support.id(), variant: variant.id() } })?;
        }
        for port in d.native.signature_types.iter() {
            if !exact(d, port.qualification, ctx)? || classes(d, port.term, ctx, b)?.is_none() { continue; }
            let attribute = match need(&d.native.signature_type_subjects, port.subject)? {
                types::SignatureTypeSubject::Return { signature } if *signature == variant.signature => Some(Attribute::NativeReturnType { role: variant.role, adjustment: variant.adjustment, receiver: native.receiver, term: port.term }),
                types::SignatureTypeSubject::Parameter { parameter } => {
                    let parameter = need(&d.native.parameters, *parameter)?;
                    if parameter.signature != variant.signature { continue; }
                    let shape = need(&d.native.shapes, parameter.shape)?;
                    Some(Attribute::NativeParameterType { role: variant.role, adjustment: variant.adjustment, receiver: native.receiver, ordinal: parameter.ordinal,
                        name: shape.name.clone(), parameter_kind: shape.kind, term: port.term })
                }
                _ => None,
            };
            if let Some(attribute) = attribute {
                for support in d.port_supports.iter().filter(|s| s.assertion == port.id()) {
                    facts.push(charge, Fact { entity, attribute: attribute.clone(), source: IncidenceSource::NativePort {
                        observation: port.id(), support: support.id(), variant: variant.id() } })?;
                }
            }
        }
    }
    Ok(())
}

/// Capture and protocol typing are characterizations of observed dependence/calls. Neither
/// unknown capture timing nor diagnostic-free exit typing implies a concrete runtime result.
pub(super) fn dependence(d: &Data, entity: Id<EntityRef>, ctx: Id<attribution::AnalysisContext>,
    facts: &mut ChargedVec<Fact>, charge: &mut StateCharge) -> Result<(), ModelError> {
    for capture in d.captures.iter().filter(|c| resolves(d, c.function, entity, ctx)) {
        if !exact(d, capture.qualification, ctx)? { continue; }
        for support in d.capture_supports.iter().filter(|s| s.assertion == capture.id()) {
            facts.push(charge, Fact { entity, attribute: Attribute::CaptureDependence { name: capture.name.clone(), origin: capture.origin,
                declaring: capture.declaring, mutable: capture.mutable, timing: capture.timing }, source: IncidenceSource::Capture {
                observation: capture.id(), support: support.id() } })?;
        }
    }
    let owned = |subject| d.native.owners.iter().any(|o| o.occurrence == subject && o.entity == entity);
    for exit in d.exits.iter().filter(|e| owned(e.subject)) {
        if !exact(d, exit.qualification, ctx)? { continue; }
        for support in d.exit_supports.iter().filter(|s| s.assertion == exit.id()) {
            facts.push(charge, Fact { entity, attribute: Attribute::ExitProtocolTyping { asynchronous: exit.asynchronous, normal: exit.normal_result,
                exceptional: exit.exceptional_result, normal_status: exit.normal_status, exceptional_status: exit.exceptional_status,
                normal_awaitability: exit.normal_awaitability, exceptional_awaitability: exit.exceptional_awaitability },
                source: IncidenceSource::ExitProtocol { observation: exit.id(), support: support.id() } })?;
        }
    }
    for terminal in d.terminals.iter().filter(|t| owned(t.subject)) {
        if !exact(d, terminal.qualification, ctx)? { continue; }
        for support in d.terminal_supports.iter().filter(|s| s.assertion == terminal.id()) {
            facts.push(charge, Fact { entity, attribute: Attribute::TerminalTyping { decision: terminal.decision, returns: terminal.return_type,
                inferred: terminal.return_is_inferred, bound: terminal.is_bound_method }, source: IncidenceSource::Terminal {
                observation: terminal.id(), support: support.id() } })?;
        }
    }
    Ok(())
}
