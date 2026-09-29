//! Actual evidence producer output drives selection; unrelated release evidence cannot leak.
use cpg_schema::{Id, evidence::*, selection::catalog::*, wire::*};
pub fn check(
    facts: &cpg_core::catalog::CatalogFacts,
    evidence: &cpg_core::evidence::EvidenceInputs,
    catalog: &cpg_core::catalog::Contracts,
    context: &cpg_core::evidence::EvidenceRows,
    snapshot: Id,
) {
    let mut catalog = catalog.clone();
    catalog.contextual = context.clone();
    let input = Inputs {
        members: catalog.members.clone(),
        domains: cpg_core::catalog_domains::derive(facts, evidence, &catalog, snapshot)
            .unwrap()
            .into_iter()
            .map(DomainInput::try_from)
            .collect::<Result<_, _>>()
            .unwrap(),
        bindings: catalog.bindings.clone(),
        signatures: catalog.signatures.clone(),
        parameters: catalog.parameters.clone(),
        types: catalog.types.clone(),
        type_args: catalog.type_args.clone(),
        type_observations: catalog.type_observations.clone(),
        surfaces: catalog.surfaces.clone(),
        configurations: catalog.configurations.clone(),
        field_links: catalog.field_links.clone(),
        scenarios: context
            .scenarios
            .iter()
            .map(|r| ScenarioInput {
                scenario_id: r.scenario_id,
                detail: r.detail.clone(),
            })
            .collect(),
        deployments: context
            .deployments
            .iter()
            .map(|r| DeploymentInput {
                deployment_id: r.deployment_id,
                span_id: r.span_id,
                detail: r.detail.clone(),
            })
            .collect(),
        associations: context
            .associations
            .iter()
            .map(|r| AssociationInput {
                association_id: r.association_id,
                member_id: r.member_id,
                release_id: r.release_id,
                evidence_id: r.evidence_id,
                evidence_kind: serde_json::from_value(r.evidence_kind.clone().into()).unwrap(),
                role: r.role.clone(),
                basis: r.basis.clone(),
                support: r.support.clone(),
            })
            .collect(),
        artifacts: context
            .artifacts
            .iter()
            .map(|r| ArtifactInput {
                artifact_id: r.artifact_id,
                release_id: r.release_id,
                alignment: serde_json::from_value(r.alignment.clone().into()).unwrap(),
            })
            .collect(),
        spans: context
            .spans
            .iter()
            .map(|r| SpanInput {
                span_id: r.span_id,
                artifact_id: r.artifact_id,
            })
            .collect(),
        ..Default::default()
    };
    let prepared = PreparedCatalog::new(&input).unwrap();
    let select = |path: &str, requirements: serde_json::Value| {
        let selection: Selection =
            serde_json::from_value(serde_json::json!({"requirements":requirements})).unwrap();
        prepared
            .classify(&selection)
            .unwrap()
            .into_iter()
            .find(|c| c.access_path == path)
            .unwrap()
    };
    for path in ["pr3pkg.run", "pr3pkg.unseeded"] {
        assert_eq!(select(path,serde_json::json!([{"predicate":"deployment_declaration","field":"requires_dist","name":"dependency"}])).outcome,SelectionOutcome::Supported);
        assert_eq!(select(path,serde_json::json!([{"predicate":"release_version","distribution":"pr3pkg","version":"1"}])).outcome,SelectionOutcome::Supported);
        assert_eq!(select(path,serde_json::json!([{"predicate":"release_version","distribution":"foreign","version":"1"}])).outcome,SelectionOutcome::Unresolved);
    }
    assert_eq!(
        select(
            "pr3pkg.run",
            serde_json::json!([{"predicate":"scenario_intent","intent":"mixed"}])
        )
        .outcome,
        SelectionOutcome::Supported
    );
    assert_eq!(
        select(
            "pr3pkg.unseeded",
            serde_json::json!([{"predicate":"scenario_check","check":"parse","status":"passed"}])
        )
        .outcome,
        SelectionOutcome::Supported
    );
    assert_ne!(select("pr3pkg.run",serde_json::json!([{"predicate":"scenario_check","check":"execution","status":"passed"}])).outcome,SelectionOutcome::Supported);
    let a = context
        .associations
        .iter()
        .find(|a| {
            a.member_id
                == input
                    .members
                    .iter()
                    .find(|m| m.access_path == "pr3pkg.run")
                    .map(|m| m.member_id)
                && a.basis == "resolved_target"
                && !serde_json::from_str::<Vec<AssociationSupport>>(&a.support)
                    .unwrap()
                    .is_empty()
        })
        .unwrap();
    let support = serde_json::from_str::<Vec<AssociationSupport>>(&a.support)
        .unwrap()
        .remove(0);
    let result = select(
        "pr3pkg.run",
        serde_json::json!([{"predicate":"relationship","role":a.role,"target":{"kind":"declaration","node":support.target_id},"fidelity":"resolved_target"}]),
    );
    assert_eq!(result.outcome, SelectionOutcome::Supported);
    assert!(result.requirements[0].witnesses.iter().any(|w|matches!(w,RequirementWitness::Positive{evidence,..} if evidence.iter().any(|e|matches!(e,WitnessEvidence::Association{edge:Some(edge),..} if *edge==support.edge_id)))));
    let mut altered = input.clone();
    let association = altered
        .associations
        .iter()
        .find(|a| a.evidence_kind == EvidenceKind::Scenario && a.member_id.is_some())
        .unwrap();
    let affected = association.member_id.unwrap();
    let scenario: ScenarioDetail = serde_json::from_str(
        &altered
            .scenarios
            .iter()
            .find(|s| s.scenario_id == association.evidence_id)
            .unwrap()
            .detail,
    )
    .unwrap();
    let artifact = altered
        .spans
        .iter()
        .find(|s| s.span_id == scenario.spans[0])
        .unwrap()
        .artifact_id;
    altered
        .artifacts
        .iter_mut()
        .find(|a| a.artifact_id == artifact)
        .unwrap()
        .alignment = Alignment::Other;
    let rows=PreparedCatalog::new(&altered).unwrap().classify(&serde_json::from_value(serde_json::json!({"requirements":[{"predicate":"source_alignment","alignment":"other"}]})).unwrap()).unwrap();
    assert_eq!(
        rows.iter()
            .find(|r| r.member_id.storage() == affected)
            .unwrap()
            .outcome,
        SelectionOutcome::Supported,
        "scenario-only association owns its original source alignment"
    );
    let mut missing = input.clone();
    let fake = Id([240; 16]);
    missing.artifacts.push(ArtifactInput {
        artifact_id: fake,
        release_id: fake,
        alignment: Alignment::Other,
    });
    let page=PreparedCatalog::new(&missing).unwrap().classify(&serde_json::from_value(serde_json::json!({"requirements":[{"predicate":"source_alignment","alignment":"other"}]})).unwrap()).unwrap();
    assert!(
        page.iter()
            .all(|c| c.outcome != SelectionOutcome::Supported),
        "unrelated artifacts cannot establish member alignment"
    );
    for deployment in &mut missing.deployments {
        let mut d: DeploymentDetail = serde_json::from_str(&deployment.detail).unwrap();
        d.interpretation = CheckStatus::Failed;
        deployment.detail = serde_json::to_string(&d).unwrap();
    }
    let page=PreparedCatalog::new(&missing).unwrap().classify(&serde_json::from_value(serde_json::json!({"requirements":[{"predicate":"deployment_declaration","field":"requires_dist","name":"dependency"}]})).unwrap()).unwrap();
    assert!(
        page.iter()
            .all(|c| c.outcome != SelectionOutcome::Supported),
        "failed interpretation is not a package declaration"
    );
}
