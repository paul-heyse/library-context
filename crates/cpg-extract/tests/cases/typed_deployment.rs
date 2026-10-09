//! Report parsing and independent source/environment association through real fact producers.
use crate::typed_driver;
use crate::inspector;
use cpg_extract::{acquisition::*, bundle::CapturedInputs, capture::CapturedInput};
use lctx_model::domain::{attribution::*, deployment::*, input::*, *};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, sync::Arc};
use typed_driver::rows;
inspector!(
    Deployment,
    DeploymentObservation,
    TaskReportObservation,
    TaskReport,
    ReportedEnvironment,
    ReportValue,
    ReportEntry,
    ProviderCoverage,
    DerivedArtifact
);
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn captured(mutation: &str) -> Arc<CapturedInputs> {
    captured_with_budget(mutation, &typed_driver::budget())
}
fn captured_with_budget(
    mutation: &str,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Arc<CapturedInputs> {
    let site = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(site.path().join("fastmcp")).unwrap();
    let module = b"def add(a, b): return a + b\n";
    std::fs::write(site.path().join("fastmcp/__init__.py"), module).unwrap();
    std::fs::create_dir_all(site.path().join("fastmcp-4.0.5.dist-info")).unwrap();
    let metadata=b"Metadata-Version: 2.5\nName: fastmcp\nVersion: 4.0.5\nRequires-Dist: dep[server]>=1; extra == 'x'\n";
    std::fs::write(
        site.path().join("fastmcp-4.0.5.dist-info/METADATA"),
        metadata,
    )
    .unwrap();
    std::fs::write(
        site.path().join("fastmcp-4.0.5.dist-info/entry_points.txt"),
        b"[console_scripts]\nfastmcp = fastmcp.cli:main\n",
    )
    .unwrap();
    let paths = vec![
        "fastmcp/__init__.py".into(),
        "fastmcp-4.0.5.dist-info/METADATA".into(),
        "fastmcp-4.0.5.dist-info/entry_points.txt".into(),
    ];
    let frozen = CapturedInput::capture(site.path(), &paths, budget).unwrap();
    let inventory = LibraryInventory {
        name: "fastmcp".into(),
        requirement: "fastmcp==4.0.5".into(),
        lock_digest: ContentHash::of(b"fixture-lock"),
        installer: Some("uv-fixture".into()),
        python_version: "3.14.7".into(),
        platform: "linux".into(),
        site_packages: site.path().into(),
        distributions: vec![InventoryDistribution {
            name: "fastmcp".into(),
            version: "4.0.5".into(),
            first_party: true,
            artifact_sha256: vec![],
            record_digest: ContentHash::of(b"record"),
        }],
        files: paths
            .into_iter()
            .map(|path| InventoryFile {
                role: if path.ends_with(".py") {
                    SourceRole::Release
                } else {
                    SourceRole::DistributionMetadata
                },
                path,
                owners: vec!["fastmcp".into()],
                record_sha256: None,
            })
            .collect(),
        configuration: ContentHash::of(b"fixture"),
    };
    let library = AcquiredInput::new(frozen, Acquisition::Installed(inventory));
    let e = cpg_extract::deployment::identity(&library).unwrap();
    let corpus = tempfile::tempdir().unwrap();
    let path = "examples/fastmcp_config/server.py";
    std::fs::create_dir_all(corpus.path().join("examples/fastmcp_config")).unwrap();
    let source = include_bytes!("../../../../fixtures/python/semantic_deployment/example.py");
    std::fs::write(corpus.path().join(path), source).unwrap();
    std::fs::write(
        corpus.path().join("launch.json"),
        br#"{"source":"examples/fastmcp_config/server.py:mcp","env":{"TOKEN":"${TOKEN}"}}"#,
    )
    .unwrap();
    let mut receipt = serde_json::json!({"format":1,"policy":"fastmcp-stdio-v1","task":"programmatic","runner_sha256":sha(include_bytes!("../../../../scripts/deployment_check.py")),"source_path":path,"source_sha256":sha(source),"environment":{"release_id":e.release_id,"lock_digest":e.lock_digest,"environment_digest":e.environment_digest,"runtime_digest":ContentHash::of(b"reported runtime"),"interpreter_digest":ContentHash::of(b"reported interpreter"),"python_version":e.python_version,"platform":e.platform,"requirement":e.requirement,"metadata":e.metadata},"command":["<environment>/bin/python","<source>/server.py"],"tool":"add","arguments":{"a":2,"b":3},"elapsed_ms":4,"timeout_seconds":45,"execution":"passed","tools":["add","add"],"result":"5","diagnostic":null});
    if mutation == "source" {
        receipt["source_sha256"] = serde_json::json!("0".repeat(64));
    }
    if mutation == "environment" {
        receipt["environment"]["python_version"] = serde_json::json!("3.13.0");
    }
    let supplied = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        supplied.path(),
        match mutation {
            "malformed" => b"{".to_vec(),
            "oversized" => vec![b'x'; 65537],
            _ => serde_json::to_vec(&receipt).unwrap(),
        },
    )
    .unwrap();
    let frozen =
        CapturedInput::capture(corpus.path(), &[path.into(), "launch.json".into()], budget)
            .unwrap()
            .with_receipts(&[supplied.path().into()])
            .unwrap();
    let input = AcquiredInput::new(
        frozen,
        Acquisition::Corpus {
            repository: "fixture://deployment".into(),
            commit: "fixture".into(),
            uses: BTreeMap::from([
                (path.into(), vec![SourceRole::Example]),
                ("launch.json".into(), vec![SourceRole::Configuration]),
            ]),
            library: 0,
        },
    );
    Arc::new(CapturedInputs::new(
        vec![library, input],
        cpg_extract::native_context::NativeContextConfig::committed(
            lctx_model::domain::stages::Profile::Catalog,
            budget,
        )
        .unwrap(),
    ))
}
#[tokio::test]
async fn valid_report_keeps_every_field_and_duplicate_ordered_values_without_execution() {
    let tables = typed_driver::Tables::default();
    typed_driver::run_with(captured(""),cpg_extract::pyrefly_stage::Pyrefly::new(cpg_extract::typed_syntax::SyntaxLimits::default()),Deployment(tables.clone())).await.unwrap();
    let reports = rows::<TaskReport>(&tables);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].elapsed_ms, Milliseconds(4));
    assert_eq!(reports[0].execution, CheckStatus::Passed);
    assert_eq!(rows::<TaskReportObservation>(&tables).len(), 1);
    assert_eq!(
        rows::<ReportValue>(&tables)
            .iter()
            .filter(|v| matches!(v,ReportValue::Tool {name,..} if name=="add"))
            .count(),
        2
    );
    assert!(
        rows::<DerivedArtifact>(&tables)
            .iter()
            .any(|d| matches!(d, DerivedArtifact::TaskReceipt { .. }))
    );
    let descriptions = rows::<DeploymentObservation>(&tables);
    let requirement = descriptions
        .iter()
        .find(|d| d.field == "requires-dist")
        .unwrap();
    assert_eq!(requirement.extras, vec!["server"]);
    assert!(requirement.marker.as_ref().unwrap().contains("extra"));
    assert!(descriptions.iter().any(|d| d.original.contains("${TOKEN}")));
}
#[tokio::test]
async fn malformed_oversized_and_mismatched_reports_remain_captured_failed_interpretations() {
    for mutation in ["malformed", "oversized", "source", "environment"] {
        let tables = typed_driver::Tables::default();
        typed_driver::run_with(
            captured(mutation),
            cpg_extract::pyrefly_stage::Pyrefly::new(
                cpg_extract::typed_syntax::SyntaxLimits::default(),
            ),
            Deployment(tables.clone()),
        )
        .await
        .unwrap();
        assert!(rows::<TaskReport>(&tables).is_empty(), "{mutation}");
        assert!(
            rows::<DeploymentObservation>(&tables)
                .iter()
                .any(|d| d.field == "task_receipt"
                    && d.interpretation == CheckStatus::Failed
                    && d.diagnostic.is_some()),
            "{mutation}"
        );
        assert!(
            rows::<ProviderCoverage>(&tables)
                .iter()
                .any(|c| c.family == FactFamily::Deployment
                    && c.status == CoverageStatus::Partial
                    && c.reason
                        == Some(if mutation == "oversized" {
                            ObligationKind::ResourceRefused
                        } else {
                            ObligationKind::OutsideProviderModel
                        })),
            "{mutation}"
        );
    }
}

/// Funds all real producers while independently denying the target-buffer allocation.
#[derive(Debug)]
struct TargetLimit(lctx_model::domain::resources::ResourceBudget);
impl lctx_model::domain::resources::ResourcePool for TargetLimit {
    fn reserve(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<Box<dyn lctx_model::domain::resources::Reservation>, ModelError> {
        if owner == "deployment_receipt_target" && bytes > 4096 {
            return Err(ModelError::Resource {
                owner,
                requested: bytes,
                used: 0,
                limit: 4096,
            });
        }
        self.0.reserve(owner, bytes)
    }
    fn reserved(&self) -> usize {
        self.0.reserved()
    }
    fn limit(&self) -> usize {
        self.0.limit()
    }
}
#[tokio::test]
async fn receipt_target_reservation_refuses_before_read_and_preserves_resource_class() {
    let resources = lctx_model::domain::resources::ResourceBudget::from_pool(Arc::new(
        TargetLimit(typed_driver::budget()),
    ))
    .unwrap();
    let tables = typed_driver::Tables::default();
    let result =
        typed_driver::run_profile_with_budget(
            captured_with_budget("", &resources),
            cpg_extract::pyrefly_stage::Pyrefly::new(
                cpg_extract::typed_syntax::SyntaxLimits::default(),
            ),
            Deployment(tables.clone()),
            lctx_model::domain::stages::Profile::Catalog,
            lctx_model::domain::batching::TransferLimits::default(),
            false,
            resources.clone(),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(ModelError::Resource {
                owner: "deployment_receipt_target",
                ..
            })
        ),
        "{result:?}"
    );
    assert!(rows::<TaskReport>(&tables).is_empty());
    assert!(
        !rows::<DeploymentObservation>(&tables)
            .iter()
            .any(|row| row.field == "task_receipt" && row.interpretation == CheckStatus::Failed)
    );
    assert_eq!(resources.reserved(), 0);
}
