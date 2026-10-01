// DORMANT P5: original bytes, bounded evidence pages and typed references over a supplied pin.
// P4 original contexts/nonleakage moved to typed catalog_evidence/domain_catalog_evidence controls.
// No Delta compilation, legacy importer, role administration or copied generation runtime returns.
use cpg_schema::{Id, evidence::*};
use lctx_postgres::{repository::PinnedGeneration, serving::ServingStore};
use std::path::Path;

pub fn write_large_original_evidence_fixture(corpus: &Path) {
    // A long Unicode paragraph exercises continuation through original UTF-8 boundaries.
    std::fs::write(
        corpus.join("docs/large.mdx"),
        format!("# Long\n\n{}", "λ🦀\n".repeat(5000)),
    )
    .unwrap();
    std::fs::write(
        corpus.join("docs/large-launch.mdx"),
        format!("# Launch\n\n```sh\n{}\n```\n", "echo λ\n".repeat(25000)),
    )
    .unwrap();
    std::fs::write(
        corpus.join("examples/large.py"),
        format!("from pr3pkg import run\nrun(\"{}\")\n", "λ".repeat(70000)),
    )
    .unwrap();
}

pub async fn postgres_original_bytes_bounded_pages_and_typed_refs(
    serving: &ServingStore, pinned: &PinnedGeneration, snapshot: Id,
    artifacts: &[CatalogArtifactsRow], spans: &[CatalogSpansRow],
    scenarios: &[CatalogScenariosRow], deployments: &[CatalogDeploymentsRow],
) {
    let artifact = artifacts
        .iter()
        .find(|a| a.path == "docs/large.mdx")
        .unwrap();
    let span = spans
        .iter()
        .filter(|s| s.artifact_id == artifact.artifact_id)
        .max_by_key(|s| s.end_byte - s.start_byte)
        .unwrap();
    let reference = EvidenceRef::new(EvidenceKind::Span, span.span_id);
    let mut cursor = None;
    let mut collected = String::new();
    let mut pages = 0;
    loop {
        let page = serving
            .get_evidence(
                pinned,
                &snapshot.hex(),
                reference.clone(),
                cursor.as_deref(),
                false,
            )
            .await
            .unwrap();
        let encoded =
            cpg_schema::wire::tool_result("get_evidence", &page.to_string(), false).unwrap();
        assert!(encoded.len() <= 32768);
        let text = page["content"][0]["text"].as_str().unwrap();
        collected.push_str(text);
        pages += 1;
        cursor = page["next_cursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
        assert!(
            serving
                .get_evidence(
                    pinned,
                    &snapshot.hex(),
                    EvidenceRef::new(EvidenceKind::Scenario, span.span_id),
                    cursor.as_deref(),
                    false
                )
                .await
                .is_err()
        );
        assert!(
            serving
                .get_evidence(
                    pinned,
                    &snapshot.hex(),
                    reference.clone(),
                    cursor.as_deref(),
                    true
                )
                .await
                .is_err()
        );
    }
    assert!(pages > 1);
    assert_eq!(
        collected.as_bytes(),
        &artifact.body.0[span.start_byte as usize..span.end_byte as usize]
    );
    let packet = serving
        .operation_packet(
            pinned,
            &serde_json::from_value(serde_json::json!({
                "snapshot_id":snapshot.hex(),"operation":"pr3pkg.run",
                "view":{"kind":"section","section":"evidence"}
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    cpg_schema::wire::tool_result("get_operation", &packet.to_string(), false).unwrap();
    assert!(!packet["items"].as_array().unwrap().is_empty());
    let reference: EvidenceRef = serde_json::from_value(
        packet["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["record"]["evidence"]["kind"] == "scenario")
            .unwrap()["record"]["evidence"]
            .clone(),
    )
    .unwrap();
    let page = serving
        .get_evidence(pinned, &snapshot.hex(), reference, None, false)
        .await
        .unwrap();
    assert_eq!(page["scenario"]["checks"]["execution"], "not_run");
    for path in ["examples/large.py", "docs/large-launch.mdx"] {
        let artifact = artifacts.iter().find(|a| a.path == path).unwrap();
        let span_ids: std::collections::BTreeSet<_> = spans
            .iter()
            .filter(|s| s.artifact_id == artifact.artifact_id)
            .map(|s| s.span_id)
            .collect();
        let reference = if path.ends_with(".py") {
            let scenario = scenarios.iter()
                .find(|s| span_ids.contains(&s.primary_span_id))
                .unwrap();
            EvidenceRef::new(EvidenceKind::Scenario, scenario.scenario_id)
        } else {
            let deployment = deployments.iter()
                .find(|d| span_ids.contains(&d.span_id))
                .unwrap();
            EvidenceRef::new(EvidenceKind::Deployment, deployment.deployment_id)
        };
        for expanded in [false, true] {
            let page = serving
                .get_evidence(pinned, &snapshot.hex(), reference.clone(), None, expanded)
                .await
                .unwrap();
            assert_eq!(page["metadata_omitted"], true);
            assert!(page["content"][0]["text"].is_string());
            assert!(page["next_cursor"].is_string());
            let encoded =
                cpg_schema::wire::tool_result("get_evidence", &page.to_string(), expanded).unwrap();
            assert!(encoded.len() <= if expanded { 262144 } else { 32768 });
        }
    }
}
