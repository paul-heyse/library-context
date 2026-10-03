//! Independent hand-expected catalog controls, without native extraction, flow or retrieval.
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    catalog::{build::*, *},
    input::*,
    normalized::{callables::*, entities::*},
    resources::ResourceBudget,
    source::*,
    symbols::*,
    syntax::*,
    *,
};
fn nominal<T>(v: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([v; 16].into_iter()))
    .unwrap()
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(16 << 20).unwrap()
}
fn fixture(b: &ResourceBudget) -> (CatalogData, AssertionQualification, Module) {
    let mut data = CatalogData::new(b);
    let source =
        SourceArtifact::from_bytes(nominal::<InputRevision>(1), "api.py".into(), b"pass\n")
            .unwrap();
    let module = Module {
        source: source.id(),
        qualified_name: "api".into(),
    };
    let q = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: nominal(2),
        scope: CoverageScope::Artifact {
            artifact: source.id(),
        }
        .id(),
        condition: conditions::Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    data.artifacts.insert(source).unwrap();
    data.modules.insert(module.clone()).unwrap();
    data.qualifications.insert(q.clone()).unwrap();
    (data, q, module)
}
fn public(
    data: &mut CatalogData,
    q: &AssertionQualification,
    module: &Module,
    name: &str,
    entity: Option<EntityRef>,
    symbol: u8,
) -> Id<PublicExposure> {
    let name = PublicNameObservation {
        qualification: q.id(),
        access: module.id(),
        name: name.into(),
        via_dunder_all: true,
        origin: ExportOrigin::Untraced {}.id(),
    };
    let exposure = PublicExposure {
        access: module.id(),
        context: q.context,
        observation: name.id(),
        origin: name.origin,
        status: if entity.is_some() {
            ResolutionStatus::Resolved
        } else {
            ResolutionStatus::Unresolved
        },
        reason: EntityReason::UntracedExposure,
    };
    data.names.insert(name).unwrap();
    data.exposures.insert(exposure.clone()).unwrap();
    if let Some(entity) = entity {
        let id = data.refs.insert(entity).unwrap();
        let resolution = SymbolEntityResolution {
            symbol: nominal(symbol),
            context: q.context,
            policy: ContentHash::of(b"test"),
            status: ResolutionStatus::Resolved,
            entity: Some(id),
            reason: EntityReason::DeclarationAgreement,
        };
        data.resolutions.insert(resolution.clone()).unwrap();
        data.entity_candidates
            .insert(SymbolEntityCandidate {
                resolution: resolution.id(),
                entity: id,
            })
            .unwrap();
        data.exposure_candidates
            .insert(PublicExposureCandidate {
                exposure: exposure.id(),
                resolution: resolution.id(),
                support: nominal(9),
            })
            .unwrap();
    }
    exposure.id()
}
fn assessment(
    callable: Id<CallableEntity>,
    q: &AssertionQualification,
) -> EffectiveCallableAssessment {
    EffectiveCallableAssessment {
        callable,
        context: q.context,
        decorators: ContentHash::of(b"decorators"),
        policy: ContentHash::of(b"policy"),
        identity: Knowledge::Unknown,
        identity_reason: CallableReason::UnsupportedDecorator,
        signatures: Knowledge::Known,
        signature_reason: CallableReason::EvidenceAgreement,
        descriptor: Knowledge::Unknown,
        descriptor_kind: None,
        descriptor_reason: CallableReason::UnsupportedDecorator,
        body: Knowledge::Unknown,
        body_admitted: false,
        body_reason: CallableReason::UnsupportedDecorator,
        asynchronous: None,
        generator: None,
    }
}
fn variants(
    data: &mut CatalogData,
    q: &AssertionQualification,
    id: Id<CallableEntity>,
) -> (SignatureSlot, SignatureSlot) {
    let seed = data.variants.len() as u8 * 4;
    let a = assessment(id, q);
    data.assessments.insert(a.clone()).unwrap();
    let v1 = SignatureVariant { role: lctx_model::domain::calls::SignatureRole::Source, native: None,
        signature: nominal(21 + seed),
        context: q.context,
        resolution: nominal(22),
        callable: Some(id),
        assessment: Some(a.id()),
        adjustment: SignatureAdjustment::Unknown,
    };
    let v2 = SignatureVariant { role: lctx_model::domain::calls::SignatureRole::Source, native: None,
        signature: nominal(23 + seed),
        ..v1.clone()
    };
    data.variants.insert(v1.clone()).unwrap();
    data.variants.insert(v2.clone()).unwrap();
    let s1 = SignatureSlot {
        parameter: nominal(24 + seed),
        variant: v1.id(),
        ordinal: 0,
        default: DefaultSlot::DefinitionTime,
    };
    let s2 = SignatureSlot {
        parameter: nominal(25 + seed),
        variant: v2.id(),
        ordinal: 0,
        default: DefaultSlot::DefinitionTime,
    };
    data.slots.insert(s1.clone()).unwrap();
    data.slots.insert(s2.clone()).unwrap();
    (s1, s2)
}
#[test]
fn unresolved_slots_and_aliases_are_the_universe_and_resolution_never_changes_identity() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    public(&mut data, &q, &module, "f", None, 3);
    public(
        &mut data,
        &q,
        &module,
        "alias",
        Some(EntityRef::Callable {
            callable: nominal(4),
        }),
        5,
    );
    let first = build(&data, &b).unwrap();
    assert_eq!(first.members.len(), 2);
    assert_eq!(first.exposures.len(), 2);
    assert_eq!(first.callables.len(), 0);
    let id = CatalogMember {
        input: nominal(1),
        access: module.id(),
        path: vec!["f".into()],
        name: "f".into(),
    }
    .id();
    assert!(first.members.get(id).is_some());
    // An improved independent exposure preserves the access slot, adds evidence and known contracts.
    let mut second_q = q.clone();
    second_q.modality = Modality::Candidate;
    data.qualifications.insert(second_q.clone()).unwrap();
    public(
        &mut data,
        &second_q,
        &module,
        "f",
        Some(EntityRef::Callable {
            callable: nominal(4),
        }),
        6,
    );
    let second = build(&data, &b).unwrap();
    assert_eq!(second.members.len(), 2);
    assert_eq!(second.exposures.len(), 3);
    assert!(second.members.get(id).is_some());
    assert!(second.members.iter().all(|r| r.access == module.id()));
}
#[test]
fn source_overloads_survive_unknown_effective_surface_and_none_is_not_unknown() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    let id = nominal(4);
    public(
        &mut data,
        &q,
        &module,
        "f",
        Some(EntityRef::Callable { callable: id }),
        5,
    );
    let (s1, _) = variants(&mut data, &q, id);
    let parameter = ParameterEntity::Source {
        declaration: nominal(30),
    };
    data.parameters.insert(parameter.clone()).unwrap();
    let link = ParameterEntityLink {
        parameter: s1.parameter,
        entity: parameter.id(),
        declaration: None,
    };
    data.parameter_links.insert(link.clone()).unwrap();
    let entity = SignatureSlotEntity {
        slot: s1.id(),
        link: link.id(),
    };
    data.slot_entities.insert(entity).unwrap();
    let literal = value::Literal::None {};
    data.parameter_syntax
        .insert(ParameterSyntaxObservation {
            qualification: q.id(),
            function: nominal(31),
            parameter: nominal(30),
            ordinal: 0,
            kind: ParameterKind::PositionalOrKeyword,
            default: Some(nominal(32)),
            default_literal: Some(literal.id()),
            annotation: None,
        })
        .unwrap();
    let output = build(&data, &b).unwrap();
    assert_eq!(output.members.len(), 1);
    assert_eq!(output.callables.len(), 1);
    assert_eq!(output.invocations.len(), 2);
    assert_eq!(output.options.len(), 2);
    assert!(
        output
            .defaults
            .get(
                CatalogDefault::Literal {
                    literal: literal.id()
                }
                .id()
            )
            .is_some()
    );
    assert!(
        output
            .defaults
            .get(CatalogDefault::Unknown {}.id())
            .is_some()
    );
    assert!(
        output
            .defaults
            .get(CatalogDefault::Absent {}.id())
            .is_none()
    );
    assert_eq!(
        data.assessments.iter().next().unwrap().identity,
        Knowledge::Unknown
    );
    // Removing source syntax cannot leave a fabricated None or absent default.
    data.parameter_syntax = lctx_model::domain::normalized::Rows::new(&b);
    let missing = build(&data, &b).unwrap();
    assert_eq!(missing.defaults.len(), 1);
    assert!(
        missing
            .defaults
            .get(CatalogDefault::Unknown {}.id())
            .is_some()
    );
}
#[test]
fn missing_candidate_and_wrong_context_are_distinct_and_memory_refusal_releases() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    let exposure = public(&mut data, &q, &module, "f", None, 3);
    let r = SymbolEntityResolution {
        symbol: nominal(4),
        context: q.context,
        policy: ContentHash::of(b"test"),
        status: ResolutionStatus::Unresolved,
        entity: None,
        reason: EntityReason::MissingCorrespondence,
    };
    data.resolutions.insert(r.clone()).unwrap();
    data.exposure_candidates
        .insert(PublicExposureCandidate {
            exposure,
            resolution: r.id(),
            support: nominal(5),
        })
        .unwrap();
    let output = build(&data, &b).unwrap();
    assert_eq!(output.candidates.len(), 1);
    assert_eq!(output.candidates.iter().next().unwrap().entity, None);
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        build(&data, &tiny),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
    data.resolutions = lctx_model::domain::normalized::Rows::new(&b);
    let mut wrong = r;
    wrong.context = nominal(6);
    data.resolutions.insert(wrong).unwrap();
    assert!(build(&data, &b).is_err());
}
#[test]
fn shared_invariant_refuses_removed_public_slot_and_forged_default() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    public(
        &mut data,
        &q,
        &module,
        "f",
        Some(EntityRef::Callable {
            callable: nominal(4),
        }),
        5,
    );
    variants(&mut data, &q, nominal(4));
    let mut output = build(&data, &b).unwrap();
    let expected = build(&data, &b).unwrap();
    output.matches(&expected).unwrap();
    output.members = lctx_model::domain::normalized::Rows::new(&b);
    assert!(output.matches(&expected).is_err());
    let mut forged = build(&data, &b).unwrap();
    forged
        .defaults
        .insert(CatalogDefault::Literal {
            literal: value::Literal::None {}.id(),
        })
        .unwrap();
    assert!(forged.matches(&expected).is_err());
    let mut check = (invariants().remove(0).create)(&b);
    macro_rules! visit_data {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::catalog_inputs!(visit_data);
    macro_rules! visit_output {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::catalog_outputs!(visit_output);
    assert!(check.finish().is_err());
}
#[test]
fn inherited_constructor_retains_ancestry_uncertainty_and_normalized_callable_policy() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    let class = nominal(40);
    let callable = nominal(41);
    public(
        &mut data,
        &q,
        &module,
        "Child",
        Some(EntityRef::Class { class }),
        5,
    );
    let owner = data.resolutions.iter().next().unwrap().clone();
    let base_ref = EntityRef::Class { class: nominal(42) };
    data.refs.insert(base_ref.clone()).unwrap();
    let base = SymbolEntityResolution {
        symbol: nominal(43),
        context: q.context,
        policy: owner.policy,
        status: ResolutionStatus::Resolved,
        entity: Some(base_ref.id()),
        reason: EntityReason::DeclarationAgreement,
    };
    data.resolutions.insert(base.clone()).unwrap();
    let observation = ClassAncestryObservation {
        qualification: q.id(),
        class: owner.symbol,
        relation: AncestryRelation::Mro,
        ancestors: nominal(44),
        linearization: Some(Linearization::Prefix),
    };
    data.ancestry_observations
        .insert(observation.clone())
        .unwrap();
    let ancestry = normalized::links::AncestryEntityAssessment {
        observation: observation.id(),
        class: owner.id(),
        status: ResolutionStatus::Ambiguous,
        reason: normalized::links::LinkReason::IncompleteInput,
    };
    data.ancestry.insert(ancestry.clone()).unwrap();
    let sequence_member = SymbolSequenceMember {
        sequence: observation.ancestors,
        ordinal: 0,
        symbol: base.symbol,
    };
    data.sequence_members
        .insert(sequence_member.clone())
        .unwrap();
    data.ancestors
        .insert(normalized::links::AncestryEntityMember {
            assessment: ancestry.id(),
            member: sequence_member.id(),
            resolution: base.id(),
        })
        .unwrap();
    let method = ProviderSymbol {
        provider: nominal(46),
        context: q.context,
        module: nominal(47),
        native_key: "Base.__init__".into(),
        name: "__init__".into(),
        kind: SymbolKind::Method,
    };
    data.symbols.insert(method.clone()).unwrap();
    let reference = EntityRef::Callable { callable };
    data.refs.insert(reference.clone()).unwrap();
    let resolution = SymbolEntityResolution {
        symbol: method.id(),
        context: q.context,
        policy: owner.policy,
        status: ResolutionStatus::Resolved,
        entity: Some(reference.id()),
        reason: EntityReason::DeclarationAgreement,
    };
    data.resolutions.insert(resolution).unwrap();
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
            defining_class: Some(base.symbol),
            overrides: None,
        })
        .unwrap();
    variants(&mut data, &q, callable);
    let output = build(&data, &b).unwrap();
    assert_eq!(output.classes.len(), 1);
    assert_eq!(output.constructors.len(), 1);
    assert_eq!(output.invocations.len(), 2);
    let constructor = output.constructors.iter().next().unwrap();
    assert_eq!(constructor.origin, ConstructorOrigin::Inherited);
    assert!(constructor.ancestry.is_some());
    assert_eq!(constructor.applicability, Knowledge::Unknown);
    assert_eq!(
        data.assessments.iter().next().unwrap().identity,
        Knowledge::Unknown
    );
}
#[test]
fn field_source_expression_and_native_unknown_default_remain_separate() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    let class = nominal(50);
    public(
        &mut data,
        &q,
        &module,
        "Config",
        Some(EntityRef::Class { class }),
        5,
    );
    let left = FieldEntity {
        class,
        name: "left".into(),
    };
    let right = FieldEntity {
        class,
        name: "right".into(),
    };
    data.fields.insert(left.clone()).unwrap();
    data.fields.insert(right.clone()).unwrap();
    for (field, target, value) in [(&left, 51, Some(52)), (&right, 53, None)] {
        let syntax = ClassFieldSyntaxObservation {
            qualification: q.id(),
            class: nominal(50),
            target: nominal(target),
            annotation: Some(nominal(54)),
            value: value.map(nominal),
        };
        data.field_syntax.insert(syntax.clone()).unwrap();
        let declaration = data
            .field_declarations
            .insert(FieldDeclarationLink {
                field: field.id(),
                declaration: syntax.id(),
                binding: nominal(target),
            })
            .unwrap();
        let default = data
            .field_defaults
            .insert(
                value.map_or(normalized::callable_aspects::FieldDefault::Absent {}, |v| {
                    normalized::callable_aspects::FieldDefault::Expression {
                        expression: nominal(v),
                    }
                }),
            )
            .unwrap();
        data.field_default_assessments
            .insert(normalized::callable_aspects::FieldDefaultAssessment {
                declaration,
                default,
            })
            .unwrap();
    }
    let native = lctx_model::domain::types::RecordFieldObservation {
        qualification: q.id(),
        class: nominal(5),
        name: "left".into(),
        record: lctx_model::domain::types::RecordKind::Dataclass,
        ordinal: 0,
        term: nominal(55),
        declared: true,
        declaration: None,
        default_term: None,
        has_default: Some(true),
        init: Some(true),
        alias: None,
        kw_only: None,
        required: None,
        read_only: None,
    };
    data.field_observations.insert(native.clone()).unwrap();
    data.field_links
        .insert(FieldEntityLink {
            field: left.id(),
            observation: native.id(),
        })
        .unwrap();
    let output = build(&data, &b).unwrap();
    assert_eq!(output.options.len(), 3);
    assert_eq!(output.subjects.len(), 2);
    let left_defaults = output
        .options
        .iter()
        .filter(|r| r.subject == CatalogOptionSubject::Field { field: left.id() }.id())
        .map(|r| r.default)
        .collect::<Vec<_>>();
    assert!(
        left_defaults.contains(
            &CatalogDefault::Expression {
                expression: nominal(52)
            }
            .id()
        )
    );
    assert!(left_defaults.contains(&CatalogDefault::Unknown {}.id()));
    let right_defaults = output
        .options
        .iter()
        .filter(|r| r.subject == CatalogOptionSubject::Field { field: right.id() }.id())
        .map(|r| r.default)
        .collect::<Vec<_>>();
    assert_eq!(right_defaults, vec![CatalogDefault::Absent {}.id()]);
}
#[test]
fn nested_paths_join_declaration_identity_and_rebinding_is_explicit() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    let old = ClassEntity::Source {
        declaration: nominal(100),
    };
    let current = ClassEntity::Source {
        declaration: nominal(110),
    };
    data.source_classes.insert(old.clone()).unwrap();
    data.source_classes.insert(current.clone()).unwrap();
    public(
        &mut data,
        &q,
        &module,
        "Retained",
        Some(EntityRef::Class { class: old.id() }),
        5,
    );
    public(
        &mut data,
        &q,
        &module,
        "Rebound",
        Some(EntityRef::Class {
            class: current.id(),
        }),
        6,
    );
    for (class, decl, name) in [
        (nominal(100), 101, "old_method"),
        (nominal(110), 111, "new_method"),
    ] {
        let callable = CallableEntity::Source {
            declaration: nominal(decl),
            kind: CallableKind::Function,
        };
        data.source_callables.insert(callable.clone()).unwrap();
        data.refs
            .insert(EntityRef::Callable {
                callable: callable.id(),
            })
            .unwrap();
        let scope = lexical::LexicalScope {
            owner: class,
            kind: lexical::LexicalScopeKind::Class,
        };
        data.lexical_scopes.insert(scope.clone()).unwrap();
        let event = lexical::BindingEvent {
            site: nominal(decl),
            name: name.into(),
        };
        data.binding_events.insert(event.clone()).unwrap();
        data.bindings
            .insert(lexical::BindingObservation {
                qualification: q.id(),
                event: event.id(),
                scope: scope.id(),
                kind: lexical::BindingEventKind::FunctionDef,
                ordinal: 0,
                value: None,
                static_branch: None,
                static_polarity: None,
            })
            .unwrap();
        data.declarations
            .insert(DeclarationObservation {
                qualification: q.id(),
                declaration: nominal(decl),
                name: nominal(decl + 1),
                kind: DeclarationKind::Function,
                parent: Some(class),
                overload: false,
                docstring: None,
            })
            .unwrap();
        variants(&mut data, &q, callable.id());
    }
    let output = build(&data, &b).unwrap();
    let names = output
        .members
        .iter()
        .map(|r| r.name.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"Retained.old_method"));
    assert!(names.contains(&"Rebound.new_method"));
    assert!(!names.contains(&"Retained.new_method"));
    assert!(!names.contains(&"Rebound.old_method"));
    assert_eq!(output.paths.len(), 2);
    let scope = lexical::LexicalScope {
        owner: nominal(100),
        kind: lexical::LexicalScopeKind::Class,
    };
    let event = lexical::BindingEvent {
        site: nominal(120),
        name: "old_method".into(),
    };
    data.binding_events.insert(event.clone()).unwrap();
    data.bindings
        .insert(lexical::BindingObservation {
            qualification: q.id(),
            event: event.id(),
            scope: scope.id(),
            kind: lexical::BindingEventKind::Assignment,
            ordinal: 1,
            value: Some(nominal(121)),
            static_branch: None,
            static_polarity: None,
        })
        .unwrap();
    let rebound = build(&data, &b).unwrap();
    assert!(
        rebound
            .boundaries
            .iter()
            .any(|r| r.reason == PublicPathBoundaryReason::Rebound)
    );
    assert!(
        rebound
            .paths
            .iter()
            .any(|r| r.disposition == PublicPathDisposition::Candidate)
    );
}
#[test]
fn catalog_stage_declares_completed_nominal_and_shared_invariant_inputs() {
    use std::collections::BTreeSet;
    let model = model().unwrap();
    let stage = stage(stages::Profile::Catalog);
    let declared: BTreeSet<_> = stage.inputs.iter().map(|r| r.name()).collect();
    let facts: BTreeSet<_> = facts_relations().iter().map(Relation::name).collect();
    for relation in model
        .relations()
        .iter()
        .filter(|r| declared.contains(r.name()) && !facts.contains(r.name()))
    {
        for field in relation.fields() {
            if let Some((_, target)) = field.target() {
                assert!(
                    declared.contains(target) || facts.contains(target),
                    "missing completed target {}.{} -> {target}",
                    relation.name(),
                    field.name()
                );
            }
        }
    }
    for invariant in stage.read_invariants(&model).unwrap() {
        for input in &invariant.inputs {
            assert!(
                declared.contains(input.name()) || facts.contains(input.name()),
                "missing completed shared invariant {} -> {}",
                invariant.name,
                input.name()
            );
        }
    }
    assert!(declared.contains(normalized::callable_aspects::AspectSource::NAME));
    assert!(declared.contains(models::ModelCatalog::NAME));
    assert!(!declared.contains(flow::FlowUseObservation::NAME));
}
#[test]
fn source_alias_requires_owned_module_binding_and_same_context_reference() {
    use lexical::*;
    use normalized::links::*;
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    public(&mut data, &q, &module, "alias", None, 3);
    variants(&mut data, &q, nominal(4));
    let owner = nominal(80);
    let scope = LexicalScope {
        owner,
        kind: LexicalScopeKind::Module,
    };
    data.lexical_scopes.insert(scope.clone()).unwrap();
    let module_entity = data
        .refs
        .insert(EntityRef::Module {
            module: module.id(),
        })
        .unwrap();
    data.ownership
        .insert(OccurrenceOwnership {
            occurrence: nominal(81),
            owner,
            entity: module_entity,
        })
        .unwrap();
    let event = BindingEvent {
        site: nominal(81),
        name: "alias".into(),
    };
    data.binding_events.insert(event.clone()).unwrap();
    let binding = BindingObservation {
        qualification: q.id(),
        event: event.id(),
        scope: scope.id(),
        kind: BindingEventKind::Assignment,
        ordinal: 0,
        value: Some(nominal(82)),
        static_branch: None,
        static_polarity: None,
    };
    data.bindings.insert(binding.clone()).unwrap();
    let reference = ReferenceObservation {
        qualification: q.id(),
        read: nominal(82),
        scope: scope.id(),
        parent: nominal(83),
        field: SyntaxField::Value,
        name: "choose".into(),
    };
    data.references.insert(reference.clone()).unwrap();
    let assessment = ReferenceEntityAssessment {
        reference: reference.id(),
        status: ResolutionStatus::Resolved,
        reason: LinkReason::ExplicitIdentity,
    };
    data.reference_assessments
        .insert(assessment.clone())
        .unwrap();
    let raw = LexicalResolution {
        qualification: q.id(),
        read: reference.read,
        target: nominal(84),
        captured: false,
    };
    data.lexical_resolutions.insert(raw.clone()).unwrap();
    let entity = data
        .refs
        .insert(EntityRef::Callable {
            callable: nominal(4),
        })
        .unwrap();
    let target = data
        .reference_targets
        .insert(ReferenceEntityTarget::Binding {
            event: nominal(85),
            entity,
        })
        .unwrap();
    data.reference_candidates
        .insert(ReferenceEntityCandidate {
            assessment: assessment.id(),
            resolution: raw.id(),
            target,
        })
        .unwrap();
    let output = build(&data, &b).unwrap();
    assert_eq!(output.members.len(), 1);
    assert_eq!(output.aliases.len(), 1);
    assert_eq!(output.callables.len(), 1);
    assert_eq!(output.invocations.len(), 2);
    assert!(
        output
            .callables
            .iter()
            .all(|r| r.basis == CatalogContractBasis::SourceOnlyAlias)
    );
    assert_eq!(
        data.exposures.iter().next().unwrap().status,
        ResolutionStatus::Unresolved
    );
    assert!(output.candidates.iter().all(|r| r.candidate.is_none()));
    // Flow-insensitive candidate references retain source contracts without becoming a winning identity.
    let candidate_q = AssertionQualification {
        modality: Modality::Candidate,
        ..q.clone()
    };
    data.qualifications.insert(candidate_q.clone()).unwrap();
    let candidate_raw = LexicalResolution {
        qualification: candidate_q.id(),
        ..raw.clone()
    };
    data.lexical_resolutions = normalized::Rows::new(&b);
    data.lexical_resolutions
        .insert(candidate_raw.clone())
        .unwrap();
    data.reference_candidates = normalized::Rows::new(&b);
    data.reference_candidates
        .insert(ReferenceEntityCandidate {
            assessment: assessment.id(),
            resolution: candidate_raw.id(),
            target,
        })
        .unwrap();
    let candidate = build(&data, &b).unwrap();
    assert!(
        candidate
            .callables
            .iter()
            .all(|r| r.basis == CatalogContractBasis::SourceOnlyAlias)
    );
    assert_eq!(candidate.aliases.len(), 1);
    // The same spelling inside a function cannot supply a public alias.
    let local = LexicalScope {
        owner: nominal(90),
        kind: LexicalScopeKind::Function,
    };
    data.lexical_scopes.insert(local.clone()).unwrap();
    data.bindings = normalized::Rows::new(&b);
    data.bindings
        .insert(BindingObservation {
            scope: local.id(),
            ..binding.clone()
        })
        .unwrap();
    let local = build(&data, &b).unwrap();
    assert_eq!(local.members.len(), 1);
    assert_eq!(local.callables.len(), 0);
    assert_eq!(local.aliases.len(), 0);
    // A function-body global assignment targets a module scope but is not a module-owned event.
    data.bindings = normalized::Rows::new(&b);
    data.bindings.insert(binding.clone()).unwrap();
    data.ownership = normalized::Rows::new(&b);
    data.ownership
        .insert(OccurrenceOwnership {
            occurrence: nominal(81),
            owner: nominal(90),
            entity,
        })
        .unwrap();
    assert_eq!(build(&data, &b).unwrap().aliases.len(), 0);
    // Same artifact and name in a different module-owned scope also do not supply the slot.
    let other = Module {
        source: module.source,
        qualified_name: "other".into(),
    };
    data.modules.insert(other.clone()).unwrap();
    let other_entity = data
        .refs
        .insert(EntityRef::Module { module: other.id() })
        .unwrap();
    data.ownership = normalized::Rows::new(&b);
    data.ownership
        .insert(OccurrenceOwnership {
            occurrence: nominal(81),
            owner,
            entity: other_entity,
        })
        .unwrap();
    data.bindings = normalized::Rows::new(&b);
    data.bindings.insert(binding.clone()).unwrap();
    assert_eq!(build(&data, &b).unwrap().callables.len(), 0);
    // An exact reference from another analysis context is not this exposure's evidence.
    data.ownership = normalized::Rows::new(&b);
    data.ownership
        .insert(OccurrenceOwnership {
            occurrence: nominal(81),
            owner,
            entity: module_entity,
        })
        .unwrap();
    let foreign = AssertionQualification {
        context: nominal(99),
        ..q.clone()
    };
    data.qualifications.insert(foreign.clone()).unwrap();
    data.references = normalized::Rows::new(&b);
    data.references
        .insert(ReferenceObservation {
            qualification: foreign.id(),
            ..reference
        })
        .unwrap();
    assert_eq!(build(&data, &b).unwrap().aliases.len(), 0);
}
#[test]
fn unavailable_effective_parameter_bridge_keeps_original_source_default_separate() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    let callable = CallableEntity::Source {
        declaration: nominal(120),
        kind: CallableKind::Function,
    };
    data.source_callables.insert(callable.clone()).unwrap();
    public(
        &mut data,
        &q,
        &module,
        "wrapped",
        Some(EntityRef::Callable {
            callable: callable.id(),
        }),
        5,
    );
    variants(&mut data, &q, callable.id());
    let parameter = ParameterEntity::Source {
        declaration: nominal(121),
    };
    data.parameters.insert(parameter.clone()).unwrap();
    data.parameter_syntax
        .insert(ParameterSyntaxObservation {
            qualification: q.id(),
            function: nominal(120),
            parameter: nominal(121),
            ordinal: 0,
            kind: ParameterKind::PositionalOrKeyword,
            default: Some(nominal(122)),
            default_literal: Some(value::Literal::None {}.id()),
            annotation: None,
        })
        .unwrap();
    let out = build(&data, &b).unwrap();
    assert_eq!(out.invocations.len(), 2);
    assert_eq!(out.options.len(), 3);
    assert!(out.options.iter().any(|r| {
        r.subject
            == CatalogOptionSubject::SourceParameter {
                parameter: parameter.id(),
            }
            .id()
            && r.default
                == CatalogDefault::Literal {
                    literal: value::Literal::None {}.id(),
                }
                .id()
    }));
    assert_eq!(
        out.options
            .iter()
            .filter(|r| r.default == CatalogDefault::Unknown {}.id())
            .count(),
        2
    );
}
#[test]
fn source_parameter_wrapper_uses_exact_formal_placement_without_pairing_effective_slots() {
    let b = budget();
    let (mut data, q, module) = fixture(&b);
    let callable = CallableEntity::Source {
        declaration: nominal(120),
        kind: CallableKind::Function,
    };
    data.source_callables.insert(callable.clone()).unwrap();
    public(
        &mut data,
        &q,
        &module,
        "wrapped",
        Some(EntityRef::Callable {
            callable: callable.id(),
        }),
        5,
    );
    variants(&mut data, &q, callable.id());
    let occurrence = Occurrence {
        source: module.source,
        start: 0,
        end: 1,
        syntax_kind: SyntaxKind::Parameter,
        role: OccurrenceRole::Parameter,
        structural_path: vec![0],
    };
    data.occurrences.insert(occurrence.clone()).unwrap();
    let parameter = ParameterEntity::Source {
        declaration: occurrence.id(),
    };
    data.parameters.insert(parameter.clone()).unwrap();
    let placement = SyntaxPlacement {
        qualification: q.id(),
        occurrence: occurrence.id(),
        parent: Some(nominal(121)),
        field: lexical::SyntaxField::Child,
        ordinal: 0,
    };
    data.placements.insert(placement.clone()).unwrap();
    let syntax = ParameterSyntaxObservation {
        qualification: q.id(),
        function: nominal(120),
        parameter: nominal(121),
        ordinal: 0,
        kind: ParameterKind::PositionalOrKeyword,
        default: Some(nominal(122)),
        default_literal: Some(value::Literal::None {}.id()),
        annotation: None,
    };
    data.parameter_syntax.insert(syntax.clone()).unwrap();
    let out = build(&data, &b).unwrap();
    assert_eq!(out.options.len(), 3);
    assert!(
        out.evidence
            .get(
                CatalogOptionEvidence::SourceParameter {
                    syntax: syntax.id(),
                    placement: Some(placement.id())
                }
                .id()
            )
            .is_some()
    );
    assert_eq!(
        out.options
            .iter()
            .filter(|r| r.default == CatalogDefault::Unknown {}.id())
            .count(),
        2
    );
    // A child edge in another context cannot establish the formal parameter for this source surface.
    let foreign = AssertionQualification {
        context: nominal(99),
        ..q.clone()
    };
    data.qualifications.insert(foreign.clone()).unwrap();
    data.placements = normalized::Rows::new(&b);
    data.placements
        .insert(SyntaxPlacement {
            qualification: foreign.id(),
            ..placement
        })
        .unwrap();
    let out = build(&data, &b).unwrap();
    assert_eq!(out.options.len(), 2);
    assert!(
        out.options
            .iter()
            .all(|r| r.default == CatalogDefault::Unknown {}.id())
    );
}
