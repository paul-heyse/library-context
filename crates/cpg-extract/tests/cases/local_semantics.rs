//! Actual native flow exercises the shared Local producer and stored witness replay.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    analysis::{self, local, native::*},
    assertion::*,
    local_semantics::*,
    normalized::entity_normalization,
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::*,
    stages::*,
    value::*,
    *,
};
inspector!(Facts, calls::CallTarget);
async fn fixture() -> (
    LocalData,
    local::AnalysisInvocation,
    analysis::AnalysisDefinition,
    ResourceBudget,
) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(
        &typed_driver::files("local_semantics"),
        Facts(tables.clone()),
    )
    .await
    .unwrap();
    let budget = ResourceBudget::fixed(512 << 20).unwrap();
    let mut data = LocalData::new(&budget);
    let mut entities = entity_normalization::EntityData::new(&budget);
    macro_rules! facts {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){entities.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(facts);
    let normalized = entity_normalization::normalize(entities.inputs(), &budget).unwrap();
    macro_rules! input {($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&typed_driver::rows::<$ty>(&tables)).unwrap()).unwrap();)*};}
    lctx_model::entry_value_inputs!(input);
    lctx_model::local_semantic_inputs!(input);
    macro_rules! output {($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&normalized.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(output);
    macro_rules! field_entities{($($field:ident:$ty:ty,)*)=>{$(data.fields.visit(<$ty>::NAME,&<$ty as Record>::encode(&normalized.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(field_entities);
    macro_rules! theory_input {($($field:ident:$ty:ty,)*)=>{$(data.theory.visit(<$ty>::NAME,&<$ty as Record>::encode(&typed_driver::rows::<$ty>(&tables)).unwrap()).unwrap();)*};}
    lctx_model::local_theory_inputs!(theory_input);
    macro_rules! field_input{($($field:ident:$ty:ty,)*)=>{$(data.fields.visit(<$ty>::NAME,&<$ty as Record>::encode(&typed_driver::rows::<$ty>(&tables)).unwrap()).unwrap();)*};}
    lctx_model::local_field_inputs!(field_input);
    let mut relation_data = normalized::relation_normalization::RelationData::new(&budget);
    relation_data.facts = entities;
    relation_data.entities = normalized;
    macro_rules! relationship_input {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){relation_data.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_relation_inputs!(relationship_input);
    let relationships =
        normalized::relation_normalization::normalize(&relation_data, &budget).unwrap();
    macro_rules! relationship_output {($($field:ident:$ty:ty,)*)=>{$(data.theory.visit(<$ty>::NAME,&<$ty as Record>::encode(&relationships.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_relation_outputs!(relationship_output);
    let mut inventory = NativeInventory::new(&budget);
    for input in NativeInventory::inputs() {
        let name = input.name();
        let frames = tables.lock().unwrap();
        if let Some(batches) = frames.get(name) {
            inventory.visit(name, batches).unwrap();
        }
    }
    let inventory = inventory.collect().unwrap();
    for row in inventory.qualifications.iter() {
        data.native.insert(row.clone()).unwrap();
    }
    let (_, definition) = lctx_model::domain::local_semantics::definition();
    let run = data
        .entry
        .runs
        .iter()
        .find(|r| {
            data.entry
                .providers
                .get(r.provider)
                .is_some_and(|p| p.tool == "ty")
        })
        .unwrap();
    let (invocation, _) =
        local::AnalysisInvocation::new(run.input, run.context, definition.id(), None, []);
    (data, invocation, definition, budget)
}
fn owner_name(data: &LocalData, key: &transfer::local::TransferKey) -> String {
    let normalized::entities::EntityRef::Callable { callable } =
        data.entry.refs.get(key.owner).unwrap()
    else {
        panic!()
    };
    let normalized::entities::CallableEntity::Source { declaration, .. } =
        data.entry.callables.get(*callable).unwrap()
    else {
        panic!()
    };
    let occurrence = data.entry.occurrences.get(*declaration).unwrap();
    let artifact = data.entry.artifacts.get(occurrence.source).unwrap();
    let files = typed_driver::files("local_semantics");
    String::from_utf8(
        files[&artifact.path][occurrence.start as usize..occurrence.end as usize].to_vec(),
    )
    .unwrap()
}
#[tokio::test]
async fn native_local_parameter_receiver_and_class_receiver_return_transfers_are_witnessed() {
    let (data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    for name in ["parameter", "method", "class_method"] {
        assert!(
            rows.keys
                .iter()
                .any(|key| owner_name(&data, key).contains(&format!("def {name}("))),
            "missing {name}: {:?}",
            rows.assessments
                .iter()
                .map(|a| a.reason)
                .collect::<Vec<_>>()
        );
    }
    for name in ["assigned", "unbound", "loop", "nested", "nonlocal_write"] {
        assert!(
            !rows
                .keys
                .iter()
                .any(|key| owner_name(&data, key).contains(&format!("def {name}("))),
            "admitted {name}"
        );
    }
    assert!(
        rows.assessments
            .iter()
            .any(|a| a.reason == Some(ObligationKind::CallTransfer))
    );
    assert!(rows.assessments.iter().any(|a| a.reason.is_some()));
    assert_eq!(
        rows.contributions.len() + rows.atom_restrictions.len(),
        rows.supports.len()
    );
    assert!(
        rows.entry_sources
            .iter()
            .any(|source| matches!(source, conditions::entry::EntryAccessSource::Use { .. })),
        "Local publishes separate Use read inventory"
    );
    for key in rows.keys.iter() {
        assert!(rows.roots.iter().any(|r| {
            matches!(r, PlaceRoot::Entry { .. })
                && rows
                    .places
                    .iter()
                    .any(|p| p.root == r.id() && p.id() == key.input)
        }));
    }
}
fn replay(
    data: &LocalData,
    invocation: &local::AnalysisInvocation,
    rows: &LocalRecords,
    mutation: u8,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let invariant = lctx_model::domain::validation::invariants_for::<LocalAssessment>().remove(0);
    let mut check = (invariant.create)(budget);
    macro_rules! input {($($field:ident:$ty:ty,)*)=>{$({let input=ValidationInput::of::<$ty>(&["id"]);let input=if is_vocabulary(input.name()){input.at_epoch(PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.entry.$field.iter().cloned().collect::<Vec<_>>())?)?;})*};}
    lctx_model::entry_value_inputs!(input);
    macro_rules! input {($($field:ident:$ty:ty,)*)=>{$({let input=ValidationInput::of::<$ty>(&["id"]);let input=if is_vocabulary(input.name()){input.at_epoch(PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>())?)?;})*};}
    lctx_model::local_semantic_inputs!(input);
    check.visit(
        local::AnalysisInvocation::NAME,
        &local::AnalysisInvocation::encode(std::slice::from_ref(invocation))?,
    )?;
    let mut contributions = rows.contributions.iter().cloned().collect::<Vec<_>>();
    let mut guards = rows.guards.iter().cloned().collect::<Vec<_>>();
    let mut influences = rows.influences.iter().cloned().collect::<Vec<_>>();
    if mutation == 1 {
        contributions[0].transfer = rows
            .keys
            .iter()
            .find(|row| row.id() != contributions[0].transfer)
            .unwrap()
            .id();
    }
    if mutation == 2 {
        guards[0].entry = rows
            .entries
            .iter()
            .find(|row| row.id() != guards[0].entry)
            .unwrap()
            .id();
    }
    if mutation == 3 {
        influences.clear();
    }
    if mutation == 4 {
        contributions[0].qualification = data
            .entry
            .qualifications
            .iter()
            .find(|row| row.id() != contributions[0].qualification)
            .unwrap()
            .id();
    }
    macro_rules! visit {
        ($ty:ty,$rows:expr) => {
            check.visit(<$ty>::NAME, &<$ty as Record>::encode(&$rows)?)?;
        };
    }
    visit!(LocalContribution, contributions);
    visit!(LocalGuardContribution, guards);
    visit!(transfer::local::ControlInfluence, influences);
    visit!(
        LocalAssessment,
        rows.assessments.iter().cloned().collect::<Vec<_>>()
    );
    visit!(
        LocalGuardAssessment,
        rows.guard_assessments.iter().cloned().collect::<Vec<_>>()
    );
    visit!(
        conditions::entry::EntryAccessSource,
        rows.entry_sources.iter().cloned().collect::<Vec<_>>()
    );
    visit!(
        conditions::entry::EntryValueWitness,
        rows.entries.iter().cloned().collect::<Vec<_>>()
    );
    visit!(
        conditions::stability::StabilityWitness,
        rows.stability.iter().cloned().collect::<Vec<_>>()
    );
    visit!(
        transfer::local::TransferKey,
        rows.keys.iter().cloned().collect::<Vec<_>>()
    );
    check.finish()
}
#[tokio::test]
async fn local_shared_replay_refuses_forged_transfer_guard_entry_influence_and_qualification() {
    let (data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    assert!(!rows.guards.is_empty());
    replay(&data, &invocation, &rows, 0, &budget).unwrap();
    for mutation in 1..=4 {
        assert!(
            replay(&data, &invocation, &rows, mutation, &budget).is_err(),
            "mutation {mutation}"
        );
    }
}
#[tokio::test]
async fn local_cross_context_and_budget_refuse_without_producing_proofs() {
    let (data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    let contribution = rows.contributions.iter().next().unwrap();
    let mut foreign = invocation.clone();
    foreign.context = serde_json::from_value(serde_json::to_value([119u8; 16]).unwrap()).unwrap();
    assert!(
        LocalContribution::derive(
            &data,
            &foreign,
            contribution.value,
            contribution.support,
            &budget
        )
        .unwrap()
        .is_err()
    );
    let tiny = ResourceBudget::fixed(16).unwrap();
    assert!(
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &tiny)
            .is_err()
    );
    assert_eq!(tiny.reserved(), 0);
}

fn entry_replay(
    data: &LocalData,
    source: &conditions::entry::EntryAccessSource,
    witness: &conditions::entry::EntryValueWitness,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut check = (conditions::entry::entry_invariants().remove(0).create)(budget);
    macro_rules! input {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.entry.$field.iter().cloned().collect::<Vec<_>>())?)?;)*};}
    lctx_model::entry_value_inputs!(input);
    check.visit(
        conditions::entry::EntryAccessSource::NAME,
        &<conditions::entry::EntryAccessSource as Record>::encode(std::slice::from_ref(source))?,
    )?;
    check.visit(
        conditions::entry::EntryValueWitness::NAME,
        &conditions::entry::EntryValueWitness::encode(std::slice::from_ref(witness))?,
    )?;
    check.finish()
}
#[tokio::test]
async fn native_entry_sources_refuse_other_values_regions_supports_and_guard_phases() {
    use conditions::entry::*;
    use conditions::stability::StabilityWitness;
    let (mut data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    let contribution = rows
        .contributions
        .iter()
        .find(|c| owner_name(&data, rows.keys.get(c.transfer).unwrap()).contains("def parameter("))
        .unwrap();
    let proof = LocalContribution::derive(
        &data,
        &invocation,
        contribution.value,
        contribution.support,
        &budget,
    )
    .unwrap()
    .unwrap();
    let entry = proof.entry();
    let request = entry.witness().request();
    entry_replay(&data, entry.source(), entry.witness(), &budget).unwrap();
    assert_eq!(
        entry.evidence_status(),
        analysis::policy::EvidenceStatus::StructurallyObserved
    );
    assert_ne!(
        entry.qualification().condition,
        conditions::Diagram::always().id(),
        "guarded return retains region condition"
    );
    let EntryAccessSource::Value {
        observation,
        support,
        region,
        region_support,
    } = entry.source()
    else {
        panic!("value source")
    };
    let other_value = data
        .entry
        .values
        .iter()
        .find(|v| v.use_ != data.entry.values.get(*observation).unwrap().use_)
        .unwrap();
    let other_support = data
        .entry
        .value_supports
        .iter()
        .find(|s| s.assertion == other_value.id())
        .unwrap();
    let wrong_value = EntryAccessSource::Value {
        observation: other_value.id(),
        support: other_support.id(),
        region: *region,
        region_support: *region_support,
    };
    let raw_region = data.entry.regions.get(*region).unwrap();
    let access = data.entry.occurrences.get(request.access).unwrap();
    let sibling = data
        .entry
        .regions
        .iter()
        .find(|r| {
            r.scope == raw_region.scope
                && data
                    .entry
                    .occurrences
                    .get(r.statement)
                    .is_some_and(|s| !access.structural_path.starts_with(&s.structural_path))
        })
        .unwrap();
    let sibling_support = data
        .entry
        .region_supports
        .iter()
        .find(|s| s.assertion == sibling.id())
        .unwrap();
    let wrong_region = EntryAccessSource::Value {
        observation: *observation,
        support: *support,
        region: sibling.id(),
        region_support: sibling_support.id(),
    };
    let nested = data
        .entry
        .regions
        .iter()
        .find(|r| {
            data.entry
                .lexical_scopes
                .get(r.scope)
                .and_then(|s| data.entry.occurrences.get(s.owner))
                .is_some_and(|o| {
                    o.structural_path.len()
                        > data
                            .entry
                            .occurrences
                            .get(
                                data.entry
                                    .lexical_scopes
                                    .get(raw_region.scope)
                                    .unwrap()
                                    .owner,
                            )
                            .unwrap()
                            .structural_path
                            .len()
                })
        })
        .unwrap();
    let nested_support = data
        .entry
        .region_supports
        .iter()
        .find(|s| s.assertion == nested.id())
        .unwrap();
    let wrong_nested = EntryAccessSource::Value {
        observation: *observation,
        support: *support,
        region: nested.id(),
        region_support: nested_support.id(),
    };
    let mut foreign_support = data
        .entry
        .region_supports
        .get(*region_support)
        .unwrap()
        .clone();
    foreign_support.run = data
        .entry
        .runs
        .iter()
        .find(|r| r.id() != request.run)
        .unwrap()
        .id();
    let wrong_support = EntryAccessSource::Value {
        observation: *observation,
        support: *support,
        region: *region,
        region_support: foreign_support.id(),
    };
    data.entry.region_supports.insert(foreign_support).unwrap();
    for source in [wrong_value, wrong_region, wrong_nested, wrong_support] {
        assert!(
            EntryValueWitness::derive_for(&data.entry, request, &source, &budget)
                .unwrap()
                .is_err()
        );
        let mut forged = entry.witness().clone();
        forged.access_source = source.id();
        assert!(
            entry_replay(&data, &source, &forged, &budget).is_err(),
            "coupled source/witness forgery"
        );
    }
    let guard = rows.guards.iter().next().unwrap();
    let guard_proof =
        LocalGuardContribution::derive(&data, &invocation, guard.leaf, guard.support, &budget)
            .unwrap()
            .unwrap();
    let guard_entry = guard_proof.entry();
    assert!(
        EntryValueWitness::derive_for(
            &data.entry,
            guard_entry.witness().request(),
            entry.source(),
            &budget
        )
        .unwrap()
        .is_err(),
        "return source cannot certify guard access"
    );
    assert!(
        StabilityWitness::derive(
            &data.entry,
            data.entry.leaves.get(guard.leaf).unwrap().atom,
            entry
        )
        .is_err()
    );
}
fn occurrence_text(data: &LocalData, id: Id<Occurrence>) -> String {
    let occurrence = data.entry.occurrences.get(id).unwrap();
    let source = data.entry.artifacts.get(occurrence.source).unwrap();
    let bytes = typed_driver::files("local_semantics");
    String::from_utf8(
        bytes[&source.path][occurrence.start as usize..occurrence.end as usize].to_vec(),
    )
    .unwrap()
}
#[tokio::test]
async fn native_finite_domains_scalar_refutation_and_complete_diamond_mro_retain_boundaries() {
    use lctx_model::domain::local_theory::*;
    let (data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    let theory = &rows.theory;
    assert!(
        theory
            .domains
            .iter()
            .any(|domain| domain.completeness == DomainCompleteness::FiniteUnderTypingModel)
    );
    let diamond = theory
        .ancestry
        .iter()
        .find(|row| data.entry.symbols.get(row.class).unwrap().name == "Diamond")
        .unwrap_or_else(|| {
            panic!(
                "no Diamond domain: {:?}",
                theory
                    .type_assessments
                    .iter()
                    .map(|a| a.reason)
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        theory.domains.get(diamond.domain).unwrap().completeness,
        DomainCompleteness::OpenClasses
    );
    for name in ["Diamond", "Left", "Right", "Root"] {
        assert!(
            theory.classes.iter().any(|row| row.ancestry == diamond.id()
                && data.entry.symbols.get(row.ancestor).unwrap().name == name),
            "missing {name}"
        );
    }
    let false_ = theory
        .witnesses
        .iter()
        .find(|row| {
            occurrence_text(&data, data.entry.leaves.get(row.leaf).unwrap().test) == "value == 3"
        })
        .unwrap_or_else(|| {
            panic!(
                "no finite integer witness: {:?}",
                theory
                    .scalar_assessments
                    .iter()
                    .map(|a| (
                        occurrence_text(&data, data.entry.leaves.get(a.leaf).unwrap().test),
                        a.reason
                    ))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(false_.result, PredicateResult::AlwaysFalseUnderTypingModel);
    let membership = theory
        .witnesses
        .iter()
        .find(|row| {
            occurrence_text(&data, data.entry.leaves.get(row.leaf).unwrap().test)
                .starts_with("value in")
        })
        .unwrap();
    assert_eq!(
        membership.result,
        PredicateResult::AlwaysTrueUnderTypingModel
    );
    let positive = theory
        .witnesses
        .iter()
        .find(|row| {
            occurrence_text(&data, data.entry.leaves.get(row.leaf).unwrap().test)
                == "isinstance(value, Root)"
        })
        .unwrap();
    assert_eq!(positive.result, PredicateResult::AlwaysTrueUnderTypingModel);
    assert_eq!(
        theory.domains.get(positive.domain).unwrap().completeness,
        DomainCompleteness::OpenClasses
    );
    assert!(
        theory
            .scalar_assessments
            .iter()
            .any(|row| row.reason == Some(TheoryReason::OpenClassUniverse))
    );
    assert!(
        theory
            .type_assessments
            .iter()
            .any(|row| row.reason == Some(TheoryReason::OpaqueType))
    );
    for name in ["open_nonmember", "open_mixed"] {
        let function = data
            .entry
            .occurrences
            .iter()
            .find(|o| {
                o.syntax_kind == SyntaxKind::StmtFunctionDef
                    && occurrence_text(&data, o.id()).starts_with(&format!("def {name}("))
            })
            .unwrap();
        let leaves = data
            .entry
            .leaves
            .iter()
            .filter(|leaf| {
                data.entry.occurrences.get(leaf.test).is_some_and(|o| {
                    o.source == function.source
                        && o.structural_path.starts_with(&function.structural_path)
                })
            })
            .collect::<Vec<_>>();
        assert!(!leaves.is_empty());
        for leaf in leaves {
            let assessment = theory
                .scalar_assessments
                .iter()
                .find(|a| a.leaf == leaf.id())
                .unwrap();
            assert_eq!(assessment.reason, Some(TheoryReason::OpenClassUniverse));
            assert!(
                assessment.witness.is_none(),
                "open negative or mixed hierarchy does not prove false"
            );
        }
    }
    assert!(
        rows.sources
            .iter()
            .any(|source| matches!(source, local::SupportSource::TheoryWitness { .. })),
        "theory uses actual shared Local proof source"
    );
}
fn replay_theory(
    data: &LocalData,
    invocation: &local::AnalysisInvocation,
    records: &lctx_model::domain::local_theory::TheoryRecords,
    mutation: u8,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    use lctx_model::domain::local_theory::*;
    let invariant =
        lctx_model::domain::validation::invariants_for::<TypeDomainAssessment>().remove(0);
    let mut check = (invariant.create)(budget);
    macro_rules! entry{($($field:ident:$ty:ty,)*)=>{$({let input=ValidationInput::of::<$ty>(&["id"]);let input=if is_vocabulary(input.name()){input.at_epoch(PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.entry.$field.iter().cloned().collect::<Vec<_>>())?)?;})*};}
    lctx_model::entry_value_inputs!(entry);
    macro_rules! inventory{($($field:ident:$ty:ty,)*)=>{$({let input=ValidationInput::of::<$ty>(&["id"]);let input=if is_vocabulary(input.name()){input.at_epoch(PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.theory.$field.iter().cloned().collect::<Vec<_>>())?)?;})*};}
    lctx_model::local_theory_inputs!(inventory);
    check.visit(
        local::AnalysisInvocation::NAME,
        &local::AnalysisInvocation::encode(std::slice::from_ref(invocation))?,
    )?;
    macro_rules! output{($($field:ident:$ty:ty,)*)=>{$({let mut rows=records.$field.iter().cloned().collect::<Vec<_>>();if <$ty>::NAME==TypeDomain::NAME&&mutation==1{let mut typed=TypeDomain::decode(&<$ty as Record>::encode(&rows)?)?;let open=typed.iter_mut().find(|d|d.completeness==DomainCompleteness::OpenClasses).unwrap();open.completeness=DomainCompleteness::FiniteUnderTypingModel;check.visit(<$ty>::NAME,&TypeDomain::encode(&typed)?)?;}else if <$ty>::NAME==TheoryWitness::NAME&&mutation==2{let mut typed=TheoryWitness::decode(&<$ty as Record>::encode(&rows)?)?;let row=typed.iter_mut().find(|row|row.result==PredicateResult::AlwaysFalseUnderTypingModel).unwrap();row.result=PredicateResult::AlwaysTrueUnderTypingModel;check.visit(<$ty>::NAME,&TheoryWitness::encode(&typed)?)?;}else if (<$ty>::NAME==ClassDomainMember::NAME&&mutation==3)||(<$ty>::NAME==BuiltinOperandWitness::NAME&&mutation==4){rows.clear();check.visit(<$ty>::NAME,&<$ty as Record>::encode(&rows)?)?;}else{check.visit(<$ty>::NAME,&<$ty as Record>::encode(&rows)?)?;}})*};}
    lctx_model::local_theory_outputs!(output);
    check.finish()
}
#[tokio::test]
async fn theory_shared_replay_refuses_forged_exhaustiveness_scalar_truth_and_ancestry_members() {
    let (data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    replay_theory(&data, &invocation, &rows.theory, 0, &budget).unwrap();
    for mutation in 1..=4 {
        assert!(
            replay_theory(&data, &invocation, &rows.theory, mutation, &budget).is_err(),
            "forgery {mutation}"
        );
    }
}
#[tokio::test]
async fn local_definitions_are_bound_to_code_parameters_and_interpretation() {
    let (data, invocation, definition, budget) = fixture().await;
    let before = budget.reserved();
    for mutation in 0..3 {
        let mut definition = definition.clone();
        match mutation {
            0 => definition.semantic_version = ContentHash::of(b"foreign local semantics"),
            1 => {
                let (mut parameters, _) = lctx_model::domain::local_semantics::definition();
                parameters.work = Some(10);
                definition.parameters = parameters.id();
            }
            _ => definition.interpretation = analysis::Interpretation::Heuristic,
        }
        assert!(
            lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
                .is_err()
        );
    }
    assert_eq!(budget.reserved(), before);
}

#[tokio::test]
async fn native_type_domains_refuse_missing_prefix_mro_partial_coverage_and_foreign_frames() {
    use lctx_model::domain::{attribution::*, local_theory::*, symbols::*};
    use normalized::Rows;
    let (mut data, invocation, definition, budget) = fixture().await;
    let records =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    let class = records
        .theory
        .ancestry
        .iter()
        .find(|row| data.entry.symbols.get(row.class).unwrap().name == "Diamond")
        .unwrap();
    let domain = records.theory.domains.get(class.domain).unwrap();
    let (observation, support) = (domain.observation, domain.support);
    let mut foreign = invocation.clone();
    foreign.context = serde_json::from_value(serde_json::to_value([92u8; 16]).unwrap()).unwrap();
    assert!(matches!(
        TypeDomain::derive(
            &TheoryData {
                entry: &data.entry,
                inventory: &data.theory
            },
            &foreign,
            observation,
            support,
            &budget
        )
        .unwrap(),
        Err(TheoryReason::IncompatibleFrame)
    ));
    // The compound fixture has an unavailable selected trace. Its module is partial, while
    // this exact Diamond operand query remains available with native support.
    assert!(
        data.entry
            .coverage
            .iter()
            .any(|row| row.family == FactFamily::Types
                && row.status == CoverageStatus::Partial
                && row.reason == Some(ObligationKind::MissingEvidence))
    );
    let queries = std::mem::replace(&mut data.theory.type_queries, Rows::new(&budget));
    assert!(
        matches!(
            TypeDomain::derive(
                &TheoryData {
                    entry: &data.entry,
                    inventory: &data.theory
                },
                &invocation,
                observation,
                support,
                &budget
            )
            .unwrap(),
            Err(TheoryReason::IncompleteCoverage)
        ),
        "partial module cannot grant a missing selected query"
    );
    for row in queries.iter() {
        let mut row = row.clone();
        if row.observation == Some(observation) {
            row.status = types::TypeQueryStatus::Partial;
            row.reason = Some(ObligationKind::BudgetReached);
        }
        data.theory.type_queries.insert(row).unwrap();
    }
    assert!(
        matches!(
            TypeDomain::derive(
                &TheoryData {
                    entry: &data.entry,
                    inventory: &data.theory
                },
                &invocation,
                observation,
                support,
                &budget
            )
            .unwrap(),
            Err(TheoryReason::IncompleteCoverage)
        ),
        "a partial selected query cannot certify a domain"
    );
    data.theory.type_queries = queries;
    let original = std::mem::replace(&mut data.theory.ancestry, Rows::new(&budget));
    assert!(matches!(
        TypeDomain::derive(
            &TheoryData {
                entry: &data.entry,
                inventory: &data.theory
            },
            &invocation,
            observation,
            support,
            &budget
        )
        .unwrap(),
        Err(TheoryReason::IncompleteMro)
    ));
    for row in original.iter() {
        let mut row = row.clone();
        if row.class == class.class && row.relation == AncestryRelation::Mro {
            row.linearization = Some(Linearization::Prefix);
        }
        data.theory.ancestry.insert(row).unwrap();
    }
    assert!(matches!(
        TypeDomain::derive(
            &TheoryData {
                entry: &data.entry,
                inventory: &data.theory
            },
            &invocation,
            observation,
            support,
            &budget
        )
        .unwrap(),
        Err(TheoryReason::IncompleteMro)
    ));
    data.theory.ancestry = original;
    let coverage = std::mem::replace(&mut data.entry.coverage, Rows::new(&budget));
    for row in coverage.iter() {
        let mut row = row.clone();
        if row.family == FactFamily::Types {
            row.status = CoverageStatus::Partial;
            row.reason = Some(ObligationKind::IncompleteDomain);
        }
        data.entry.coverage.insert(row).unwrap();
    }
    assert!(matches!(
        TypeDomain::derive(
            &TheoryData {
                entry: &data.entry,
                inventory: &data.theory
            },
            &invocation,
            observation,
            support,
            &budget
        )
        .unwrap(),
        Err(TheoryReason::IncompleteCoverage)
    ));
    let tiny = ResourceBudget::fixed(16).unwrap();
    data.entry.coverage = coverage;
    assert!(
        TypeDomain::derive(
            &TheoryData {
                entry: &data.entry,
                inventory: &data.theory
            },
            &invocation,
            observation,
            support,
            &tiny
        )
        .is_err()
    );
    assert_eq!(tiny.reserved(), 0);
}

#[tokio::test]
async fn native_builtin_type_predicates_use_supported_operands_and_refuse_shadowed_functions() {
    use lctx_model::domain::local_theory::*;
    let (data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    let type_witness = rows
        .theory
        .witnesses
        .iter()
        .find(|w| {
            occurrence_text(&data, data.entry.leaves.get(w.leaf).unwrap().test)
                == "type(value) is int"
        })
        .unwrap_or_else(|| {
            panic!(
                "type predicate boundary: {:?}",
                rows.theory
                    .scalar_assessments
                    .iter()
                    .map(|a| (
                        occurrence_text(&data, data.entry.leaves.get(a.leaf).unwrap().test),
                        a.reason
                    ))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        type_witness.result,
        PredicateResult::AlwaysTrueUnderTypingModel
    );
    assert!(type_witness.builtin.is_some());
    let instance = rows
        .theory
        .witnesses
        .iter()
        .find(|w| {
            occurrence_text(&data, data.entry.leaves.get(w.leaf).unwrap().test)
                == "isinstance(value, str)"
        })
        .unwrap_or_else(|| {
            panic!(
                "instance predicate boundary: {:?}",
                rows.theory
                    .scalar_assessments
                    .iter()
                    .map(|a| (
                        occurrence_text(&data, data.entry.leaves.get(a.leaf).unwrap().test),
                        a.reason
                    ))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        instance.result,
        PredicateResult::AlwaysFalseUnderTypingModel
    );
    for w in rows.theory.witnesses.iter() {
        let leaf = data.entry.leaves.get(w.leaf).unwrap();
        let occurrence = data.entry.occurrences.get(leaf.test).unwrap();
        assert!(
            !data.entry.occurrences.iter().any(|owner| matches!(
                owner.syntax_kind,
                SyntaxKind::StmtFunctionDef
            ) && occurrence.source == owner.source
                && occurrence
                    .structural_path
                    .starts_with(&owner.structural_path)
                && occurrence_text(&data, owner.id()).starts_with("def shadowed_")),
            "shadowed builtin was admitted"
        );
    }
    replay_theory(&data, &invocation, &rows.theory, 0, &budget).unwrap();
}

#[tokio::test]
async fn native_scalar_admission_refuses_a_type_known_only_outside_the_leaf_domain() {
    use lctx_model::domain::{conditions::Diagram, local_theory::*, types::*};
    use normalized::{Rows, links::*};
    let (mut data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    let witness = rows
        .theory
        .witnesses
        .iter()
        .find(|w| {
            occurrence_text(&data, data.entry.leaves.get(w.leaf).unwrap().test) == "value == 3"
        })
        .unwrap();
    let old_link = data
        .theory
        .operand_links
        .get(witness.operand_link)
        .unwrap()
        .clone();
    let old = data
        .theory
        .type_observations
        .get(old_link.observation)
        .unwrap()
        .clone();
    let original_support = data
        .theory
        .type_supports
        .iter()
        .find(|s| s.assertion == old.id())
        .unwrap()
        .clone();
    let (condition, nodes) = Diagram::never().records();
    data.entry.conditions.insert(condition.clone()).unwrap();
    for node in nodes {
        data.entry.condition_nodes.insert(node).unwrap();
    }
    let qualification = AssertionQualification {
        condition: condition.id(),
        ..data
            .entry
            .qualifications
            .get(old.qualification)
            .unwrap()
            .clone()
    };
    data.entry
        .qualifications
        .insert(qualification.clone())
        .unwrap();
    let observation = TypeObservation {
        qualification: qualification.id(),
        ..old.clone()
    };
    let support = TypeSupport {
        assertion: observation.id(),
        ..original_support.clone()
    };
    data.theory
        .type_observations
        .insert(observation.clone())
        .unwrap();
    data.theory.type_supports.insert(support).unwrap();
    let links = data
        .theory
        .operand_links
        .iter()
        .filter(|l| l.id() != old_link.id())
        .cloned()
        .collect::<Vec<_>>();
    data.theory.operand_links = Rows::new(&budget);
    for link in links {
        data.theory.operand_links.insert(link).unwrap();
    }
    data.theory
        .operand_links
        .insert(TestOperandTypeLink {
            observation: observation.id(),
            ..old_link
        })
        .unwrap();
    assert!(matches!(
        TheoryWitness::derive(
            &TheoryData {
                entry: &data.entry,
                inventory: &data.theory
            },
            &invocation,
            witness.leaf,
            witness.support,
            &budget
        )
        .unwrap(),
        Err(TheoryReason::IncompatibleFrame)
    ));
}
fn replay_fields(
    data: &LocalData,
    invocation: &local::AnalysisInvocation,
    records: &LocalRecords,
    mutation: u8,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    use local_fields::*;
    let invariant =
        lctx_model::domain::validation::invariants_for::<FieldLocationAssessment>().remove(0);
    let mut check = (invariant.create)(budget);
    macro_rules! entry{($($field:ident:$ty:ty,)*)=>{$({let input=ValidationInput::of::<$ty>(&["id"]);let input=if is_vocabulary(input.name()){input.at_epoch(PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.entry.$field.iter().cloned().collect::<Vec<_>>())?)?;})*};}
    lctx_model::entry_value_inputs!(entry);
    macro_rules! theory{($($field:ident:$ty:ty,)*)=>{$({let input=ValidationInput::of::<$ty>(&["id"]);let input=if is_vocabulary(input.name()){input.at_epoch(PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.theory.$field.iter().cloned().collect::<Vec<_>>())?)?;})*};}
    lctx_model::local_theory_inputs!(theory);
    macro_rules! fields{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.fields.$field.iter().cloned().collect::<Vec<_>>())?)?;)*};}
    lctx_model::local_field_inputs!(fields);
    check.visit(
        local::AnalysisInvocation::NAME,
        &local::AnalysisInvocation::encode(std::slice::from_ref(invocation))?,
    )?;
    check.visit(
        conditions::entry::EntryValueWitness::NAME,
        &conditions::entry::EntryValueWitness::encode(
            &records.entries.iter().cloned().collect::<Vec<_>>(),
        )?,
    )?;
    check.visit(
        conditions::entry::EntryAccessSource::NAME,
        &<conditions::entry::EntryAccessSource as Record>::encode(
            &records.entry_sources.iter().cloned().collect::<Vec<_>>(),
        )?,
    )?;
    check.visit(
        FieldLocationAssessment::NAME,
        &FieldLocationAssessment::encode(
            &records
                .fields
                .assessments
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
        )?,
    )?;
    let mut locations = records.fields.locations.iter().cloned().collect::<Vec<_>>();
    if mutation == 1 {
        locations[0].phase = calls::CallPhase::Definition;
    }
    if mutation == 2 {
        locations[0].receiver = records
            .places
            .iter()
            .find(|e| e.id() != locations[0].receiver)
            .unwrap()
            .id();
    }
    check.visit(FieldLocation::NAME, &FieldLocation::encode(&locations)?)?;
    let mut candidates = records
        .fields
        .candidates
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    if mutation == 3 {
        let other = data
            .fields
            .fields
            .iter()
            .find(|f| f.id() != candidates[0].field)
            .unwrap();
        candidates[0].field = other.id();
    }
    check.visit(
        FieldLocationCandidate::NAME,
        &FieldLocationCandidate::encode(&candidates)?,
    )?;
    check.finish()
}
#[tokio::test]
async fn native_field_locations_keep_two_fields_and_unrelated_readers_distinct_and_state_open() {
    use local_fields::*;
    let (data, invocation, definition, budget) = fixture().await;
    let records =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    assert!(
        !records.fields.locations.is_empty(),
        "{:?}",
        records
            .fields
            .assessments
            .iter()
            .map(|a| a.reason)
            .collect::<Vec<_>>()
    );
    assert!(
        !records.fields.candidates.is_empty(),
        "no typed source class field candidates"
    );
    let mut classes = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for candidate in records.fields.candidates.iter() {
        let field = data.fields.fields.get(candidate.field).unwrap();
        classes.insert(field.class);
        names.insert(field.name.as_str().to_owned());
        let location = records.fields.locations.get(candidate.location).unwrap();
        let load = data.fields.loads.get(location.load).unwrap();
        assert_eq!(load.name, field.name.as_str());
        assert_eq!(location.phase, calls::CallPhase::Call);
        assert_eq!(
            location.state,
            FieldStateBoundary::AllocationAliasAndMutationUnavailable
        );
    }
    assert!(classes.len() >= 2, "unrelated readers do not merge classes");
    assert!(names.contains("left") && names.contains("right"));
    for location in records.fields.locations.iter() {
        let normalized::entities::EntityRef::Callable { callable } =
            data.entry.refs.get(location.owner).unwrap()
        else {
            panic!()
        };
        let normalized::entities::CallableEntity::Source { declaration, .. } =
            data.entry.callables.get(*callable).unwrap()
        else {
            panic!()
        };
        assert!(!occurrence_text(&data, *declaration).starts_with("def rebinding("));
        let proof = FieldLocation::derive(
            &FieldData {
                entry: &data.entry,
                theory: &data.theory,
                inventory: &data.fields,
            },
            &invocation,
            location.load,
            location.support,
            &budget,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(
            proof.entry().source(),
            conditions::entry::EntryAccessSource::Use { .. }
        ));
        assert_eq!(proof.location(), location);
    }
    replay_fields(&data, &invocation, &records, 0, &budget).unwrap();
    for mutation in 1..=3 {
        assert!(
            replay_fields(&data, &invocation, &records, mutation, &budget).is_err(),
            "field forgery {mutation}"
        );
    }
}
#[tokio::test]
async fn native_field_locations_refuse_wrong_role_foreign_placement_support_and_context() {
    use local_fields::*;
    use normalized::Rows;
    for mutation in 0..3 {
        let (mut data, invocation, definition, budget) = fixture().await;
        let records =
            lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
                .unwrap();
        let location = records.fields.locations.iter().next().unwrap().clone();
        if mutation == 0 {
            let original = data
                .entry
                .placements
                .get(location.placement)
                .unwrap()
                .clone();
            let keep = data
                .entry
                .placements
                .iter()
                .filter(|r| r.id() != original.id())
                .cloned()
                .collect::<Vec<_>>();
            data.entry.placements = Rows::new(&budget);
            for row in keep {
                data.entry.placements.insert(row).unwrap();
            }
            let wrong = syntax::SyntaxPlacement {
                field: lexical::SyntaxField::Right,
                ..original
            };
            data.entry.placements.insert(wrong).unwrap();
        } else if mutation == 1 {
            let original = data
                .entry
                .placement_supports
                .get(location.placement_support)
                .unwrap()
                .clone();
            let keep = data
                .entry
                .placement_supports
                .iter()
                .filter(|r| r.id() != original.id())
                .cloned()
                .collect::<Vec<_>>();
            data.entry.placement_supports = Rows::new(&budget);
            for row in keep {
                data.entry.placement_supports.insert(row).unwrap();
            }
            let other = data
                .entry
                .runs
                .iter()
                .find(|run| run.id() != original.run)
                .unwrap()
                .id();
            let wrong = syntax::SyntaxPlacementSupport {
                run: other,
                ..original
            };
            data.entry.placement_supports.insert(wrong).unwrap();
        } else {
            let mut foreign = invocation.clone();
            foreign.context =
                serde_json::from_value(serde_json::to_value([33u8; 16]).unwrap()).unwrap();
            assert!(
                FieldLocation::derive(
                    &FieldData {
                        entry: &data.entry,
                        theory: &data.theory,
                        inventory: &data.fields
                    },
                    &foreign,
                    location.load,
                    location.support,
                    &budget
                )
                .unwrap()
                .is_err()
            );
            continue;
        }
        assert!(
            FieldLocation::derive(
                &FieldData {
                    entry: &data.entry,
                    theory: &data.theory,
                    inventory: &data.fields
                },
                &invocation,
                location.load,
                location.support,
                &budget
            )
            .unwrap()
            .is_err()
        );
        assert!(
            replay_fields(&data, &invocation, &records, 0, &budget).is_err(),
            "coupled field premise forgery {mutation}"
        );
    }
}

#[tokio::test]
async fn native_assignment_transfer_retains_exact_definition_premise_and_refuses_rebound_input() {
    let (mut data, invocation, definition, budget) = fixture().await;
    let rows =
        lctx_model::domain::local_semantics::produce(&data, &invocation, &definition, &budget)
            .unwrap();
    let c = rows
        .contributions
        .iter()
        .find(|c| {
            c.definition.is_some()
                && owner_name(&data, rows.keys.get(c.transfer).unwrap())
                    .starts_with("def alias_assignment(")
        })
        .expect("identity assignment derives from RHS and exact native target");
    let observation = data
        .entry
        .definition_observations
        .get(c.definition.unwrap())
        .unwrap();
    let target = data.entry.definitions.get(observation.definition).unwrap();
    let value = data.entry.values.get(c.value).unwrap();
    assert_eq!(observation.value, Some(value.sink));
    assert_ne!(target.occurrence, value.sink);
    assert_eq!(rows.keys.get(c.transfer).unwrap().output, target.place);
    assert!(!rows.contributions.iter().any(|c| {
        c.definition.is_some()
            && owner_name(&data, rows.keys.get(c.transfer).unwrap())
                .starts_with("def rebound_assignment(")
    }));
    let supports = std::mem::replace(
        &mut data.entry.definition_supports,
        normalized::Rows::new(&budget),
    );
    for support in supports
        .iter()
        .filter(|s| Some(s.id()) != c.definition_support)
    {
        data.entry
            .definition_supports
            .insert(support.clone())
            .unwrap();
    }
    assert!(
        LocalContribution::derive(&data, &invocation, c.value, c.support, &budget)
            .unwrap()
            .is_err(),
        "assignment target requires native definition support"
    );
}
