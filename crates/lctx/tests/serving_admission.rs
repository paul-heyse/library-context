//! Real catalog generation: semantic admission precedes filters, empty catalogs retain corpus.
#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{self, serving::ranking::DocumentScore, serving::*};
use lctx_postgres::generations::{Error, RetrievalService};
use support::ServingFixture;
fn request(tool: &str, value: serde_json::Value) -> Request {
    decode_request(tool, &value.to_string(), &ResourceLimits::default()).unwrap()
}
#[tokio::test]
async fn admitted_empty_corpus_and_unknown_domain_agree_through_pg_native_and_wire() {
    let fixture=ServingFixture::start_with_analytics(b"__all__ = []\n", "catalog", &[], 0, None,
        Some(b"# Captured deployment guide\n\nPinned corpus instructions remain discoverable without public API members.\n")).await;
    let execution = fixture.service.execution().await.unwrap();
    let retrieval = RetrievalService::prepare(fixture.catalog.clone(), false)
        .await
        .unwrap();
    let library = Name::new("demo").unwrap();
    let find = FindOperationsRequest {
        library: library.clone(),
        selection: SelectionInput::default(),
        page: PageRequest::default(),
    };
    let found = fixture.catalog.find(&execution, &find).await.unwrap();
    assert!(
        found.supported.items.is_empty()
            && found.unresolved.items.is_empty()
            && found.conflicting.items.is_empty()
    );
    assert!(matches!(
        found.extent,
        SelectionExtent::CompleteDomain { total: 0 }
    ));
    assert_eq!(found.domains.len(), 1);
    assert_eq!(found.domains[0].name.as_str(), "demo");
    assert_eq!(found.domains[0].captures.len(), 1);
    let old_policy = identity::policy_identity(&(
        "catalog-complete/v1",
        "public-path-member-analysis-order",
        "source-declaration-ownership",
    ))
    .unwrap();
    let old_cursor = Cursor {
        offset: 0,
        binding: CursorBinding {
            generation: GenerationKey(*fixture.generation.bytes()),
            request: Request::FindOperations(find.clone())
                .canonical_identity()
                .unwrap(),
            policy: old_policy,
            wire: wire_identity(),
            channels: ChannelState {
                lexical: false,
                vector: VectorChannel::Disabled {},
            }
            .identity(),
            group: Name::new("supported").unwrap(),
            section: Name::new("members").unwrap(),
            member: None,
            ordering: domain::ContentHash::of(b"catalog-public-path/member/analysis/v1"),
        },
    }
    .encode()
    .unwrap();
    let mut legacy = find.clone();
    legacy.page.cursor = Optional(Some(old_cursor));
    assert!(
        matches!(
            fixture.catalog.find(&execution, &legacy).await,
            Err(Error::Codec(_))
        ),
        "pre-admission catalog policy cannot resume under the new contract"
    );

    assert!(!found.domains[0].captures[0].corpora.is_empty());
    assert!(!found.domains[0].captures[0].coverage.is_empty());
    assert!(
        found.domains[0].captures[0]
            .coverage
            .iter()
            .any(|c| c.status == domain::attribution::CoverageStatus::NotRequested)
    );
    let operation = OperationSelector::PublicPath {
        path: vec![Name::new("demo").unwrap(), Name::new("api").unwrap()],
    };
    let get = fixture
        .catalog
        .operation(
            &execution,
            &GetOperationRequest {
                library: library.clone(),
                operation: operation.clone(),
                comparison: Optional::default(),
                reference_parameter: Optional::default(),
                sections: vec![],
                page: PageRequest::default(),
            },
        )
        .await
        .unwrap();
    assert!(matches!(get.operation, OperationResolution::Missing { .. }));
    assert_eq!(get.domains, found.domains);
    let browse = fixture
        .catalog
        .browse(
            &execution,
            &BrowseLibraryRequest {
                library: library.clone(),
                scope: BrowseScope::default(),
                view: BrowseView::Members,
                selection: SelectionInput::default(),
                page: PageRequest::default(),
            },
        )
        .await
        .unwrap();
    assert!(browse.entries.items.is_empty());
    assert_eq!(browse.domains, found.domains);
    let compare = fixture
        .catalog
        .compare(
            &execution,
            &CompareOperationsRequest {
                library: library.clone(),
                operations: vec![operation],
                selection: SelectionInput::default(),
                page: PageRequest::default(),
            },
        )
        .await
        .unwrap();
    assert!(compare.operations[0].candidates.is_empty());
    assert_eq!(compare.domains, found.domains);
    for (tool, value) in [
        ("find_operations", serde_json::json!({"library":"unknown"})),
        (
            "get_operation",
            serde_json::json!({"library":"unknown","operation":{"kind":"public_path","path":["absent"]}}),
        ),
        ("browse_library", serde_json::json!({"library":"unknown"})),
        (
            "compare_operations",
            serde_json::json!({"library":"unknown","operations":[{"kind":"public_path","path":["absent"]}]}),
        ),
    ] {
        let result = match request(tool, value) {
            Request::FindOperations(r) => fixture.catalog.find(&execution, &r).await.map(|_| ()),
            Request::GetOperation(r) => fixture.catalog.operation(&execution, &r).await.map(|_| ()),
            Request::BrowseLibrary(r) => fixture.catalog.browse(&execution, &r).await.map(|_| ()),
            Request::CompareOperations(r) => {
                fixture.catalog.compare(&execution, &r).await.map(|_| ())
            }
            _ => panic!("catalog route"),
        };
        assert!(
            matches!(result, Err(Error::LibraryAdmission(_))),
            "{tool}: {result:?}"
        );
    }
    let corpus = retrieval.numerical_corpus(&execution).unwrap();
    for tool in [
        "search_operations",
        "search_evidence",
        "search_capabilities",
    ] {
        let raw = if tool == "search_evidence" {
            serde_json::json!({"library":"unknown","query":"deployment","families":[]})
        } else {
            serde_json::json!({"library":"unknown","query":"deployment"})
        };
        let unknown = request(tool, raw);
        assert!(matches!(
            retrieval.request(&execution, &unknown, None, None).await,
            Err(Error::LibraryAdmission(_))
        ));
        let raw = if tool == "search_evidence" {
            serde_json::json!({"query":"deployment","families":[]})
        } else {
            serde_json::json!({"query":"deployment"})
        };
        let req = request(tool, raw);
        let prepared = retrieval
            .request(&execution, &req, None, None)
            .await
            .unwrap();
        let scores = corpus
            .documents
            .iter()
            .map(|d| DocumentScore {
                document: d.id,
                score: Some(1.0),
            })
            .collect();
        let (prepared, ranked) = retrieval
            .rank(&execution, prepared, scores, None)
            .await
            .unwrap();
        let response = retrieval
            .finish(&execution, req, prepared, ranked)
            .await
            .unwrap();
        match response {
            Response::SearchOperations(r) => {
                assert!(r.results.items.is_empty());
                assert_eq!(r.domains, found.domains);
            }
            Response::SearchEvidence(r) => {
                assert!(!r.results.items.is_empty());
                assert_eq!(r.domains, found.domains);
            }
            Response::SearchCapabilities(r) => {
                assert!(r.results.items.is_empty());
                assert_eq!(r.domains, found.domains);
            }
            _ => panic!("search route"),
        }
    }
    drop(execution);
    let python_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let python_paths = std::env::join_paths([
        python_root.join("python/lctx_mcp/src"),
        python_root.join("python/lctx_storage/python"),
        python_root.join("python/lctx_semantics/python"),
    ])
    .unwrap();
    let transport = std::process::Command::new("uv")
        .current_dir(&python_root)
        .args([
            "run",
            "--no-sync",
            "pytest",
            "-q",
            "python/lctx_mcp/tests/current_transport.py",
            "-k",
            "library_admission",
        ])
        .env("PYTHONPATH", python_paths)
        .env(
            "LCTX_SERVING_TEST_CONFIG",
            fixture.dir.path().join("postgres-serving.json"),
        )
        .env("LCTX_SERVING_TEST_GENERATION", fixture.generation.hex())
        .env("LCTX_SERVING_TEST_LIBRARY", "demo")
        .env("LCTX_SERVING_TEST_PROFILE", "catalog")
        .env("LCTX_SERVING_ADMISSION_EMPTY", "1")
        .output()
        .unwrap();
    assert!(
        transport.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&transport.stdout),
        String::from_utf8_lossy(&transport.stderr)
    );
    drop(retrieval);
    fixture.finish().await;
}

#[tokio::test]
async fn subjectless_first_party_documents_survive_explicit_and_union_search_with_originals() {
    let document = b"# Captured packaging guide\n\nStandalone packaging instructions retain exact original bytes.\n";
    let fixture = ServingFixture::start_captured(b"__all__ = ['api']\ndef api():\n    pass\n", Some(document), &["1.0"]).await;
    let execution = fixture.service.execution().await.unwrap();
    let retrieval = RetrievalService::prepare(fixture.catalog.clone(), false).await.unwrap();
    let corpus = retrieval.numerical_corpus(&execution).unwrap();
    let mut answered = Vec::new();
    for library in [Some("demo"), None] {
        let mut raw = serde_json::json!({"query":"packaging", "families":[domain::retrieval::Family::DocumentationDeployment]});
        if let Some(library) = library { raw["library"] = library.into(); }
        let req = request("search_evidence", raw);
        let prepared = retrieval.request(&execution, &req, None, None).await.unwrap();
        let scores = corpus.documents.iter().map(|d| DocumentScore { document: d.id, score: Some(1.0) }).collect();
        let (prepared, ranked) = retrieval.rank(&execution, prepared, scores, None).await.unwrap();
        let Response::SearchEvidence(response) = retrieval.finish(&execution, req, prepared, ranked).await.unwrap() else { panic!("evidence route"); };
        let hits = response.results.items;
        assert!(!hits.is_empty(), "first-party documentary evidence was dropped");
        assert!(hits.iter().all(|hit| hit.associated_members.is_empty()));
        for hit in &hits {
            assert!(!hit.originals.is_empty());
            for original in &hit.originals {
                let evidence = fixture.service.evidence(&execution, &GetEvidenceRequest { source: original.source.clone(), page: PageRequest::default() }).await.unwrap();
                assert_eq!(evidence.evidence.original.artifact, original.artifact);
                assert_eq!(evidence.evidence.original.digest, original.digest);
                assert_eq!(evidence.evidence.body.bytes, document[original.start as usize..original.end as usize]);
                assert!(!evidence.evidence.body.truncated);
            }
        }
        answered.push(hits);
    }
    assert_eq!(answered[0], answered[1]);
    drop(execution);
    drop(retrieval);
    fixture.finish().await;
}

#[tokio::test]
async fn one_member_in_multiple_release_captures_is_ambiguous_in_get_and_compare() {
    let fixture = ServingFixture::start_captured(b"__all__ = ['api']\ndef api():\n    pass\n", None, &["1.0", "2.0"]).await;
    let execution = fixture.service.execution().await.unwrap();
    let library = Name::new("demo").unwrap();
    let operation = support::path("demo.api");
    let get = fixture.catalog.operation(&execution, &GetOperationRequest { library: library.clone(),
        operation: operation.clone(), comparison: Optional::default(), reference_parameter: Optional::default(),
        sections: vec![], page: PageRequest::default() }).await.unwrap();
    let OperationResolution::Ambiguous { candidates } = get.operation else { panic!("multiple captures were silently selected"); };
    assert!(!candidates.is_empty());
    assert_eq!(candidates.iter().map(|c| c.member).collect::<std::collections::BTreeSet<_>>().len(), 1);
    assert!(candidates.iter().all(|c| c.releases.len() == 2));
    let compare = fixture.catalog.compare(&execution, &CompareOperationsRequest { library,
        operations: vec![operation], selection: SelectionInput::default(), page: PageRequest::default() }).await.unwrap();
    assert!(compare.operations[0].ambiguous);
    assert_eq!(compare.operations[0].candidates, candidates);
    drop(execution);
    fixture.finish().await;
}
