//! Actual native compilation admits graph streams without a database or a publication transaction.
#[path = "fixtures/catalog_runtime.rs"]
mod runtime;
#[path = "fixtures/scoped_source_execution.rs"]
mod scoped_source_execution;
#[path = "fixtures/scoped_summary.rs"]
mod scoped_summary;
use cpg_core::{
    artifact,
    compilation::{self, PreparedCompilation},
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{ContentHash, admission::Frontier, graph::GraphFamily, stages::Profile};
use std::sync::Arc;

async fn compiled(
    profile: Profile,
    frontier: Frontier,
    memory: usize,
    batch_rows: usize,
) -> artifact::AdmittedArtifact {
    let workspace = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: memory,
            partitions: 1,
            batch_rows,
        },
    )
    .unwrap();
    let captured = runtime::capture("catalog_core", profile, workspace.budget());
    let prepared = matches!(frontier, Frontier::Analysis | Frontier::Catalog).then(|| {
        PreparedCompilation::new(
            frontier,
            runtime::settings("api"),
            captured.config().catalog(),
            None,
            workspace.budget(),
        )
        .unwrap()
    });
    compilation::compile(
        &workspace,
        captured.clone(),
        profile,
        ContentHash::of(b"artifact-fixture"),
        frontier,
        prepared.as_ref(),
        None,
        None,
    )
    .await
    .unwrap();
    artifact::admit(
        &workspace,
        &captured,
        frontier,
        profile,
        ContentHash::of(b"artifact-fixture"),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn both_profiles_admit_every_frontier_and_export_exact_originals() {
    for profile in Profile::ALL {
        for frontier in [
            Frontier::Facts,
            Frontier::Normalized,
            Frontier::Analysis,
            Frontier::Catalog,
        ] {
            let admitted = compiled(profile, frontier, 1 << 30, 4096).await;
            let manifest = admitted.manifest();
            manifest.validate().unwrap();
            assert_eq!(manifest.frontier, frontier);
            assert!(
                manifest
                    .families
                    .iter()
                    .any(|f| f.family == GraphFamily::Entities && f.rows > 0)
            );
            assert!(
                manifest
                    .families
                    .iter()
                    .any(|f| f.family == GraphFamily::Assertions && f.rows > 0)
            );
            assert!(!manifest.required_outcomes.is_empty());
            let root = tempfile::tempdir().unwrap();
            let output = root.path().join("graph");
            admitted.export(&output).unwrap();
            assert!(output.join("manifest.json").is_file());
            assert!(output.join("entities.arrow").is_file());
            for original in &manifest.originals {
                let bytes =
                    std::fs::read(output.join(format!("original-{}.bin", original.source.0.hex())))
                        .unwrap();
                assert_eq!(bytes.len() as u64, original.byte_len);
                assert_eq!(ContentHash::of(&bytes), original.content);
            }
            let marker = std::fs::read(output.join("manifest.json")).unwrap();
            assert!(admitted.export(&output).is_err());
            assert_eq!(std::fs::read(output.join("manifest.json")).unwrap(), marker);
        }
    }
}
#[tokio::test]
async fn graph_content_is_independent_of_transfer_batching() {
    let ordinary = compiled(Profile::Catalog, Frontier::Normalized, 1 << 30, 4096).await;
    let small = compiled(Profile::Catalog, Frontier::Normalized, 128 << 20, 7).await;
    assert_eq!(ordinary.manifest().content(), small.manifest().content());
}
#[tokio::test]
async fn transported_graph_refuses_missing_and_tampered_originals() {
    let admitted = compiled(Profile::Catalog, Frontier::Facts, 1 << 30, 4096).await;
    let resources = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("graph");
    admitted.export(&output).unwrap();
    admitted.verify_export(&output, &resources).await.unwrap();
    let original = &admitted.manifest().originals[0];
    let file = output.join(format!("original-{}.bin", original.source.0.hex()));
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(&file, b"tampered").unwrap();
    assert!(admitted.verify_export(&output, &resources).await.is_err());
    std::fs::write(&file, &bytes).unwrap();
    std::fs::remove_file(&file).unwrap();
    assert!(admitted.verify_export(&output, &resources).await.is_err());
    std::fs::write(&file, &bytes).unwrap();
    std::fs::write(output.join("assertions.arrow"), b"invalid stream").unwrap();
    assert!(admitted.verify_export(&output, &resources).await.is_err());
}
#[tokio::test]
async fn incomplete_or_cancelled_compilation_cannot_be_admitted() {
    let workspace = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let captured = runtime::capture("catalog_core", Profile::Catalog, workspace.budget());
    assert!(
        artifact::admit(
            &workspace,
            &captured,
            Frontier::Facts,
            Profile::Catalog,
            ContentHash::of(b"fixture")
        )
        .await
        .is_err()
    );
    workspace.cancellation().cancel();
    assert!(
        artifact::admit(
            &workspace,
            &captured,
            Frontier::Facts,
            Profile::Catalog,
            ContentHash::of(b"fixture")
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn artifact_identity_includes_exact_consumed_embedding_values() {
    use cpg_core::embedding_service::{Embedder, FakeEmbedder};
    let embedder = FakeEmbedder::new();
    let workspace = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let captured = runtime::capture("normalized_relations", Profile::Catalog, workspace.budget());
    let mut settings = runtime::settings("analytic_text");
    settings.knn = true;
    let prepared = PreparedCompilation::new(
        Frontier::Analysis,
        settings,
        captured.config().catalog(),
        Some(&embedder),
        workspace.budget(),
    )
    .unwrap();
    compilation::compile(
        &workspace,
        captured.clone(),
        Profile::Catalog,
        ContentHash::of(b"vector-artifact-fixture"),
        Frontier::Analysis,
        Some(&prepared),
        Some(&embedder),
        None,
    )
    .await
    .unwrap();
    let admitted = artifact::admit(
        &workspace,
        &captured,
        Frontier::Analysis,
        Profile::Catalog,
        ContentHash::of(b"vector-artifact-fixture"),
    )
    .await
    .unwrap();
    assert!(
        !admitted.manifest().embeddings.is_empty(),
        "selected exact kNN must retain actual consumption"
    );
    assert!(
        admitted
            .manifest()
            .embeddings
            .iter()
            .all(|value| value.dimension == embedder.spec().dimensions
                && value.specification == embedder.spec().hash())
    );
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("vector-graph");
    admitted.export(&destination).unwrap();
    admitted
        .verify_export(&destination, &workspace)
        .await
        .unwrap();
}

#[tokio::test]
async fn completed_compilation_binds_frontier_captures_and_configuration() {
    use lctx_model::domain::{input::Package, stages::ProviderOutcome};
    let workspace = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let captured = runtime::capture("catalog_core", Profile::Catalog, workspace.budget());
    let configuration = ContentHash::of(b"completed-facts-fixture");
    compilation::compile(
        &workspace,
        captured.clone(),
        Profile::Catalog,
        configuration,
        Frontier::Facts,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert!(
        artifact::admit(
            &workspace,
            &captured,
            Frontier::Normalized,
            Profile::Catalog,
            configuration
        )
        .await
        .is_err(),
        "completion cannot upgrade the frontier"
    );
    assert!(
        artifact::admit(
            &workspace,
            &captured,
            Frontier::Facts,
            Profile::Behavioral,
            configuration
        )
        .await
        .is_err()
    );
    assert!(
        artifact::admit(
            &workspace,
            &captured,
            Frontier::Facts,
            Profile::Catalog,
            ContentHash::of(b"different configuration")
        )
        .await
        .is_err()
    );
    let other = runtime::capture("normalized_relations", Profile::Catalog, workspace.budget());
    assert!(
        artifact::admit(
            &workspace,
            &other,
            Frontier::Facts,
            Profile::Catalog,
            configuration
        )
        .await
        .is_err(),
        "another capture cannot be relabelled admitted"
    );
    let late = workspace.output(
        "late",
        Profile::Catalog,
        configuration,
        workspace.inputs("late", Profile::Catalog, []).unwrap(),
    );
    assert!(late.declare::<Package>().is_err());
    assert!(late.finish(ProviderOutcome::Complete).await.is_err());
    artifact::admit(
        &workspace,
        &captured,
        Frontier::Facts,
        Profile::Catalog,
        configuration,
    )
    .await
    .unwrap();
}

fn transported<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Vec<T> {
    use datafusion::arrow::{array::BinaryArray, ipc::reader::FileReader};
    FileReader::try_new(std::fs::File::open(path).unwrap(), None)
        .unwrap()
        .flat_map(|batch| {
            let batch = batch.unwrap();
            let payload = batch
                .column_by_name("payload")
                .unwrap()
                .as_any()
                .downcast_ref::<BinaryArray>()
                .unwrap();
            (0..batch.num_rows())
                .map(|row| serde_json::from_slice(payload.value(row)).unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}
fn completed_rows<R: lctx_model::domain::Record>(workspace: &Workspace) -> Vec<R> {
    workspace
        .completed::<R>()
        .unwrap()
        .batches()
        .unwrap()
        .flat_map(|batch| R::decode(&batch.unwrap()).unwrap())
        .collect()
}
#[tokio::test]
async fn selected_analytics_export_membership_provenance_and_projection_losses() {
    use lctx_model::domain::{
        self as d, Record,
        graph::{AnalysisValue, Assertion, AssertionValue, Entity, SemanticKey},
    };
    let workspace = Workspace::new(
        Arc::new(d::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let captured = runtime::capture("analytic_optional", Profile::Catalog, workspace.budget());
    let mut settings = runtime::settings("api");
    settings.communities = true;
    settings.pagerank = true;
    settings.fca = true;
    settings.rca = true;
    let configuration = ContentHash::of(b"selected-analytics-artifact");
    let prepared = PreparedCompilation::new(
        Frontier::Analysis,
        settings,
        captured.config().catalog(),
        None,
        workspace.budget(),
    )
    .unwrap();
    compilation::compile(
        &workspace,
        captured.clone(),
        Profile::Catalog,
        configuration,
        Frontier::Analysis,
        Some(&prepared),
        None,
        None,
    )
    .await
    .unwrap();
    let admitted = artifact::admit(
        &workspace,
        &captured,
        Frontier::Analysis,
        Profile::Catalog,
        configuration,
    )
    .await
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("selected-graph");
    admitted.export(&destination).unwrap();
    admitted
        .verify_export(&destination, &workspace)
        .await
        .unwrap();
    let assertions: Vec<Assertion> = transported(&destination.join("assertions.arrow"));
    let entities: Vec<Entity> = transported(&destination.join("entities.arrow"));
    let results = assertions
        .iter()
        .filter_map(|value| match &value.value {
            AssertionValue::Analysis(AnalysisValue::AnalyticTechnique(row)) => Some(row),
            _ => None,
        })
        .collect::<Vec<_>>();
    for method in [
        d::analysis::AnalysisMethod::Communities,
        d::analysis::AnalysisMethod::PageRank,
        d::analysis::AnalysisMethod::Concepts,
        d::analysis::AnalysisMethod::RelationalConcepts,
    ] {
        assert!(
            results.iter().any(|row| row.method == method
                && row.selected
                && row.status != d::analysis::AnalysisStatus::NotRequested),
            "selected {method:?} has no attributed graph outcome"
        );
    }
    // These are semantic members and evidence, not optional diagnostic traces. Check exact
    // typed values and source keys against the completed owners, independently of graph mapping.
    // Explicit matches keep the acceptance inventory independent from emission macros.
    // Native run partitions are selected semantic memberships; heuristic presentation may be empty.
    let partitions = completed_rows::<d::analytics::PartitionMember>(&workspace);
    assert!(
        !partitions.is_empty(),
        "selected community analysis has no native partition memberships"
    );
    for row in partitions {
        assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Membership(d::graph::MembershipValue::AnalyticPartitionMember(v)) if v==&row)));
    }
    for row in completed_rows::<d::analytics::CommunityMember>(&workspace) {
        assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Membership(d::graph::MembershipValue::AnalyticCommunityMember(v)) if v==&row)));
    }
    for row in completed_rows::<d::analytics::ConceptExtent>(&workspace) {
        assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Membership(d::graph::MembershipValue::AnalyticConceptExtent(v)) if v==&row)));
    }
    for row in completed_rows::<d::analytics::ConceptIntent>(&workspace) {
        assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Membership(d::graph::MembershipValue::AnalyticConceptIntent(v)) if v==&row)));
    }
    for row in completed_rows::<d::analytics::Incidence>(&workspace) {
        assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Provenance(d::graph::ProvenanceValue::AnalyticIncidence(v)) if v==&row)));
    }
    for row in completed_rows::<d::analytics::IncidenceSource>(&workspace) {
        assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Provenance(d::graph::ProvenanceValue::AnalyticIncidenceSource(v)) if v==&row)));
    }
    assert!(!completed_rows::<d::analytics::ConceptExtent>(&workspace).is_empty());
    assert!(!completed_rows::<d::analytics::ConceptIntent>(&workspace).is_empty());
    assert!(!completed_rows::<d::analytics::Incidence>(&workspace).is_empty());
    for row in completed_rows::<d::analytics::Attribute>(&workspace) {
        assert!(
            entities
                .iter()
                .any(|entity| matches!(entity,Entity::AnalyticAttribute(v) if v==&row))
        );
    }
    let projection = admitted
        .manifest()
        .projections
        .iter()
        .find(|p| p.name == "CallableInvocation")
        .unwrap();
    assert_ne!(projection.source_membership, ContentHash::of(b""));
    assert_eq!(
        projection.definition,
        d::analysis::ProjectionDefinition::builtin(
            d::projection::ProjectionName::CallableInvocation
        )
        .content_digest()
    );
    assert!(
        projection
            .declared_losses
            .iter()
            .any(|loss| loss.contains("conditions, provider qualifications and source evidence"))
    );
    assert!(projection.declared_losses.iter().any(|loss| {
        loss.contains(d::normalized::entities::EntityCategory::Type.label())
            && loss.contains(d::normalized::entities::EntityCategory::Place.label())
    }));
    let mut manifest = admitted.manifest().clone();
    manifest
        .projections
        .iter_mut()
        .find(|p| p.name == "CallableInvocation")
        .unwrap()
        .declared_losses
        .clear();
    std::fs::write(
        destination.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(
        admitted
            .verify_export(&destination, &workspace)
            .await
            .is_err(),
        "transport cannot erase projection losses"
    );
}

#[tokio::test]
async fn raw_original_bytes_and_half_open_span_survive_artifact_transport() {
    use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
    use lctx_model::domain::{
        self as d, Record,
        graph::{Entity, EntityId},
    };
    let workspace = Workspace::new(
        Arc::new(d::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let input = tempfile::tempdir().unwrap();
    let raw = b"a\x00\xff\xfez\r\n";
    std::fs::write(input.path().join("original.bin"), raw).unwrap();
    std::fs::write(input.path().join("api.py"), b"def f(x): return x\n").unwrap();
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture(
                input.path(),
                &["api.py".into(), "original.bin".into()],
                workspace.budget(),
            )
            .unwrap(),
            "raw-artifact",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(
            Profile::Catalog,
            workspace.budget(),
        )
        .unwrap(),
    ));
    let configuration = ContentHash::of(b"raw-original-artifact");
    let source = captured.inputs()[0]
        .captured()
        .artifacts()
        .iter()
        .find(|row| row.path == "original.bin")
        .unwrap()
        .clone();
    let evidence = d::assertion::Evidence::SourceSpan {
        source: source.id(),
        start: 1,
        end: 4,
    };
    let declaration = d::stages::Stage {
        name: "raw-source-span-fixture",
        inputs: vec![],
        outputs: vec![],
        contributes: vec![d::stages::RelationUse::of::<d::assertion::Evidence>()],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: d::stages::Effect::Pure,
        code: ContentHash::of(b"raw-source-span-fixture/v1"),
        configuration,
    };
    let output = workspace.producer(
        &declaration,
        Profile::Catalog,
        workspace
            .inputs(declaration.name, Profile::Catalog, [])
            .unwrap(),
    );
    let batch = d::Batch::new(
        workspace.model(),
        vec![evidence.clone()],
        workspace.budget(),
    )
    .unwrap();
    output.contribute(&batch).unwrap();
    output
        .finish(d::stages::ProviderOutcome::Complete)
        .await
        .unwrap();
    compilation::compile(
        &workspace,
        captured.clone(),
        Profile::Catalog,
        configuration,
        Frontier::Facts,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let admitted = artifact::admit(
        &workspace,
        &captured,
        Frontier::Facts,
        Profile::Catalog,
        configuration,
    )
    .await
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("raw-graph");
    admitted.export(&destination).unwrap();
    admitted
        .verify_export(&destination, &workspace)
        .await
        .unwrap();
    assert!(completed_rows::<d::source::SourceArtifact>(&workspace).contains(&source));
    let source_id = EntityId::of(source.id());
    let originals =
        std::fs::read(destination.join(format!("original-{}.bin", source_id.0.hex()))).unwrap();
    assert_eq!(originals, raw);
    assert!(std::str::from_utf8(&originals).is_err());
    let entities: Vec<Entity> = transported(&destination.join("entities.arrow"));
    assert!(
        entities
            .iter()
            .any(|entity| matches!(entity,Entity::Source(row) if row==&source))
    );
    let restored = entities
        .into_iter()
        .find(|entity| entity == &Entity::from(evidence.clone()))
        .expect("admitted artifact lost its source-span evidence");
    if let Entity::Evidence(d::assertion::Evidence::SourceSpan { source, start, end }) = restored {
        assert_eq!(EntityId::of(source), source_id);
        assert_eq!(&originals[start as usize..end as usize], b"\x00\xff\xfe");
    } else {
        panic!("source evidence lost its typed span");
    }
}

#[tokio::test]
async fn trusted_local_export_verifies_without_live_compiler_token() {
    use datafusion::arrow::{
        array::{BinaryArray, FixedSizeBinaryArray},
        ipc::{reader::FileReader, writer::FileWriter},
        record_batch::RecordBatch,
    };
    use lctx_model::domain::graph::{
        EmbeddingConsumption, Entity, FamilyContent, FamilyHasher, Manifest,
    };
    use std::{fs::File, io::Read};
    let admitted = compiled(Profile::Catalog, Frontier::Normalized, 1 << 30, 4096).await;
    let workspace = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("graph");
    admitted.export(&output).unwrap();
    admitted.verify_export(&output, &workspace).await.unwrap();
    let manifest = admitted.manifest().clone();
    drop(admitted);
    let verified = artifact::verify_export(&output, &workspace).await.unwrap();
    assert_eq!(verified.manifest(), &manifest);
    assert_eq!(
        verified
            .entities()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .len() as u64,
        manifest.families[0].rows
    );
    assert_eq!(
        verified
            .assertions()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .len() as u64,
        manifest.families[1].rows
    );
    for original in verified.originals() {
        let (original, mut input) = original.unwrap();
        let mut bytes = vec![];
        input.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes.len() as u64, original.byte_len);
        assert_eq!(ContentHash::of(&bytes), original.content);
    }
    drop(verified);
    let marker = output.join("manifest.json");
    let write_manifest = |value: &Manifest| {
        std::fs::write(&marker, serde_json::to_vec(value).unwrap()).unwrap();
    };
    let mut changed = manifest.clone();
    changed.semantic_contract = ContentHash::of(b"another model");
    write_manifest(&changed);
    assert!(matches!(
        artifact::verify_export(&output, &workspace).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "artifact semantic contract"
        ))
    ));
    changed = manifest.clone();
    changed.families[0].rows += 1;
    write_manifest(&changed);
    assert!(matches!(
        artifact::verify_export(&output, &workspace).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "artifact graph family"
        ))
    ));
    changed = manifest.clone();
    changed.families.push(FamilyContent {
        family: GraphFamily::Projection,
        rows: 0,
        content: ContentHash::of(b"extra"),
    });
    write_manifest(&changed);
    assert!(matches!(
        artifact::verify_export(&output, &workspace).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "artifact graph families"
        ))
    ));
    changed = manifest.clone();
    changed.originals.pop().unwrap();
    write_manifest(&changed);
    assert!(matches!(
        artifact::verify_export(&output, &workspace).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "artifact source membership"
        ))
    ));
    changed = manifest.clone();
    assert!(!changed.projections.is_empty());
    changed.projections[0].definition = ContentHash::of(b"another projection");
    write_manifest(&changed);
    assert!(matches!(
        artifact::verify_export(&output, &workspace).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "artifact projection definition"
        ))
    ));
    changed = manifest.clone();
    changed.embeddings.push(EmbeddingConsumption {
        specification: ContentHash::of(b"unconsumed specification"),
        text: ContentHash::of(b"unconsumed text"),
        dimension: 1,
        values: ContentHash::of(b"unconsumed values"),
    });
    write_manifest(&changed);
    assert!(matches!(
        artifact::verify_export(&output, &workspace).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "artifact embedding membership"
        ))
    ));
    write_manifest(&manifest);
    let entity_file = output.join("entities.arrow");
    let original_entities = std::fs::read(&entity_file).unwrap();
    let batches = FileReader::try_new(File::open(&entity_file).unwrap(), None)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let write_batches = |batches: &[RecordBatch]| {
        let mut writer =
            FileWriter::try_new(File::create(&entity_file).unwrap(), &batches[0].schema()).unwrap();
        for batch in batches {
            writer.write(batch).unwrap();
        }
        writer.finish().unwrap();
    };
    let mut noncanonical = batches.clone();
    let first = &batches[0];
    let payloads = first
        .column(2)
        .as_any()
        .downcast_ref::<BinaryArray>()
        .unwrap();
    let altered = BinaryArray::from_iter_values((0..first.num_rows()).map(|row| {
        let mut payload = payloads.value(row).to_vec();
        if row == 0 {
            payload.insert(0, b' ');
        }
        payload
    }));
    let mut columns = first.columns().to_vec();
    columns[2] = Arc::new(altered);
    noncanonical[0] = RecordBatch::try_new(first.schema(), columns).unwrap();
    write_batches(&noncanonical);
    assert!(matches!(
        artifact::verify_export(&output, &workspace).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "artifact canonical payload"
        ))
    ));
    std::fs::write(&entity_file, &original_entities).unwrap();
    // Alter the declared family to match a graph missing a provider. A self-consistent digest
    // cannot hide missing semantic endpoints from the shared reference-closure validator.
    let mut missing = vec![];
    let mut removed = false;
    let mut family = FamilyHasher::new(GraphFamily::Entities);
    for batch in &batches {
        let payloads = batch
            .column(2)
            .as_any()
            .downcast_ref::<BinaryArray>()
            .unwrap();
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        let hashes = batch
            .column(1)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        for row in 0..batch.num_rows() {
            let value: Entity = serde_json::from_slice(payloads.value(row)).unwrap();
            if !removed && matches!(value, Entity::Provider(_)) {
                removed = true;
                continue;
            }
            family
                .push(
                    ContentHash(ids.value(row).try_into().unwrap()),
                    ContentHash(hashes.value(row).try_into().unwrap()),
                )
                .unwrap();
            missing.push(batch.slice(row, 1));
        }
    }
    assert!(removed);
    write_batches(&missing);
    changed = manifest.clone();
    changed.families[0] = family.finish();
    write_manifest(&changed);
    let error = artifact::verify_export(&output, &workspace)
        .await
        .err()
        .unwrap();
    assert!(
        error.to_string().contains("graph reference closure"),
        "{error}"
    );
    std::fs::write(&entity_file, &original_entities).unwrap();
    write_manifest(&manifest);
    let original = &manifest.originals[0];
    let file = output.join(format!("original-{}.bin", original.source.0.hex()));
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(&file, b"tampered").unwrap();
    assert!(matches!(
        artifact::verify_export(&output, &workspace).await,
        Err(lctx_model::domain::ModelError::Conflict(
            "artifact original bytes"
        ))
    ));
    std::fs::write(&file, &bytes).unwrap();
    std::fs::remove_file(&file).unwrap();
    assert!(artifact::verify_export(&output, &workspace).await.is_err());
    std::fs::write(&file, &bytes).unwrap();
    artifact::verify_export(&output, &workspace).await.unwrap();
}

#[tokio::test]
async fn remediation_detached_admission_refuses_unsupported_facts_with_consistent_hashes() {
    use datafusion::arrow::{
        array::{Array, BinaryArray, FixedSizeBinaryArray, UInt32Array},
        compute::take,
        ipc::{reader::FileReader, writer::FileWriter},
        record_batch::RecordBatch,
    };
    use lctx_model::domain::{
        Record,
        graph::{Assertion, FamilyHasher, Manifest},
        source::SyntaxSupport,
    };
    use std::fs::File;
    let admitted = compiled(Profile::Catalog, Frontier::Facts, 1 << 30, 256).await;
    let runtime = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("graph");
    admitted.export(&output).unwrap();
    artifact::verify_export(&output, &runtime).await.unwrap();
    let file = output.join("assertions.arrow");
    let reader = FileReader::try_new(File::open(&file).unwrap(), None).unwrap();
    let schema = reader.schema();
    let replacement = output.join("changed.arrow");
    let mut writer = FileWriter::try_new(File::create(&replacement).unwrap(), &schema).unwrap();
    let mut hasher = FamilyHasher::new(GraphFamily::Assertions);
    let mut removed = 0;
    for batch in reader {
        let batch = batch.unwrap();
        let payloads = batch
            .column(2)
            .as_any()
            .downcast_ref::<BinaryArray>()
            .unwrap();
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        let contents = batch
            .column(1)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        let mut keep = Vec::new();
        for index in 0..batch.num_rows() {
            let row: Assertion = serde_json::from_slice(payloads.value(index)).unwrap();
            if row
                .source
                .as_ref()
                .is_some_and(|source| source.domain() == SyntaxSupport::NAME)
            {
                removed += 1;
            } else {
                keep.push(index as u32);
                hasher
                    .push(
                        ContentHash(ids.value(index).try_into().unwrap()),
                        ContentHash(contents.value(index).try_into().unwrap()),
                    )
                    .unwrap();
            }
        }
        let indices = UInt32Array::from(keep);
        let columns = batch
            .columns()
            .iter()
            .map(|column| take(column.as_ref(), &indices, None).unwrap())
            .collect();
        writer
            .write(&RecordBatch::try_new(schema.clone(), columns).unwrap())
            .unwrap();
    }
    assert!(removed > 0);
    writer.finish().unwrap();
    drop(writer);
    std::fs::rename(replacement, &file).unwrap();
    let mut manifest: Manifest =
        serde_json::from_slice(&std::fs::read(output.join("manifest.json")).unwrap()).unwrap();
    *manifest
        .families
        .iter_mut()
        .find(|f| f.family == GraphFamily::Assertions)
        .unwrap() = hasher.finish();
    manifest.validate().unwrap();
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let error = match artifact::verify_export(&output, &runtime).await {
        Ok(_) => panic!("unsupported facts admitted"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("assertion has no attributed support"),
        "{error}"
    );
}

#[tokio::test]
async fn remediation_detached_admission_all_frontiers_and_profiles_without_producer_replay() {
    for profile in Profile::ALL {
        for frontier in [
            Frontier::Facts,
            Frontier::Normalized,
            Frontier::Analysis,
            Frontier::Catalog,
        ] {
            let admitted = compiled(profile, frontier, 1 << 30, 128).await;
            let directory = tempfile::tempdir().unwrap();
            let export = directory.path().join("export");
            admitted.export(&export).unwrap();
            let semantic = admitted.manifest().content();
            drop(admitted);
            // This workspace has no providers, captured inputs, producer receipts or grants.
            let importer = Workspace::new(
                Arc::new(lctx_model::domain::model().unwrap()),
                WorkspaceOptions {
                    memory_bytes: 1 << 30,
                    batch_rows: 31,
                    ..Default::default()
                },
            )
            .unwrap();
            let imported = artifact::verify_export(&export, &importer)
                .await
                .unwrap_or_else(|error| panic!("{profile:?}/{frontier:?}: {error}"));
            assert_eq!(imported.manifest().content(), semantic);
        }
    }
}

#[tokio::test]
async fn remediation_detached_admission_refuses_equal_count_foreign_outcome_domain() {
    use lctx_model::domain::graph::Manifest;
    let admitted = compiled(Profile::Catalog, Frontier::Facts, 1 << 30, 128).await;
    let directory = tempfile::tempdir().unwrap();
    let export = directory.path().join("export");
    admitted.export(&export).unwrap();
    let mut manifest: Manifest =
        serde_json::from_slice(&std::fs::read(export.join("manifest.json")).unwrap()).unwrap();
    let count = manifest.outcomes.len();
    let key = manifest.outcomes[0].key.clone();
    manifest.outcomes[0].key.domain = ContentHash::of(b"foreign-outcome-domain");
    let replacement = manifest.outcomes[0].key.clone();
    *manifest
        .required_outcomes
        .iter_mut()
        .find(|required| **required == key)
        .unwrap() = replacement;
    manifest.required_outcomes.sort();
    manifest.outcomes.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(manifest.outcomes.len(), count);
    manifest.validate().unwrap();
    std::fs::write(
        export.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let importer = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..Default::default()
        },
    )
    .unwrap();
    let error = match artifact::verify_export(&export, &importer).await {
        Ok(_) => panic!("foreign outcome domain admitted"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("semantic outcome"), "{error}");
}

/// These finite fixture rows are decoded from the actual canonical export. The production
/// admission owner is selected from the same complete declared frontier, including empty tables.
mod enriched_graph_adversaries {
    use super::*;
    use datafusion::arrow::{
        array::{BinaryArray, FixedSizeBinaryArray, UInt32Array},
        compute::take,
        ipc::{reader::FileReader, writer::FileWriter},
        record_batch::RecordBatch,
    };
    use lctx_model::domain::{
        self as d, Record,
        graph::{Assertion, Entity, FamilyHasher, Manifest, Target},
    };
    use std::{
        collections::{BTreeSet, HashSet},
        fs::File,
        path::Path,
    };

    #[derive(Default)]
    struct Compact {
        frames: Vec<d::analysis::enriched_execution::AnalysisInvocation>,
        sources: Vec<d::analysis::source_call::AnalysisInvocation>,
        inputs: Vec<d::analysis::enriched_execution::AnalysisInput>,
        parents: Vec<d::analysis::enriched_execution::InvocationSource>,
        definitions: Vec<d::analysis::AnalysisDefinition>,
        parameters: Vec<d::analysis::MethodParameters>,
    }
    fn graph(path: &Path) -> (Vec<Entity>, Vec<Assertion>) {
        let mut entities = Vec::new();
        let mut assertions = Vec::new();
        for (file, intrinsic) in [("entities.arrow", true), ("assertions.arrow", false)] {
            for batch in FileReader::try_new(File::open(path.join(file)).unwrap(), None).unwrap() {
                let batch = batch.unwrap();
                let payloads = batch
                    .column(2)
                    .as_any()
                    .downcast_ref::<BinaryArray>()
                    .unwrap();
                for row in 0..batch.num_rows() {
                    if intrinsic {
                        entities.push(serde_json::from_slice(payloads.value(row)).unwrap());
                    } else {
                        assertions.push(serde_json::from_slice(payloads.value(row)).unwrap());
                    }
                }
            }
        }
        (entities, assertions)
    }
    fn compact(entities: &[Entity], assertions: &[Assertion]) -> Compact {
        let mut rows = Compact::default();
        for entity in entities {
            match entity {
                Entity::EnrichedExecutionRun(row) => rows.frames.push(row.clone()),
                Entity::SourceCallRun(row) => rows.sources.push(row.clone()),
                Entity::EnrichedExecutionParent(row) => rows.parents.push(row.clone()),
                Entity::AnalysisDefinition(row) => rows.definitions.push(row.clone()),
                Entity::MethodParameters(row) => rows.parameters.push(row.clone()),
                _ => {}
            }
        }
        for assertion in assertions {
            if assertion.source.as_ref().is_some_and(|source| {
                source.domain() == d::analysis::enriched_execution::AnalysisInput::NAME
            }) {
                rows.inputs
                    .push(d::graph::record::assertion_record(assertion).unwrap());
            }
        }
        rows
    }
    fn check(rows: &Compact) -> Result<(), d::ModelError> {
        let model = d::model().unwrap();
        // SemanticImport declares the whole frontier, even when an owned family has no rows.
        let names = d::analysis_frontier_relations()
            .iter()
            .map(d::Relation::name)
            .collect::<BTreeSet<_>>();
        assert!(names.contains(d::analysis::enriched_execution::AnalysisInvocation::NAME));
        let candidates = model.admission_candidates_for_scope(&names).unwrap();
        let invariant = candidates
            .iter()
            .find(|invariant| invariant.name == "enriched_frame_fidelity")
            .expect("complete detached frontier must select necessary Enriched frame admission");
        assert_eq!(invariant.purpose, d::InvariantPurpose::Admission);
        let budget = d::resources::ResourceBudget::fixed(128 << 20).unwrap();
        let mut checker = (invariant.create)(&budget);
        macro_rules! visit {
            ($field:ident,$ty:ty) => {{
                let input = invariant
                    .inputs
                    .iter()
                    .find(|input| input.name() == <$ty>::NAME)
                    .unwrap();
                checker.visit_input(input, &<$ty>::encode(&rows.$field)?)?;
            }};
        }
        visit!(frames, d::analysis::enriched_execution::AnalysisInvocation);
        visit!(sources, d::analysis::source_call::AnalysisInvocation);
        visit!(inputs, d::analysis::enriched_execution::AnalysisInput);
        visit!(parents, d::analysis::enriched_execution::InvocationSource);
        visit!(definitions, d::analysis::AnalysisDefinition);
        visit!(parameters, d::analysis::MethodParameters);
        checker.finish()
    }
    fn key(target: &Target) -> Option<(u8, ContentHash)> {
        match target {
            Target::Entity(id) => Some((0, id.0)),
            Target::Assertion(id) => Some((1, id.0)),
            Target::External { .. } => None,
        }
    }
    /// Delete the selected Enriched frames and only graph rows that actually refer to removed
    /// endpoints. Kept identities, payloads and role-labelled edges remain byte-for-byte exact.
    fn remove_frames(path: &Path, entities: &[Entity], assertions: &[Assertion]) {
        let mut removed = entities
            .iter()
            .filter(|entity| matches!(entity, Entity::EnrichedExecutionRun(_)))
            .map(|entity| (0, entity.id().0))
            .collect::<HashSet<_>>();
        assert!(!removed.is_empty());
        loop {
            let before = removed.len();
            for entity in entities {
                if entity
                    .references()
                    .unwrap()
                    .iter()
                    .any(|(target, _)| key(target).is_some_and(|key| removed.contains(&key)))
                {
                    removed.insert((0, entity.id().0));
                }
            }
            for assertion in assertions {
                if assertion
                    .references()
                    .unwrap()
                    .iter()
                    .any(|(target, _)| key(target).is_some_and(|key| removed.contains(&key)))
                {
                    removed.insert((1, assertion.id().0));
                }
            }
            if removed.len() == before {
                break;
            }
        }
        let mut manifest: Manifest =
            serde_json::from_slice(&std::fs::read(path.join("manifest.json")).unwrap()).unwrap();
        for (file, family, kind) in [
            ("entities.arrow", GraphFamily::Entities, 0),
            ("assertions.arrow", GraphFamily::Assertions, 1),
        ] {
            let file = path.join(file);
            let reader = FileReader::try_new(File::open(&file).unwrap(), None).unwrap();
            let schema = reader.schema();
            let replacement = file.with_extension("changed.arrow");
            let mut writer =
                FileWriter::try_new(File::create(&replacement).unwrap(), &schema).unwrap();
            let mut hasher = FamilyHasher::new(family);
            for batch in reader {
                let batch = batch.unwrap();
                let ids = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<FixedSizeBinaryArray>()
                    .unwrap();
                let contents = batch
                    .column(1)
                    .as_any()
                    .downcast_ref::<FixedSizeBinaryArray>()
                    .unwrap();
                let mut keep = Vec::new();
                for row in 0..batch.num_rows() {
                    let id = ContentHash(ids.value(row).try_into().unwrap());
                    if !removed.contains(&(kind, id)) {
                        hasher
                            .push(id, ContentHash(contents.value(row).try_into().unwrap()))
                            .unwrap();
                        keep.push(row as u32);
                    }
                }
                let indices = UInt32Array::from(keep);
                let columns = batch
                    .columns()
                    .iter()
                    .map(|column| take(column.as_ref(), &indices, None).unwrap())
                    .collect();
                writer
                    .write(&RecordBatch::try_new(schema.clone(), columns).unwrap())
                    .unwrap();
            }
            writer.finish().unwrap();
            drop(writer);
            std::fs::rename(replacement, file).unwrap();
            *manifest
                .families
                .iter_mut()
                .find(|content| content.family == family)
                .unwrap() = hasher.finish();
        }
        manifest.validate().unwrap();
        std::fs::write(
            path.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn remediation_detached_enriched_frame_admission_uses_actual_compact_graph_domain() {
        let admitted = compiled(Profile::Catalog, Frontier::Analysis, 1 << 30, 128).await;
        let directory = tempfile::tempdir().unwrap();
        let export = directory.path().join("export");
        admitted.export(&export).unwrap();
        drop(admitted);
        let (entities, assertions) = graph(&export);
        let mut actual = compact(&entities, &assertions);
        assert!(
            !actual.sources.is_empty() && !actual.frames.is_empty() && !actual.inputs.is_empty()
        );
        check(&actual).unwrap();
        let frame = actual
            .frames
            .iter()
            .find(|frame| {
                actual
                    .inputs
                    .iter()
                    .any(|input| input.invocation == frame.id())
            })
            .unwrap()
            .clone();
        let old = frame.id();
        let mut parents = actual
            .inputs
            .iter()
            .filter(|input| input.invocation == old)
            .map(|input| input.parent)
            .collect::<Vec<_>>();
        parents.sort();
        let omitted = parents.remove(0);
        assert!(actual.parents.iter().any(|parent| parent.id() == omitted));
        let (mut changed, _) = d::analysis::enriched_execution::AnalysisInvocation::new(
            frame.input,
            frame.context,
            frame.definition,
            frame.subject,
            parents.clone(),
        );
        changed.sources = frame.sources;
        changed.projections = frame.projections;
        assert_ne!(changed.id(), old);
        let canonical = Entity::from(changed.clone());
        canonical.validate().unwrap();
        assert_eq!(
            d::graph::record::entity_record::<d::analysis::enriched_execution::AnalysisInvocation>(
                &canonical
            )
            .unwrap(),
            changed
        );
        *actual
            .frames
            .iter_mut()
            .find(|row| row.id() == old)
            .unwrap() = changed.clone();
        actual.inputs.retain(|row| row.invocation != old);
        for parent in parents {
            let row = d::analysis::enriched_execution::AnalysisInput {
                invocation: changed.id(),
                parent,
            };
            let canonical = Assertion::from_record(row.clone()).unwrap();
            // Rebuild the nominal assertion identity and its exact role-labelled parent/run edges.
            assert_eq!(d::graph::record::assertion_record::<d::analysis::enriched_execution::AnalysisInput>(&canonical).unwrap(),row);
            assert!(
                canonical
                    .references()
                    .unwrap()
                    .iter()
                    .any(|(target, _)| *target
                        == Target::Entity(d::graph::EntityId::of(changed.id())))
            );
            actual.inputs.push(row);
        }
        let error = check(&actual).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Enriched exact complete SourceCall parent domain"),
            "{error}"
        );
        // The compact case intentionally does not rewrite every downstream semantic digest. The
        // separate empty-family case below exercises complete detached transport and admission.
    }

    #[tokio::test]
    async fn remediation_detached_admission_refuses_absent_whole_enriched_frame_family() {
        let admitted = compiled(Profile::Catalog, Frontier::Analysis, 1 << 30, 128).await;
        let directory = tempfile::tempdir().unwrap();
        let export = directory.path().join("export");
        admitted.export(&export).unwrap();
        drop(admitted);
        let importer = || {
            Workspace::new(
                Arc::new(d::model().unwrap()),
                WorkspaceOptions {
                    memory_bytes: 1 << 30,
                    ..Default::default()
                },
            )
            .unwrap()
        };
        artifact::verify_export(&export, &importer()).await.unwrap();
        let (entities, assertions) = graph(&export);
        remove_frames(&export, &entities, &assertions);
        let (entities, assertions) = graph(&export);
        let rows = compact(&entities, &assertions);
        assert!(rows.frames.is_empty() && rows.inputs.is_empty());
        assert!(!rows.sources.is_empty());
        let error = check(&rows).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Enriched complete actual frame domain"),
            "{error}"
        );
        let existing_entities = entities
            .iter()
            .map(|row| (0, row.id().0))
            .collect::<HashSet<_>>();
        let existing_assertions = assertions
            .iter()
            .map(|row| (1, row.id().0))
            .collect::<HashSet<_>>();
        for (target, _) in entities
            .iter()
            .flat_map(|row| row.references().unwrap())
            .chain(assertions.iter().flat_map(|row| row.references().unwrap()))
        {
            if let Some(key) = key(&target) {
                assert!(
                    existing_entities.contains(&key) || existing_assertions.contains(&key),
                    "missing kept nominal endpoint {target:?}"
                );
            }
        }
        let error = match artifact::verify_export(&export, &importer()).await {
            Ok(_) => panic!("absent Enriched frame family admitted"),
            Err(error) => error,
        };
        // Independent completeness owners may also refuse deleted dependent rows. The direct
        // selected predicate above establishes the Enriched refusal, and this asserts detached
        // verification reaches semantic admission rather than transport/hash/reference rejection.
        assert!(
            matches!(
                error,
                d::ModelError::Invalid(_) | d::ModelError::Frontier(_)
            ),
            "{error}"
        );
    }
}

async fn detached_behavioral_upper_place_endpoints(frontier: Frontier) {
    use lctx_model::domain::{Record, graph::Entity, normalized::entities::EntityRef};
    use std::collections::BTreeSet;
    let admitted = compiled(Profile::Behavioral, frontier, 1 << 30, 128).await;
    let directory = tempfile::tempdir().unwrap();
    let export = directory.path().join("export");
    admitted.export(&export).unwrap();
    let semantic = admitted.manifest().content();
    drop(admitted);
    let importer = Workspace::new(
        Arc::new(lctx_model::domain::model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            batch_rows: 31,
            ..Default::default()
        },
    )
    .unwrap();
    let imported = artifact::verify_export(&export, &importer)
        .await
        .unwrap_or_else(|error| panic!("Behavioral/{frontier:?}: {error}"));
    assert_eq!(imported.manifest().content(), semantic);
    let mut places = BTreeSet::new();
    let mut endpoints = BTreeSet::new();
    for row in imported.entities().unwrap() {
        match row.unwrap() {
            Entity::Place(place) => {
                places.insert(place.id());
            }
            Entity::EntityReference(EntityRef::Place { place }) => {
                endpoints.insert(place);
            }
            _ => {}
        }
    }
    assert!(
        !places.is_empty(),
        "actual Behavioral frontier vocabulary must contain places"
    );
    if frontier == Frontier::Facts {
        assert!(
            endpoints.is_empty(),
            "Facts must not transport undeclared normalized endpoints"
        );
    } else {
        assert!(
            places.is_subset(&endpoints),
            "every final canonical place needs its exact endpoint"
        );
    }
}

#[tokio::test]
async fn remediation_detached_behavioral_analysis_with_upper_place_endpoints() {
    detached_behavioral_upper_place_endpoints(Frontier::Analysis).await;
}

#[tokio::test]
async fn remediation_detached_behavioral_catalog_with_upper_place_endpoints() {
    detached_behavioral_upper_place_endpoints(Frontier::Catalog).await;
}

#[tokio::test]
async fn remediation_detached_behavioral_facts_without_normalized_place_endpoints() {
    detached_behavioral_upper_place_endpoints(Frontier::Facts).await;
}
