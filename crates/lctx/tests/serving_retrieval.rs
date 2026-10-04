#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{
    serving::ranking::{DocumentScore, Target},
    serving::*,
    *,
};
use lctx_postgres::generations::RetrievalService;
use support::{SOURCE, ServingFixture};

#[tokio::test]
async fn canonical_search_filters_before_ranking_and_retains_original_contexts() {
    let fixture = ServingFixture::start(SOURCE).await;
    let retrieval = RetrievalService::prepare(fixture.catalog.clone(), false)
        .await
        .unwrap();
    let execution = fixture.service.execution().await.unwrap();
    let corpus = retrieval.numerical_corpus(&execution).unwrap();
    assert!(!corpus.documents.is_empty());
    let request = Request::SearchOperations(SearchOperationsRequest {
        library: Optional(Some(Name::new("demo").unwrap())),
        query: QueryText::new("consume").unwrap(),
        selection: SelectionInput(selection::Selection {
            requirements: vec![selection::Requirement {
                predicate: selection::Predicate::DeclaresParameter {
                    name: "value".into(),
                },
                quantifier: selection::Quantifier::AnyApplicable,
            }],
            mode: selection::Mode::Strict,
            joint: selection::JointPolicy::IndependentRecords,
        }),
        page: PageRequest::default(),
    });
    let prepared = retrieval
        .request(&execution, &request, None, None)
        .await
        .unwrap();
    let tokens = retrieval
        .numerical_query(&execution, &prepared, "consume")
        .unwrap();
    assert!(
        tokens
            .tokens
            .iter()
            .any(|(_, values)| values.contains(&"consume".into()))
    );
    // A numerically superior document for an ineligible operation cannot vote into the domain.
    let scores = corpus
        .documents
        .iter()
        .map(|d| DocumentScore {
            document: d.id,
            score: Some(if d.text.contains("consume") {
                1.0
            } else {
                100.0
            }),
        })
        .collect();
    let (prepared, ranked) = retrieval
        .rank(&execution, prepared, scores, None)
        .await
        .unwrap();
    let response = retrieval
        .finish(&execution, request, prepared, ranked)
        .await
        .unwrap();
    let Response::SearchOperations(response) = response else {
        panic!("typed route")
    };
    assert!(!response.results.items.is_empty());
    assert!(response.results.items.iter().all(|r| {
        r.name.as_str() == "demo.consume"
            && r.requirements
                .iter()
                .all(|r| r.outcome == selection::Outcome::Supported)
    }));
    assert!(response.ranking.iter().all(|hit|matches!(hit.target,Target::Member{member}if response.results.items.iter().any(|c|c.member==member))));
    for hit in &response.ranking {
        for witness in &hit.witnesses {
            assert!(
                response
                    .results
                    .items
                    .iter()
                    .any(|c| c.analysis == witness.occurrence.context)
            );
        }
    }
    // A real ranked evidence winner remains addressable through its original byte owner.
    let request = Request::SearchEvidence(SearchEvidenceRequest {
        library: Optional(Some(Name::new("demo").unwrap())),
        query: QueryText::new("consume").unwrap(),
        families: vec![retrieval::Family::ApiOptions],
        page: PageRequest::default(),
    });
    let prepared = retrieval
        .request(&execution, &request, None, None)
        .await
        .unwrap();
    let scores = corpus
        .documents
        .iter()
        .map(|d| DocumentScore {
            document: d.id,
            score: Some(if d.text.contains("consume") {
                100.0
            } else {
                1.0
            }),
        })
        .collect();
    let (prepared, ranked) = retrieval
        .rank(&execution, prepared, scores, None)
        .await
        .unwrap();
    let Response::SearchEvidence(hits) = retrieval
        .finish(&execution, request, prepared, ranked)
        .await
        .unwrap()
    else {
        panic!("typed route")
    };
    assert!(!hits.results.items.is_empty());
    let winner = &hits.results.items[0];
    assert!(
        hits.ranking
            .iter()
            .any(|r| matches!(r.target,Target::Unit{unit}if unit==winner.unit))
    );
    assert!(!winner.originals.is_empty());
    for original in &winner.originals {
        let response = fixture
            .service
            .evidence(
                &execution,
                &GetEvidenceRequest {
                    source: original.source.clone(),
                    page: PageRequest::default(),
                },
            )
            .await
            .unwrap();
        assert_eq!(response.evidence.original.artifact, original.artifact);
        assert_eq!(response.evidence.original.digest, original.digest);
        assert_eq!(response.evidence.original.context, original.context);
        let captured = execution
            .query({
                let artifact = original.artifact;
                move |lease| {
                    Box::pin(async move {
                        let chunks = lease
                            .read_for::<artifact::ArtifactChunk, source::SourceArtifact>(
                                "artifact",
                                &[artifact],
                            )
                            .await?;
                        let mut rows = chunks.rows().to_vec();
                        rows.sort_by_key(|r| r.ordinal);
                        Ok(rows.into_iter().flat_map(|r| r.body.0).collect::<Vec<_>>())
                    })
                }
            })
            .await
            .unwrap();
        let body = &response.evidence.body;
        assert_eq!(body.bytes, captured[body.start as usize..body.end as usize]);
        assert!(!body.bytes.is_empty());
    }
    let request = Request::SearchEvidence(SearchEvidenceRequest {
        library: Optional(Some(Name::new("other").unwrap())),
        query: QueryText::new("consume").unwrap(),
        families: vec![retrieval::Family::ApiOptions],
        page: PageRequest::default(),
    });
    assert!(matches!(
        retrieval.request(&execution, &request, None, None).await,
        Err(lctx_postgres::generations::Error::LibraryAdmission(_))
    ));
    // Omitted filter nominates the admitted generation union.
    let Request::SearchEvidence(mut request) = request else {
        panic!("typed route")
    };
    request.library = Optional::default();
    let request = Request::SearchEvidence(request);
    let prepared = retrieval
        .request(&execution, &request, None, None)
        .await
        .unwrap();
    let scores = corpus
        .documents
        .iter()
        .map(|d| DocumentScore {
            document: d.id,
            score: Some(100.0),
        })
        .collect();
    let (prepared, ranked) = retrieval
        .rank(&execution, prepared, scores, None)
        .await
        .unwrap();
    let Response::SearchEvidence(response) = retrieval
        .finish(&execution, request, prepared, ranked)
        .await
        .unwrap()
    else {
        panic!("typed route")
    };
    assert!(!response.results.items.is_empty());
    assert!(!response.ranking.is_empty());
    assert_eq!(response.domains.len(), 1);
    assert_eq!(response.domains[0].name.as_str(), "demo");
    assert!(matches!(
        response.channels.vector,
        VectorChannel::Disabled {}
    ));
    drop(execution);
    drop(retrieval);
    fixture.finish().await;
}
