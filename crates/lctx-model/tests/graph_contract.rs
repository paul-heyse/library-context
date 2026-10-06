use lctx_model::domain::{self as d, graph::*, *};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Lookup {
    entities: BTreeMap<EntityId, EntityKind>,
    assertions: BTreeSet<AssertionId>,
    sources: BTreeMap<EntityId, u64>,
    subtypes: BTreeMap<EntityId, i16>,
}
impl Lookup {
    fn add(&mut self, entity: &Entity) {
        self.entities.insert(entity.id(), entity.kind());
        if let Some(tag) = entity.subtype() {
            self.subtypes.insert(entity.id(), tag);
        }
        if let Entity::Source(row) = entity {
            self.sources
                .insert(entity.id(), row.byte_len.try_into().unwrap());
        }
    }
}
impl GraphLookup for Lookup {
    fn entity_kind(&self, id: EntityId) -> Result<Option<EntityKind>, ModelError> {
        Ok(self.entities.get(&id).copied())
    }
    fn entity_subtype(&self, id: EntityId) -> Result<Option<i16>, ModelError> {
        Ok(self.subtypes.get(&id).copied())
    }
    fn assertion_exists(&self, id: AssertionId) -> Result<bool, ModelError> {
        Ok(self.assertions.contains(&id))
    }
    fn source_length(&self, id: EntityId) -> Result<Option<u64>, ModelError> {
        Ok(self.sources.get(&id).copied())
    }
}
fn id<T>(n: u8) -> Id<T> {
    serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
}
fn hash(n: u8) -> ContentHash {
    ContentHash([n; 32])
}
fn inline() -> Qualification {
    Qualification::Inline(InlineQualification {
        context: EntityId(hash(1)),
        scope: EntityId(hash(2)),
        condition: EntityId(hash(3)),
        modality: d::attribution::Modality::Definite,
        approximation: d::assertion::Approximation::Exact,
        assumptions: vec![],
    })
}
fn claim(participants: Vec<Participant>) -> Assertion {
    Assertion {
        source: None,
        kind: AssertionKind::ParameterBinding,
        participants,
        qualification: inline(),
        run: None,
        evidence: vec![],
        value: AssertionValue::None,
        derivation: None,
    }
}
fn qualified_lookup() -> Lookup {
    let mut lookup = Lookup::default();
    lookup.entities.extend([
        (EntityId(hash(1)), EntityKind::Context),
        (EntityId(hash(2)), EntityKind::Scope),
        (EntityId(hash(3)), EntityKind::Condition),
    ]);
    lookup
}

#[test]
fn intrinsic_entities_keep_nominal_semantic_keys_and_conflicting_payloads_refuse() {
    let input = d::input::InputRevision::from_entries(vec![]).unwrap();
    let capture = Entity::from(input.clone());
    assert_eq!(capture.id(), EntityId::of(input.id()));
    let original =
        d::source::SourceArtifact::from_bytes(input.id(), "source.py".into(), b"abc").unwrap();
    let source = Entity::from(original.clone());
    let mut changed = original;
    changed.byte_len = 4;
    let changed = Entity::from(changed);
    assert_eq!(source.id(), changed.id());
    assert_ne!(source.content(), changed.content());
    let mut family = FamilyHasher::new(GraphFamily::Entities);
    assert!(family.push(source.id().0, source.content()).unwrap());
    assert!(!family.push(source.id().0, source.content()).unwrap());
    assert!(family.push(changed.id().0, changed.content()).is_err());
    // The empty capture is an independent entity, even with no incident assertion.
    let mut lookup = Lookup::default();
    lookup.add(&capture);
    admit_entity(&capture, &lookup).unwrap();
}
#[test]
fn occurrence_and_evidence_ranges_resolve_to_exact_captured_bytes() {
    let input = d::input::InputRevision::from_entries(vec![]).unwrap();
    let source = d::source::SourceArtifact::from_bytes(input.id(), "a.py".into(), b"abc").unwrap();
    let mut lookup = Lookup::default();
    lookup.add(&Entity::from(input));
    lookup.add(&Entity::from(source.clone()));
    let occurrence = d::source::Occurrence {
        source: source.id(),
        start: 1,
        end: 4,
        syntax_kind: d::source::SyntaxKind::ExprName,
        role: d::source::OccurrenceRole::Syntax,
        structural_path: vec![0],
    };
    assert!(admit_entity(&Entity::from(occurrence), &lookup).is_err());
    let evidence = d::assertion::Evidence::SourceSpan {
        source: source.id(),
        start: 0,
        end: 4,
    };
    assert!(admit_entity(&Entity::from(evidence), &lookup).is_err());
}
#[test]
fn native_supports_are_parallel_addressable_assertions_and_references_are_nominal() {
    let assertion = d::source::SyntaxObservation {
        qualification: id(1),
        occurrence: id(2),
        spelling: "x".into(),
    };
    let support = d::source::SyntaxSupport {
        assertion: assertion.id(),
        run: id(3),
        surface: id(4),
        evidence: id(5),
        origin: d::attribution::Origin::AnalyzerAssertion,
        mode: d::attribution::ExtractionMode::NativeTraversal,
        fidelity: d::attribution::Fidelity::NativeStructural,
    };
    let mut second = support.clone();
    second.run = id(6);
    let first = Assertion::from_record(support.clone()).unwrap();
    let second = Assertion::from_record(second).unwrap();
    assert_ne!(first.id(), second.id());
    assert_eq!(first.id(), AssertionId::of(support.id()));
    first.validate().unwrap();
    let refs = first.references().unwrap();
    assert!(refs.contains(&(Target::Assertion(AssertionId::of(assertion.id())), None)));
    assert!(refs.contains(&(
        Target::Entity(EntityId::of(support.run)),
        Some(EntityKind::Run)
    )));
    let row = Assertion::from_record(assertion.clone()).unwrap();
    assert_eq!(row.id(), AssertionId::of(assertion.id()));
    assert!(row.references().unwrap().contains(&(
        Target::Entity(EntityId::of(assertion.qualification)),
        Some(EntityKind::Qualification)
    )));
    let mut wrong = row;
    wrong.source = Some(SemanticKey::of(support.id()));
    assert!(wrong.validate().is_err());
}
#[test]
fn nary_roles_and_ordered_premises_survive_and_missing_internal_targets_refuse() {
    let mut lookup = qualified_lookup();
    let caller = EntityId(hash(4));
    let argument = EntityId(hash(5));
    let parameter = EntityId(hash(6));
    lookup.entities.extend([
        (caller, EntityKind::Occurrence),
        (argument, EntityKind::Occurrence),
        (parameter, EntityKind::Parameter),
    ]);
    let mut assertion = claim(vec![
        Participant {
            role: ParticipantRole::Caller,
            field: None,
            position: None,
            target: Target::Entity(caller),
        },
        Participant {
            role: ParticipantRole::Argument,
            field: None,
            position: None,
            target: Target::Entity(argument),
        },
        Participant {
            role: ParticipantRole::Parameter,
            field: None,
            position: None,
            target: Target::Entity(parameter),
        },
    ]);
    admit_assertion(&assertion, &lookup).unwrap();
    let original = assertion.id();
    assertion.participants.swap(1, 2);
    assert_ne!(original, assertion.id());
    assertion.participants[0].target = Target::Entity(EntityId(hash(7)));
    assert!(admit_assertion(&assertion, &lookup).is_err());
    let a = AssertionId(hash(8));
    let b = AssertionId(hash(9));
    lookup.assertions.extend([a, b]);
    let mut derived = claim(vec![Participant {
        role: ParticipantRole::Subject,
        field: None,
        position: None,
        target: Target::Entity(caller),
    }]);
    derived.derivation = Some(Derivation {
        rule: "compose".into(),
        revision: 1,
        conclusion: None,
        premises: vec![Target::Assertion(a), Target::Assertion(b)],
        assumptions: vec![],
        outcome: OutcomeKind::Complete,
    });
    admit_assertion(&derived, &lookup).unwrap();
    let first = derived.id();
    derived.derivation.as_mut().unwrap().premises.reverse();
    assert_ne!(first, derived.id());
    assert!(
        admit_derivations([
            (a, &[Target::Assertion(b)][..]),
            (b, &[Target::Assertion(a)][..])
        ])
        .is_err()
    );
    admit_derivations([(a, &[Target::Assertion(b)][..])]).unwrap();
}
#[test]
fn external_uncertainty_is_explicit_but_cannot_fill_internal_roles() {
    let mut lookup = qualified_lookup();
    let provider = EntityId(hash(4));
    lookup.entities.insert(provider, EntityKind::Provider);
    let target = Target::External {
        provider,
        context: EntityId(hash(1)),
        name: "other.module".into(),
        reason: d::normalized::entities::EntityReason::ProviderExternal,
    };
    let mut assertion = claim(vec![Participant {
        role: ParticipantRole::Callee,
        field: None,
        position: None,
        target,
    }]);
    admit_assertion(&assertion, &lookup).unwrap();
    assertion.participants[0].role = ParticipantRole::Declaration;
    assert!(admit_assertion(&assertion, &lookup).is_err());
}
#[test]
fn manifest_distinguishes_exact_empty_partial_and_missing_obligations() {
    let key = OutcomeKey {
        producer: "native".into(),
        scope: EntityId(hash(1)),
        domain: hash(2),
    };
    let mut manifest = Manifest {
        format_version: ARTIFACT_FORMAT_VERSION,
        frontier: d::admission::Frontier::Facts,
        profile: d::stages::Profile::Catalog,
        captures: vec![EntityId(hash(3))],
        semantic_contract: hash(4),
        producers: vec![ProducerImplementation {
            producer: "native".into(),
            implementation: hash(5),
            configuration: hash(6),
        }],
        settings: hash(7),
        families: vec![],
        required_outcomes: vec![key.clone()],
        outcomes: vec![],
        originals: vec![],
        projections: vec![],
        embeddings: vec![],
    };
    assert!(manifest.validate().is_err());
    manifest.outcomes.push(Outcome {
        key,
        status: OutcomeKind::Complete,
        observed: Some(0),
        detail: None,
    });
    manifest.validate().unwrap();
    let complete_empty = manifest.content();
    manifest.outcomes[0].status = OutcomeKind::Partial;
    assert_ne!(complete_empty, manifest.content());
    manifest.validate().unwrap();
    manifest.outcomes[0].status = OutcomeKind::NotRequested;
    manifest.outcomes[0].observed = Some(1);
    assert!(manifest.validate().is_err());
    manifest.format_version = 0;
    assert!(manifest.validate().is_err());
}
#[test]
fn family_content_is_independent_of_transport_chunks_and_preserves_payload() {
    let rows = [(hash(1), hash(2)), (hash(3), hash(4)), (hash(5), hash(6))];
    let mut first = FamilyHasher::new(GraphFamily::Assertions);
    for (key, value) in rows {
        first.push(key, value).unwrap();
    }
    let mut second = FamilyHasher::new(GraphFamily::Assertions);
    for chunk in rows.chunks(2) {
        for (key, value) in chunk {
            second.push(*key, *value).unwrap();
            second.push(*key, *value).unwrap();
        }
    }
    assert_eq!(first.finish(), second.finish());
}

#[test]
fn selected_semantic_inventory_has_nominally_closed_reference_types() {
    let mut absent = BTreeSet::new();
    let mut selected = BTreeSet::new();
    macro_rules! inventory {($($variant:ident:$record:ty,)*)=>{$(selected.insert(<$record>::NAME);)*};}
    lctx_model::graph_entity_records!(inventory);
    lctx_model::graph_assertion_records!(inventory);
    // Check the original packet bindings, before any dependency-lowering filters can hide a loss.
    // Original chunks have their own checked byte transport and are not semantic graph nodes.
    for kind in d::serving::mappings::PacketKind::ALL {
        for relation in &kind.binding().mapping.sources {
            if relation.name() != d::artifact::ArtifactChunk::NAME
                && !selected.contains(relation.name())
            {
                absent.insert(format!("packet {kind:?} requires {}", relation.name()));
            }
        }
    }
    // These are actual request-kernel inputs, excluding compiler-only invariant replay metadata.
    let mut inputs = d::selection::classification::ClassificationData::inputs();
    inputs.extend(d::selection::build::Output::inputs());
    inputs.extend(d::native_requests::NativeInventory::inputs());
    inputs.extend(d::native_requests::PreparationRows::inputs());
    inputs.extend(d::normalized::binding_normalization::BindingData::validation_inputs());
    inputs.extend(d::normalized::binding_normalization::BindingOutput::validation_inputs());
    for input in inputs {
        if !selected.contains(input.name()) {
            absent.insert(format!("request kernel requires {}", input.name()));
        }
    }
    macro_rules! check {($($variant:ident:$record:ty,)*)=>{$(for field in Relation::of::<$record>().fields(){if let Some((_,target))=field.target(){let reference=SemanticReference{field:field.name(),target,key:[0;16],subtype:field.subtype()};if reference_target(&reference).is_err(){absent.insert(format!("{}::{} -> {target}",<$record>::NAME,field.name()));}}})*};}
    lctx_model::graph_entity_records!(check);
    lctx_model::graph_assertion_records!(check);
    assert!(
        absent.is_empty(),
        "unselected semantic references: {absent:#?}"
    );
}

#[test]
fn graph_admission_preserves_nominal_sum_arm_obligations() {
    let node = d::documents::DocumentNode::Passage {
        span: serde_json::from_value(serde_json::json!(vec![1; 16])).unwrap(),
        ordinal: 0,
    };
    let node_entity = Entity::from(node.clone());
    let row = d::documents::PassageObservation {
        qualification: id(2),
        passage: d::documents::DocumentNodePassageId::of(&node).unwrap(),
        level: 0,
        heading: None,
        heading_path: vec![],
        text: "body".into(),
    };
    let assertion = Assertion::from_record(row).unwrap();
    let mut lookup = Lookup::default();
    lookup.add(&node_entity);
    lookup.entities.insert(
        EntityId::of(id::<d::assertion::AssertionQualification>(2)),
        EntityKind::Qualification,
    );
    assert!(
        assertion
            .reference_requirements()
            .unwrap()
            .iter()
            .any(|reference| reference.subtype == Some(0))
    );
    admit_assertion(&assertion, &lookup).unwrap();
    // A structurally valid membership entry with the same nominal family but a wrong arm fails.
    lookup.subtypes.insert(node_entity.id(), 1);
    assert!(admit_assertion(&assertion, &lookup).is_err());
}

#[test]
fn projection_exclusions_share_the_endpoint_category_policy() {
    use lctx_model::domain::{
        normalized::entities::{EntityCategory, EntityRef},
        projection::{ProjectionName, ProjectionSpec},
    };
    let entities = [
        EntityRef::Module { module: id(0) },
        EntityRef::Callable { callable: id(0) },
        EntityRef::Class { class: id(0) },
        EntityRef::Parameter { parameter: id(0) },
        EntityRef::Field { field: id(0) },
        EntityRef::Occurrence { occurrence: id(0) },
        EntityRef::Type { term: id(0) },
        EntityRef::Place { place: id(0) },
    ];
    for name in ProjectionName::ALL {
        let policy = ProjectionSpec::builtin(name);
        let excluded = policy.excluded_categories().collect::<Vec<_>>();
        for (entity, category) in entities.iter().zip(EntityCategory::ALL) {
            assert_eq!(entity.category(), category);
            assert_eq!(policy.accepts(entity), !excluded.contains(&category));
        }
        assert!(excluded.contains(&EntityCategory::Type));
        assert!(excluded.contains(&EntityCategory::Place));
    }
}

#[test]
fn intrinsic_lowering_and_emission_share_the_typed_declaration() {
    let mut emitted = BTreeSet::new();
    macro_rules! emitted {($($variant:ident:$record:ty,)*)=>{$(assert!(emitted.insert(<$record as Record>::NAME),"duplicate intrinsic emission owner");)*};}
    lctx_model::graph_entity_records!(emitted);
    macro_rules! declared {($consumer:ident;$($variant:ident:$kind:ident=>$record:ty,)*)=>{$(assert!(emitted.contains(<$record as Record>::NAME));assert_eq!(<$record as GraphEntityRecord>::GRAPH_KIND,EntityKind::$kind);)*};}
    lctx_model::graph_entity_declarations!(declared, parity);
    // Regression for the concrete missing method-parameter owner found by artifact admission.
    assert!(emitted.contains(<d::analysis::MethodParameters as Record>::NAME));
}

#[test]
fn typed_binding_membership_and_source_call_roles_survive_transport() {
    let binding = d::normalized::bindings::CallBinding {
        attempt: id(1),
        ordinal: 2,
        slot: id(3),
        source: id(4),
        kind: d::calls::BindingKind::Positional,
        projection: id(5),
    };
    let mapped = Assertion::from_record(binding.clone()).unwrap();
    let expected = [
        (
            ParticipantRole::Object,
            "attempt",
            Target::Assertion(AssertionId::of(binding.attempt)),
        ),
        (
            ParticipantRole::Parameter,
            "slot",
            Target::Entity(EntityId::of(binding.slot)),
        ),
        (
            ParticipantRole::Source,
            "source",
            Target::Assertion(AssertionId::of(binding.source)),
        ),
        (
            ParticipantRole::Object,
            "projection",
            Target::Assertion(AssertionId::of(binding.projection)),
        ),
    ];
    assert_eq!(
        mapped.participants,
        expected
            .into_iter()
            .enumerate()
            .map(|(position, (role, field, target))| Participant {
                role,
                field: Some(field.into()),
                position: Some(position as u32),
                target
            })
            .collect::<Vec<_>>()
    );
    assert!(
        matches!(&mapped.value,AssertionValue::Analysis(AnalysisValue::Binding(row)) if row.ordinal==2 && row==&binding)
    );
    let mut changed = binding;
    changed.ordinal = 3;
    assert_ne!(mapped.id(), Assertion::from_record(changed).unwrap().id());

    let member = d::types::TypeSequenceMember {
        sequence: id(6),
        ordinal: 1,
        role: d::types::TypeChildRole::Element,
        child: id(7),
    };
    let mapped_member = Assertion::from_record(member.clone()).unwrap();
    let restored: Assertion =
        serde_json::from_slice(&serde_json::to_vec(&mapped_member).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(mapped_member, restored);
    assert!(
        matches!(&restored.value,AssertionValue::Membership(MembershipValue::TypeSequenceMember(row)) if row==&member)
    );

    let header = d::execution::source_call_records::SourceCallHeader {
        invocation: id(8),
        event: id(9),
        attempt: id(10),
        owner: id(11),
        callee: id(12),
        declaration: id(13),
        qualification: id(14),
        status: d::analysis::policy::EvidenceStatus::StructurallyObserved,
        premises: hash(15),
    };
    let mapped = Assertion::from_record(header.clone()).unwrap();
    let roles = [
        (ParticipantRole::Invocation, "invocation"),
        (ParticipantRole::Object, "event"),
        (ParticipantRole::Object, "attempt"),
        (ParticipantRole::Owner, "owner"),
        (ParticipantRole::Callee, "callee"),
        (ParticipantRole::Declaration, "declaration"),
        (ParticipantRole::Qualification, "qualification"),
    ];
    assert_eq!(
        mapped
            .participants
            .iter()
            .map(|p| (p.role, p.field.as_deref().unwrap(), p.position.unwrap()))
            .collect::<Vec<_>>(),
        roles
            .into_iter()
            .enumerate()
            .map(|(i, (role, field))| (role, field, i as u32))
            .collect::<Vec<_>>()
    );
    let derivation = mapped.derivation.as_ref().unwrap();
    assert_eq!(derivation.rule, "fresh_source_binding");
    assert_eq!(
        derivation.premises,
        vec![
            Target::Entity(EntityId::of(header.invocation)),
            Target::Assertion(AssertionId::of(header.attempt))
        ]
    );
    assert_eq!(derivation.revision, 1);
    assert_eq!(derivation.outcome, OutcomeKind::Complete);
    let restored: Assertion =
        serde_json::from_slice(&serde_json::to_vec(&mapped).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(mapped, restored);
    let mut swapped = mapped.clone();
    swapped.participants.swap(3, 4);
    assert!(swapped.validate().is_err());
    let mut wrong_role = mapped;
    wrong_role.participants[4].role = ParticipantRole::Caller;
    assert!(wrong_role.validate().is_err());
}

#[test]
fn analytic_semantic_output_inventory_is_retained_without_iteration_history() {
    let mut selected = BTreeSet::new();
    macro_rules! selected {($($variant:ident:$record:ty,)*)=>{$(selected.insert(<$record as Record>::NAME);)*};}
    lctx_model::graph_entity_records!(selected);
    lctx_model::graph_assertion_records!(selected);
    // The only omitted output is an incidental quality trace; result, universe, parameters,
    // memberships, weights, availability and provenance each have an explicit semantic owner.
    macro_rules! outputs {($($field:ident:$record:ty,)*)=>{$(if <$record as Record>::NAME!=d::analytics::QualityStep::NAME {assert!(selected.contains(<$record as Record>::NAME),"missing analytic semantic owner {}",<$record as Record>::NAME);})*};}
    lctx_model::analytic_outputs!(outputs);
    let row = d::analytics::Incidence {
        scope: id(1),
        entity: id(2),
        attribute: id(3),
        source: id(4),
    };
    let mapped = Assertion::from_record(row.clone()).unwrap();
    assert_eq!(
        mapped.participants,
        vec![
            Participant {
                role: ParticipantRole::Scope,
                field: Some("scope".into()),
                position: Some(0),
                target: Target::Entity(EntityId::of(row.scope))
            },
            Participant {
                role: ParticipantRole::Subject,
                field: Some("entity".into()),
                position: Some(1),
                target: Target::Entity(EntityId::of(row.entity))
            },
            Participant {
                role: ParticipantRole::Attribute,
                field: Some("attribute".into()),
                position: Some(2),
                target: Target::Entity(EntityId::of(row.attribute))
            },
            Participant {
                role: ParticipantRole::Source,
                field: Some("source".into()),
                position: Some(3),
                target: Target::Assertion(AssertionId::of(row.source))
            },
        ]
    );
    let restored: Assertion =
        serde_json::from_slice(&serde_json::to_vec(&mapped).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(mapped, restored);
    assert!(
        matches!(restored.value,AssertionValue::Provenance(ProvenanceValue::AnalyticIncidence(value)) if value==row)
    );
}

#[test]
fn couse_provenance_retains_exact_usage_and_policy_support() {
    use d::{
        normalized::events::{CallPolicy, CallPolicyAdmission, CallPolicyAssessment, PolicyReason},
        structural::{UsageEvidence, UsageSite},
    };
    let site = UsageSite {
        frame: id(1),
        site: id(2),
        targets: 2,
        complete: true,
        uncertain: false,
    };
    let policy = CallPolicyAssessment {
        event: id(3),
        policy: CallPolicy::Usage,
        event_assessment: id(4),
        admitted: 2,
        members: hash(5),
        reason: PolicyReason::Admitted,
    };
    let admission = CallPolicyAdmission {
        assessment: policy.id(),
        alternative: id(6),
    };
    let left = UsageEvidence {
        site: site.id(),
        event: policy.event,
        policy: policy.id(),
        admission: admission.id(),
        alternative: admission.alternative,
        target: id(7),
    };
    let mut right = left.clone();
    right.target = id(8);
    let pair = d::analytics::PairSource::CoUse {
        scope: site.site,
        left: left.id(),
        right: right.id(),
    };
    let site_assertion = Assertion::from_record(site.clone()).unwrap();
    let policy_assertion = Assertion::from_record(policy.clone()).unwrap();
    let admission_assertion = Assertion::from_record(admission.clone()).unwrap();
    let left_assertion = Assertion::from_record(left.clone()).unwrap();
    let right_assertion = Assertion::from_record(right.clone()).unwrap();
    let pair_assertion = Assertion::from_record(pair.clone()).unwrap();
    assert_eq!(
        left_assertion.participants,
        vec![
            Participant {
                role: ParticipantRole::Object,
                field: Some("site".into()),
                position: Some(0),
                target: Target::Assertion(AssertionId::of(site.id()))
            },
            Participant {
                role: ParticipantRole::Object,
                field: Some("event".into()),
                position: Some(1),
                target: Target::Assertion(AssertionId::of(policy.event))
            },
            Participant {
                role: ParticipantRole::Object,
                field: Some("policy".into()),
                position: Some(2),
                target: Target::Assertion(AssertionId::of(policy.id()))
            },
            Participant {
                role: ParticipantRole::Object,
                field: Some("admission".into()),
                position: Some(3),
                target: Target::Assertion(AssertionId::of(admission.id()))
            },
            Participant {
                role: ParticipantRole::Object,
                field: Some("alternative".into()),
                position: Some(4),
                target: Target::Assertion(AssertionId::of(admission.alternative))
            },
            Participant {
                role: ParticipantRole::Callee,
                field: Some("target".into()),
                position: Some(5),
                target: Target::Entity(EntityId::of(left.target))
            },
        ]
    );
    assert_eq!(
        pair_assertion
            .participants
            .iter()
            .map(|p| p.target.clone())
            .collect::<Vec<_>>(),
        vec![
            Target::Entity(EntityId::of(site.site)),
            Target::Assertion(left_assertion.id()),
            Target::Assertion(right_assertion.id())
        ]
    );
    assert!(
        matches!(&site_assertion.value,AssertionValue::Provenance(ProvenanceValue::StructuralUsageSite(row)) if row==&site)
    );
    assert!(
        matches!(&policy_assertion.value,AssertionValue::Provenance(ProvenanceValue::CallPolicyAssessment(row)) if row==&policy)
    );
    assert!(
        matches!(&admission_assertion.value,AssertionValue::Provenance(ProvenanceValue::CallPolicyAdmission(row)) if row==&admission)
    );
    assert!(
        matches!(&left_assertion.value,AssertionValue::Provenance(ProvenanceValue::StructuralUsageEvidence(row)) if row==&left)
    );
    let mut lookup = Lookup::default();
    lookup.entities.extend([
        (EntityId::of(site.frame), EntityKind::StructuralFrame),
        (EntityId::of(site.site), EntityKind::Occurrence),
        (EntityId::of(left.target), EntityKind::EntityReference),
        (EntityId::of(right.target), EntityKind::EntityReference),
    ]);
    lookup.assertions.extend([
        AssertionId::of(policy.event),
        AssertionId::of(policy.event_assessment),
        AssertionId::of(admission.alternative),
        site_assertion.id(),
        policy_assertion.id(),
        admission_assertion.id(),
        left_assertion.id(),
        right_assertion.id(),
    ]);
    for assertion in [
        &site_assertion,
        &policy_assertion,
        &admission_assertion,
        &left_assertion,
        &right_assertion,
        &pair_assertion,
    ] {
        admit_assertion(assertion, &lookup).unwrap();
        let restored: Assertion =
            serde_json::from_slice(&serde_json::to_vec(assertion).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(assertion, &restored);
    }
    lookup.assertions.remove(&admission_assertion.id());
    assert!(admit_assertion(&left_assertion, &lookup).is_err());
    let mut changed = policy;
    changed.members = hash(9);
    let changed_assertion = Assertion::from_record(changed).unwrap();
    assert_eq!(changed_assertion.id(), policy_assertion.id());
    assert_ne!(changed_assertion.content(), policy_assertion.content());
}


#[test]
fn canonical_projection_companions_survive_transport_and_require_their_subject() {
    use d::projection::{
        ProjectionGap, ProjectionGapReason, ProjectionGapSubject, ProjectionSourceCoverage,
    };
    let subject = ProjectionGapSubject::Event { event: id(21) };
    let entity = Entity::from(subject.clone());
    let restored: Entity = serde_json::from_slice(&serde_json::to_vec(&entity).unwrap()).unwrap();
    assert_eq!(entity, restored);
    assert_eq!(entity.kind(), EntityKind::Subject);
    assert_eq!(record::entity_record::<ProjectionGapSubject>(&restored).unwrap(), subject);

    let gap = ProjectionGap {
        assessment: id(22),
        subject: subject.id(),
        reason: ProjectionGapReason::Unresolved,
    };
    let assertion = Assertion::from_record(gap.clone()).unwrap();
    let restored: Assertion = serde_json::from_slice(&serde_json::to_vec(&assertion).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(assertion, restored);
    assert_eq!(record::assertion_record::<ProjectionGap>(&restored).unwrap(), gap);
    assert!(assertion.references().unwrap().contains(&(
        Target::Entity(entity.id()), Some(EntityKind::Subject),
    )));
    let mut lookup = Lookup::default();
    lookup.add(&entity);
    lookup.assertions.insert(AssertionId::of(gap.assessment));
    admit_assertion(&assertion, &lookup).unwrap();
    lookup.entities.remove(&entity.id());
    assert!(admit_assertion(&assertion, &lookup).is_err());

    let coverage = ProjectionSourceCoverage { assessment: gap.assessment, coverage: id(23) };
    let assertion = Assertion::from_record(coverage.clone()).unwrap();
    let restored: Assertion = serde_json::from_slice(&serde_json::to_vec(&assertion).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(assertion, restored);
    assert_eq!(record::assertion_record::<ProjectionSourceCoverage>(&restored).unwrap(), coverage);

    let mut selected = BTreeSet::new();
    macro_rules! inventory { ($($variant:ident:$ty:ty,)*) => {$(selected.insert(<$ty>::NAME);)*}; }
    lctx_model::graph_entity_records!(inventory);
    lctx_model::graph_assertion_records!(inventory);
    for name in [ProjectionGapSubject::NAME, ProjectionGap::NAME, ProjectionSourceCoverage::NAME] {
        assert!(selected.contains(name), "projection companion omitted from import/export: {name}");
    }
}
