//! Actual source/effective signature packet closure, original nominal IDs, and indivisible limits.
#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{
    Record,
    calls::ParameterKind,
    catalog::{CatalogDefault, CatalogMember, CatalogOption, CatalogOptionSubject},
    serving::*,
};
use lctx_postgres::generations::Error;
use support::*;
#[tokio::test]
async fn mandatory_packet_preserves_defaults_formals_contexts_and_set_hydration() {
    let fixture = ServingFixture::start(SOURCE).await;
    let execution = fixture.service.execution().await.unwrap();
    let r = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        sections: vec![],
        page: PageRequest::default(),
    };
    let response = fixture.catalog.operation(&execution, &r).await.unwrap();
    assert_eq!(response.generation.bytes(), fixture.generation.bytes());
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api did not resolve uniquely")
    };
    assert_eq!(packet.core.release.distribution.as_str(), "demo");
    assert_eq!(packet.core.release.version.as_str(), "1.0");
    assert!(!packet.core.signatures.is_empty());
    let signature = packet.core.signatures.iter().find(|s|s.role==lctx_model::domain::calls::SignatureRole::Source).expect("actual declared source signature");
    assert!(signature.complete);
    assert_eq!(signature.parameters.len(), 1);
    let source_parameter = &signature.parameters[0];
    let DefaultValue::Literal { literal } = source_parameter.default else {
        panic!(
            "declared False default was erased: {:?}",
            source_parameter.default
        )
    };
    assert!(
        packet.core.literal_values.iter().any(|row| {
            row.literal == literal && row.value == LiteralValue::Bool { value: false }
        })
    );
    assert_eq!(signature.effective_parameters.len(), 1);
    let parameter = &signature.effective_parameters[0];
    assert_eq!(parameter.name.0.as_ref().unwrap().as_str(), "flag");
    assert_eq!(parameter.kind, ParameterKind::PositionalOrKeyword);
    assert!(!parameter.formals.is_empty());
    let option_ids = packet
        .core
        .options
        .iter()
        .map(|o| o.option)
        .collect::<Vec<_>>();
    let slot_ids = parameter.slot.0.into_iter().collect::<Vec<_>>();
    let (options, subjects, defaults, slots) = execution
        .query(move |lease| {
            Box::pin(async move {
                let options = lease.read_ids::<CatalogOption>(&option_ids).await?;
                let subjects = lease
                    .read_ids::<CatalogOptionSubject>(
                        &options.rows().iter().map(|o| o.subject).collect::<Vec<_>>(),
                    )
                    .await?;
                let defaults = lease
                    .read_ids::<CatalogDefault>(
                        &options.rows().iter().map(|o| o.default).collect::<Vec<_>>(),
                    )
                    .await?;
                let slots = lease
                    .read_ids::<lctx_model::domain::normalized::callables::SignatureSlot>(&slot_ids)
                    .await?;
                Ok((options, subjects, defaults, slots))
            })
        })
        .await
        .unwrap();
    let expected=options.rows().iter().filter(|o|subjects.rows().iter().any(|s|s.id()==o.subject&&matches!(s,CatalogOptionSubject::Parameter{slot}if Some(*slot)==parameter.slot.0))).map(|o|DefaultValue::from_canonical(defaults.rows().iter().find(|d|d.id()==o.default).unwrap())).collect::<Vec<_>>();
    if !expected.is_empty() {
        assert!(
            expected.iter().all(|d| d == &parameter.default),
            "packet default differs from canonical catalog: {:?}",
            parameter.default
        );
    } else {
        assert_eq!(slots.rows().len(), 1);
        match slots.rows()[0].default {
            lctx_model::domain::normalized::callables::DefaultSlot::Required
            | lctx_model::domain::normalized::callables::DefaultSlot::Collector => {
                assert_eq!(parameter.default, DefaultValue::Absent {})
            }
            lctx_model::domain::normalized::callables::DefaultSlot::DefinitionTime
            | lctx_model::domain::normalized::callables::DefaultSlot::NativeUnknown => {
                assert_eq!(parameter.default, DefaultValue::Unknown {})
            }
        }
    }
    for option in &packet.core.options {
        let stored = options
            .rows()
            .iter()
            .find(|o| o.id() == option.option)
            .unwrap();
        let default = defaults
            .rows()
            .iter()
            .find(|d| d.id() == stored.default)
            .unwrap();
        assert_eq!(option.default, DefaultValue::from_canonical(default));
        if let CatalogDefault::Literal { literal } = default {
            assert!(
                packet
                    .core
                    .literal_values
                    .iter()
                    .any(|l| l.literal == *literal)
            );
        }
    }
    assert!(
        packet
            .core
            .invocations
            .iter()
            .any(|i| i.analysis == signature.analysis)
    );
    assert!(matches!(
        packet.scenarios.availability,
        Availability::NotRequested {}
    ));
    let members = fixture.members().await;
    let ids = members
        .iter()
        .filter(|m| ["demo.api", "demo.consume"].contains(&m.name.as_str()))
        .map(|m| m.member)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    let cores = fixture
        .catalog
        .operation_cores(&execution, &ids, false)
        .await
        .unwrap();
    assert_eq!(cores.iter().map(|c| c.member).collect::<Vec<_>>(), ids);
    let mut missing = r.clone();
    missing.operation = path("demo.absent");
    assert!(matches!(
        fixture
            .catalog
            .operation(&execution, &missing)
            .await
            .unwrap()
            .operation,
        OperationResolution::Missing { .. }
    ));
    let foreign = serde_json::from_value::<lctx_model::domain::Id<CatalogMember>>(
        serde_json::json!(vec![255u8; 16]),
    )
    .unwrap();
    assert!(
        fixture
            .catalog
            .operation_cores(&execution, &[foreign], false)
            .await
            .is_err()
    );
    drop(execution);
    fixture.finish().await;
}
#[tokio::test]
async fn complete_signature_over_default_budget_refuses_and_expanded_packet_is_complete() {
    let parameters = (0..70)
        .map(|i| format!("p{i}_{}: int", "x".repeat(350)))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("__all__ = ['huge']\ndef huge({parameters}):\n    return 0\n");
    let fixture = ServingFixture::start(source.as_bytes()).await;
    let execution = fixture.service.execution().await.unwrap();
    let mut request = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.huge"),
        sections: vec![],
        page: PageRequest::default(),
    };
    assert!(matches!(
        fixture.catalog.operation(&execution, &request).await,
        Err(Error::ResourceRefused(_))
    ));
    request.page.expanded = true;
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("huge did not resolve uniquely")
    };
    let signature=packet.core.signatures.iter().find(|s|s.role==lctx_model::domain::calls::SignatureRole::Source).unwrap();
    assert_eq!(signature.parameters.len(), 70);
    assert_eq!(signature.effective_parameters.len(), 70);
    assert!(packet.core.limits.signature_indivisible);
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn optional_sections_use_original_scenarios_and_policy_admissions_with_bound_continuations() {
    use lctx_model::domain::{
        normalized::events::{CallPolicy, CallPolicyAdmission, CallPolicyAssessment},
        *,
    };
    let targets = (0..7)
        .map(|i| format!("def t{i}(value: int) -> int:\n    return value\n"))
        .collect::<Vec<_>>()
        .join("\n");
    let callers = (0..4)
        .map(|i| format!("def caller{i}():\n    return api(True)\n"))
        .collect::<Vec<_>>()
        .join("\n");
    let names = std::iter::once("'api'".to_owned())
        .chain(std::iter::once("'relay'".to_owned()))
        .chain((0..7).map(|i| format!("'t{i}'")))
        .chain((0..4).map(|i| format!("'caller{i}'")))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "__all__ = [{names}]\ndef api(flag: bool = False) -> bool:\n    return flag\n{targets}\n{callers}\ndef relay() -> int:\n    return {}\n",
        (0..7)
            .map(|i| format!("t{i}({i})"))
            .collect::<Vec<_>>()
            .join(" + ")
    );
    let fixture = ServingFixture::start(source.as_bytes()).await;
    let execution = fixture.service.execution().await.unwrap();
    let sections = vec![
        OperationSection::Scenarios,
        OperationSection::Deployment,
        OperationSection::Relationships,
        OperationSection::Conflicts,
        OperationSection::Briefs,
        OperationSection::Behavior,
    ];
    let mut request = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.relay"),
        sections: sections.clone(),
        page: PageRequest {
            size: 100,
            ..Default::default()
        },
    };
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("relay did not resolve")
    };
    for deployment in &packet.deployment.items {
        let id = deployment.deployment;
        let row = execution
            .query(move |lease| {
                Box::pin(async move {
                    let catalog = lease
                        .read_ids::<lctx_model::domain::catalog::evidence::CatalogDeployment>(&[id])
                        .await?;
                    lease
                        .read_ids::<lctx_model::domain::deployment::DeploymentObservation>(&[
                            catalog.rows()[0].observation,
                        ])
                        .await
                })
            })
            .await
            .unwrap();
        assert_eq!(deployment.field.as_str(), row.rows()[0].field);
        assert_eq!(deployment.value.as_str(), row.rows()[0].original);
        assert!(!deployment.originals.is_empty());
    }
    assert_eq!(packet.relationships.items.len(), 5);
    assert!(packet.relationships.truncated);
    assert_eq!(packet.relationships.omitted, 2);
    let token = packet.relationships.continuation.0.clone().unwrap();
    for relationship in &packet.relationships.items {
        assert_eq!(
            relationship.role,
            lctx_model::domain::selection::RelationRole::Invokes
        );
        assert!(!relationship.proof.is_empty());
        assert!(relationship.witnesses.is_empty());
        let reference = relationship
            .proof
            .iter()
            .find(|p| p.relation.as_str() == CallPolicyAdmission::NAME)
            .unwrap();
        let admission_id: Id<CallPolicyAdmission> =
            serde_json::from_value(serde_json::json!(reference.row)).unwrap();
        let (admission, policy) = execution
            .query(move |lease| {
                Box::pin(async move {
                    let admission = lease
                        .read_ids::<CallPolicyAdmission>(&[admission_id])
                        .await?;
                    let policy = lease
                        .read_ids::<CallPolicyAssessment>(&[admission.rows()[0].assessment])
                        .await?;
                    Ok((admission, policy))
                })
            })
            .await
            .unwrap();
        assert_eq!(admission.rows().len(), 1);
        assert_eq!(policy.rows()[0].policy, CallPolicy::Invocation);
    }
    request.page.cursor = Optional(Some(token.clone()));
    request.page.size = 1;
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet: next } = response.operation else {
        panic!("relay missing")
    };
    assert_eq!(next.relationships.items.len(), 1);
    assert_eq!(next.relationships.omitted, 1);
    assert!(next.relationships.continuation.0.is_some());
    let mut foreign = request.clone();
    foreign.operation = path("demo.api");
    assert!(
        fixture
            .catalog
            .operation(&execution, &foreign)
            .await
            .is_err()
    );
    foreign = request.clone();
    foreign.sections = vec![OperationSection::Scenarios];
    assert!(
        fixture
            .catalog
            .operation(&execution, &foreign)
            .await
            .is_err()
    );
    let scenarios = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        sections: vec![OperationSection::Scenarios],
        page: PageRequest {
            size: 100,
            ..Default::default()
        },
    };
    let response = fixture
        .catalog
        .operation(&execution, &scenarios)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api missing")
    };
    assert_eq!(packet.scenarios.items.len(), 2);
    assert!(packet.scenarios.truncated);
    assert!(packet.scenarios.omitted >= 2);
    assert!(matches!(
        packet.relationships.availability,
        Availability::NotRequested {}
    ));
    for scenario in packet.scenarios.items {
        assert_eq!(
            scenario.basis,
            lctx_model::domain::catalog::evidence::AssociationBasis::ResolvedTarget
        );
        assert!(matches!(
            scenario.intent,
            lctx_model::domain::catalog::evidence::Intent::Demonstration
                | lctx_model::domain::catalog::evidence::Intent::AssertionTest
        ));
        assert!(!scenario.spans.is_empty());
        for original in scenario.spans {
            let replay = fixture
                .service
                .original_range(&execution, original.source.clone(), Some(original.context))
                .await
                .unwrap();
            assert_eq!(replay, original);
        }
    }
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn behavioral_packet_preserves_the_stored_five_verdict_condition_and_model_question() {
    use lctx_model::domain::{
        execution::summary_consequences::ClaimConclusion, synthesis::summary::SummaryFacet, *,
    };
    let fixture = ServingFixture::start_profile(SOURCE, "behavioral").await;
    let execution = fixture.service.execution().await.unwrap();
    let request = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        sections: vec![OperationSection::Behavior],
        page: PageRequest {
            size: 100,
            ..Default::default()
        },
    };
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api missing")
    };
    for behavior in &packet.behavior.items {
        let facet_reference = behavior
            .proof
            .iter()
            .find(|r| r.relation.as_str() == SummaryFacet::NAME)
            .unwrap();
        let facet_id: Id<SummaryFacet> =
            serde_json::from_value(serde_json::json!(facet_reference.row)).unwrap();
        let (facet, conclusion, qualification, parameters) = execution
            .query(move |lease| {
                Box::pin(async move {
                    let facet = lease.read_ids::<SummaryFacet>(&[facet_id]).await?;
                    let conclusion = lease
                        .read_ids::<ClaimConclusion>(&[facet.rows()[0].conclusion])
                        .await?;
                    let qualification = lease
                        .read_ids::<assertion::AssertionQualification>(&[conclusion.rows()[0]
                            .qualification
                            .unwrap()])
                        .await?;
                    let invocation = lease
                        .read_ids::<analysis::summary::AnalysisInvocation>(&[
                            conclusion.rows()[0].invocation
                        ])
                        .await?;
                    let definition = lease
                        .read_ids::<analysis::AnalysisDefinition>(
                            &[invocation.rows()[0].definition],
                        )
                        .await?;
                    let parameters = lease
                        .read_ids::<analysis::MethodParameters>(&[definition.rows()[0].parameters])
                        .await?;
                    Ok((facet, conclusion, qualification, parameters))
                })
            })
            .await
            .unwrap();
        assert_eq!(behavior.verdict, facet.rows()[0].verdict);
        assert_eq!(behavior.verdict, conclusion.rows()[0].verdict);
        assert_eq!(behavior.condition, qualification.rows()[0].condition);
        assert_eq!(Some(behavior.model), parameters.rows()[0].model_catalog);
        assert_eq!(
            behavior.presentation_truncated,
            behavior.presentation.truncated
        );
        assert!(behavior.presentation.terms.len() <= 16);
    }
    // Qualification absence stays a visible boundary rather than acquiring a made-up condition.
    let member = packet.core.member;
    let facets = execution
        .query(move |lease| {
            Box::pin(async move {
                let members = lease
                    .read_for::<catalog::CatalogMemberInvocation, catalog::CatalogMember>(
                        "member",
                        &[member],
                    )
                    .await?;
                let ids = members.rows().iter().map(Record::id).collect::<Vec<_>>();
                lease
                    .read_for::<SummaryFacet, catalog::CatalogMemberInvocation>("member", &ids)
                    .await
            })
        })
        .await
        .unwrap();
    let mut canonical = facets.rows().iter().collect::<Vec<_>>();
    canonical.sort_by_key(|f| f.id());
    let qualified = canonical
        .iter()
        .filter(|f| f.qualification.is_some())
        .count();
    assert_eq!(packet.behavior.items.len(), qualified.min(100));
    if facets.rows().iter().any(|f| f.qualification.is_none()) {
        assert!(matches!(
            packet.behavior.availability,
            Availability::Partial { .. }
        ));
        assert!(packet.behavior.omitted > 0);
    }
    assert!(matches!(
        packet.scenarios.availability,
        Availability::NotRequested {}
    ));
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn large_optional_brief_keeps_the_complete_core_and_resumes_with_expanded_budget() {
    let prose = format!(
        "Return the supplied value. {}",
        "Authored context. ".repeat(2200)
    );
    let source = format!(
        "__all__ = ['api']\ndef api(flag: bool = False) -> bool:\n    \"\"\"{prose}\"\"\"\n    return flag\n"
    );
    let fixture =
        ServingFixture::start_with_seeds(source.as_bytes(), "catalog", &["demo.api"], 1).await;
    let execution = fixture.service.execution().await.unwrap();
    let mut request = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        sections: vec![OperationSection::Briefs],
        page: PageRequest::default(),
    };
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api missing")
    };
    let signature=packet.core.signatures.iter().find(|s|s.role==lctx_model::domain::calls::SignatureRole::Source).unwrap();
    assert_eq!(signature.parameters.len(), 1);
    assert!(signature.complete);
    assert!(packet.briefs.items.is_empty());
    assert!(matches!(
        packet.briefs.availability,
        Availability::Partial { .. }
    ));
    assert!(packet.briefs.truncated);
    assert_eq!(packet.briefs.omitted, 1);
    let token = packet.briefs.continuation.0.unwrap();
    request.page.expanded = true;
    request.page.cursor = Optional(Some(token));
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api missing")
    };
    assert_eq!(packet.briefs.items.len(), 1);
    let capability = &packet.briefs.items[0];
    assert!(capability.rendered.as_str().contains(&prose));
    assert!(capability.unreviewed);
    assert!(capability.documentation_only);
    assert!(!capability.originals.is_empty());
    let standalone = fixture
        .service
        .capability(
            &execution,
            &GetCapabilityRequest {
                capability: capability.capability,
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(standalone.capability, *capability);
    use lctx_model::domain::{
        analysis::policy::EvidenceStatus, synthesis::assertions::ProgrammaticAssertion,
    };
    assert!(!capability.assertions.is_empty());
    let statuses = capability
        .assertions
        .iter()
        .map(|a| a.status)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(statuses.contains(&EvidenceStatus::Documented));
    assert!(statuses.contains(&EvidenceStatus::StructurallyObserved));
    let ids = capability
        .assertions
        .iter()
        .map(|a| a.assertion)
        .collect::<Vec<_>>();
    let originals = execution
        .query(move |lease| {
            Box::pin(async move { lease.read_ids::<ProgrammaticAssertion>(&ids).await })
        })
        .await
        .unwrap();
    for claim in &capability.assertions {
        let original = originals
            .rows()
            .iter()
            .find(|a| a.id() == claim.assertion)
            .unwrap();
        assert_eq!(claim.status, original.status());
        assert_eq!(claim.kind, original.kind());
        assert_eq!(claim.section, original.section());
        assert_eq!(claim.qualification, original.qualification());
        assert_eq!(claim.text.as_str(), original.text());
        assert!(!claim.supports.is_empty());
        for support in &claim.supports {
            assert_eq!(support.proof.len(), 3);
        }
    }
    let resource = capability.resource_text().unwrap();
    assert!(resource.starts_with(capability.rendered.as_str()));
    for claim in &capability.assertions {
        assert!(resource.contains(&format!("\"status_name\":\"{:?}\"", claim.status)));
        assert!(resource.contains(&serde_json::to_string(&claim.assertion).unwrap()));
    }
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn typed_packets_keep_wrapper_roles_generated_slots_and_native_proofs(){
 use lctx_model::domain::{calls::SignatureRole,assumptions::AssumptionSet};
 let source=br#"from typing import Callable
from dataclasses import dataclass
__all__=['changed','Settings']
def wrap(fn: Callable[[int],int]):
    def implementation(text: str) -> bytes:
        return text.encode()
    return implementation
@wrap
def changed(value: int) -> int:
    return value
@dataclass
class Settings:
    host: str
    port: int = 80
"#;
 let fixture=ServingFixture::start(source).await;let execution=fixture.service.execution().await.unwrap();
 let response=fixture.catalog.operation(&execution,&GetOperationRequest{library:Name::new("demo").unwrap(),operation:path("demo.changed"),sections:vec![],page:PageRequest{expanded:true,..Default::default()}}).await.unwrap();
 let OperationResolution::Unique{packet}=response.operation else{panic!("wrapped operation missing");};
 let declared=packet.core.signatures.iter().find(|s|s.role==SignatureRole::Source).unwrap();
 let effective=packet.core.signatures.iter().find(|s|s.role==SignatureRole::EffectiveTyped).unwrap();
 assert!(declared.parameters.iter().any(|p|p.name.0.as_ref().is_some_and(|n|n.as_str()=="value")));
 assert!(effective.effective_parameters.iter().any(|p|p.name.0.as_ref().is_some_and(|n|n.as_str()=="text")&&!p.types.is_empty()&&p.formals.is_empty()),"native text slot retains typing without invented source formal");
 assert_ne!(declared.return_types,effective.return_types,"declared int is not native effective bytes");
 assert!(declared.native.0.is_none());assert!(effective.native.0.is_some());assert!(effective.complete);
 for signature in &packet.core.signatures{
   for answer in &signature.typing{
     assert_eq!(answer.claim_basis.set,AssumptionSet::empty().id());
     assert!(answer.proof.iter().any(|p|p.relation.as_str().ends_with("supports")));
     match answer.origin {SignatureTypingOrigin::SourceDeclared{..}=>assert_eq!(signature.role,SignatureRole::Source),SignatureTypingOrigin::NativeObserved{..}=>assert_ne!(signature.role,SignatureRole::Source)}
   }
   assert!(signature.parameters.iter().chain(signature.effective_parameters.iter()).flat_map(|p|p.type_evidence.iter()).chain(signature.return_evidence.iter()).all(|proof|signature.typing.iter().any(|answer|answer.proof.contains(proof))),"every exposed type points to its actual qualified proof");
 }
 // Native/source facet witnesses returned by discovery must exist in this generation.
 for value in [lctx_model::domain::selection::FacetValue::Async{asynchronous:false},lctx_model::domain::selection::FacetValue::DecoratorQualifiedName{module:"demo".into(),path:vec!["wrap".into()]}] {
   use lctx_model::domain::selection::*;
   let facet=if matches!(value,FacetValue::Async{..}){Facet::Async}else{Facet::Decorator};
   let response=fixture.catalog.find(&execution,&FindOperationsRequest{library:Name::new("demo").unwrap(),selection:SelectionInput(Selection{requirements:vec![Requirement{predicate:Predicate::FacetMembership{facet,value},quantifier:Quantifier::AnyApplicable}],mode:Mode::Discovery,joint:JointPolicy::IndependentRecords}),page:PageRequest{size:100,..Default::default()}}).await.unwrap();
   let result=response.supported.items.iter().find(|r|r.name.as_str()=="demo.changed").expect("actual source facet supports changed");
   let requirement=&result.requirements[0];
   assert!(!requirement.positive.is_empty());
   for id in requirement.positive.iter().chain(&requirement.negative).chain(&requirement.closure){
     let present:bool=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {}.selection_witnesses WHERE id=$1)",fixture.generation.schema()))).bind(id.bytes().to_vec()).fetch_one(fixture.db.owner.pool()).await.unwrap();
     assert!(present,"request-time witness has no persisted row: {id:?}");
   }
 }
 // Independently specified source/effective return questions retain distinct answers through service preparation.
 {
  use lctx_model::domain::selection::*;
  let returns=|role,name:&str|Predicate::VariantReturnType{role,r#type:StructuralType::NominalIdentity{module:"builtins".into(),name:name.into()}};
  for (predicate,expected) in [
   (returns(SignatureRole::Source,"int"),Outcome::Supported),
   (returns(SignatureRole::Source,"bytes"),Outcome::Contradicted),
   (returns(SignatureRole::EffectiveTyped,"bytes"),Outcome::Supported),
   (returns(SignatureRole::EffectiveTyped,"int"),Outcome::Contradicted),
   (Predicate::FacetMembership{facet:Facet::Async,value:FacetValue::Async{asynchronous:true}},Outcome::Contradicted),
   (Predicate::FacetMembership{facet:Facet::ClassMetadata,value:FacetValue::ClassMetadata{trait_kind:ClassFacet::Enumeration,present:true}},Outcome::Unresolved),
  ] {
    let result=fixture.catalog.compare(&execution,&CompareOperationsRequest{library:Name::new("demo").unwrap(),operations:vec![path("demo.changed")],selection:SelectionInput(Selection{requirements:vec![Requirement{predicate:predicate.clone(),quantifier:Quantifier::AnyApplicable}],mode:Mode::Discovery,joint:JointPolicy::IndependentRecords}),page:PageRequest{expanded:true,..Default::default()}}).await.unwrap();
    assert!(!result.operations[0].candidates.is_empty());
    assert!(result.operations[0].candidates.iter().all(|r|r.requirements[0].outcome==expected),"role/facet service answer differs for {predicate:?}: {:?}",result.operations[0].candidates);
  }
 }
 let encoded=serde_json::to_value(&packet).unwrap();assert!(encoded["core"]["signatures"][0].get("source").is_none(),"hard wire migration replaces misleading source-only field");
 let response=fixture.catalog.operation(&execution,&GetOperationRequest{library:Name::new("demo").unwrap(),operation:path("demo.Settings"),sections:vec![],page:PageRequest{expanded:true,..Default::default()}}).await.unwrap();
 let OperationResolution::Unique{packet}=response.operation else{panic!("generated constructor missing");};
 let generated=packet.core.signatures.iter().find(|s|s.role==SignatureRole::Synthesized&&s.effective_parameters.iter().any(|p|p.name.0.as_ref().is_some_and(|n|n.as_str()=="host"))).unwrap();
 assert!(generated.effective_parameters.iter().any(|p|p.name.0.as_ref().is_some_and(|n|n.as_str()=="host")&&!p.types.is_empty()&&p.formals.is_empty()));
 assert!(generated.typing.iter().any(|t|matches!(t.origin,SignatureTypingOrigin::NativeObserved{..})));
 drop(execution);fixture.finish().await;
}
