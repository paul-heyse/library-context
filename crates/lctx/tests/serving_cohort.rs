//! One immutable Catalog seed, fresh per-case requests, and independent failure collection.
#[path = "fixtures/serving_support.rs"]
mod support;
use futures::FutureExt;
use lctx_model::domain::{calls::ParameterKind, catalog::{CatalogDefault, CatalogMember, CatalogOption, CatalogOptionSubject}, selection::*, serving::ranking::{DocumentScore, Target}, serving::*, *};
use lctx_postgres::generations::RetrievalService;
use std::panic::AssertUnwindSafe;
use support::*;

#[tokio::test]
async fn immutable_catalog_cohort() {
    let fixture = ServingFixture::start(SOURCE).await;
    let mut failed = Vec::new();
    if AssertUnwindSafe(complete_find_cursors_selection_browse_counts_and_caller_order(&fixture)).catch_unwind().await.is_err() {
        eprintln!("failed: complete_find_cursors_selection_browse_counts_and_caller_order");
        failed.push("complete_find_cursors_selection_browse_counts_and_caller_order");
    } else { println!("passed: complete_find_cursors_selection_browse_counts_and_caller_order"); }
    if AssertUnwindSafe(canonical_search_filters_before_ranking_and_retains_original_contexts(&fixture)).catch_unwind().await.is_err() {
        eprintln!("failed: canonical_search_filters_before_ranking_and_retains_original_contexts");
        failed.push("canonical_search_filters_before_ranking_and_retains_original_contexts");
    } else { println!("passed: canonical_search_filters_before_ranking_and_retains_original_contexts"); }
    if AssertUnwindSafe(mandatory_packet_preserves_defaults_formals_contexts_and_set_hydration(&fixture)).catch_unwind().await.is_err() {
        eprintln!("failed: mandatory_packet_preserves_defaults_formals_contexts_and_set_hydration");
        failed.push("mandatory_packet_preserves_defaults_formals_contexts_and_set_hydration");
    } else { println!("passed: mandatory_packet_preserves_defaults_formals_contexts_and_set_hydration"); }
    fixture.finish().await;
    assert!(failed.is_empty(), "failed immutable cohort cases: {failed:?}");
}

async fn complete_find_cursors_selection_browse_counts_and_caller_order(fixture: &ServingFixture) {
    let execution = fixture.service.execution().await.unwrap();
    let library = Name::new("demo").unwrap();
    let all = fixture
        .catalog
        .find(
            &execution,
            &FindOperationsRequest {
                library: library.clone(),
                selection: SelectionInput::default(),
                page: PageRequest {
                    size: 100,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert!(
        all.supported
            .items
            .iter()
            .any(|r| r.name.as_str() == "demo.api")
    );
    assert!(all.unresolved.items.is_empty() && all.conflicting.items.is_empty());
    assert!(
        all.supported
            .items
            .iter()
            .all(|r| r.joint == JointApplicability::IndependentRecords)
    );
    let mut request = FindOperationsRequest {
        library: library.clone(),
        selection: SelectionInput::default(),
        page: PageRequest {
            size: 1,
            ..Default::default()
        },
    };
    let first = fixture.catalog.find(&execution, &request).await.unwrap();
    let token = first.supported.continuation.0.clone().unwrap();
    request.page.cursor = Optional(Some(token));
    request.page.size = 100;
    let next = fixture.catalog.find(&execution, &request).await.unwrap();
    let mut joined = first.supported.items;
    joined.extend(next.supported.items);
    assert_eq!(joined, all.supported.items);
    request.library = Name::new("foreign").unwrap();
    assert!(fixture.catalog.find(&execution, &request).await.is_err());
    let selection = Selection {
        requirements: vec![Requirement {
            predicate: Predicate::DeclaresParameter {
                name: "flag".into(),
            },
            quantifier: Quantifier::AnyApplicable,
        }],
        mode: Mode::Strict,
        joint: JointPolicy::IndependentRecords,
    };
    let strict = fixture
        .catalog
        .find(
            &execution,
            &FindOperationsRequest {
                library: library.clone(),
                selection: SelectionInput(selection.clone()),
                page: PageRequest {
                    size: 100,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let api = strict
        .supported
        .items
        .iter()
        .find(|r| r.name.as_str() == "demo.api")
        .expect("api supports flag");
    assert!(!api.requirements[0].claims.is_empty());
    assert!(
        api.requirements[0]
            .claims
            .iter()
            .all(|c| c.basis == EvidenceBasis::SourceDeclaration)
    );
    assert!(
        api.requirements[0]
            .claims
            .iter()
            .any(|c| !c.positive.is_empty())
    );
    assert!(
        !strict
            .supported
            .items
            .iter()
            .any(|r| r.name.as_str() == "demo.consume")
    );
    assert!(strict.unresolved.items.is_empty() && strict.conflicting.items.is_empty());
    let browse = fixture
        .catalog
        .browse(
            &execution,
            &BrowseLibraryRequest {
                library: library.clone(),
                scope: BrowseScope::Library {},
                view: BrowseView::Modules,
                selection: SelectionInput(selection.clone()),
                page: PageRequest {
                    size: 100,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let counted: u64 = browse
        .entries
        .items
        .iter()
        .map(|r| match r {
            BrowseEntry::Module { members, .. } => *members,
            _ => panic!("wrong browse view"),
        })
        .sum();
    assert_eq!(counted, strict.supported.items.len() as u64);
    let compare = fixture
        .catalog
        .compare(
            &execution,
            &CompareOperationsRequest {
                library: library.clone(),
                operations: vec![path("demo.consume"), path("demo.api"), path("demo.absent")],
                selection: SelectionInput(selection),
                page: PageRequest::default(),
            },
        )
        .await
        .unwrap();
    assert_eq!(compare.operations.len(), 3);
    assert_eq!(
        compare.operations[0].candidates[0].name.as_str(),
        "demo.consume"
    );
    assert!(
        compare.operations[0].candidates[0]
            .requirements
            .iter()
            .any(|r| r.outcome == Outcome::Contradicted)
    );
    assert!(compare.operations[2].candidates.is_empty());
    drop(execution);
    let memory = fixture.service.memory_reserved();
    for _ in 0..3 {
        let request_execution = fixture.service.execution().await.unwrap();
        fixture
            .catalog
            .find(
                &request_execution,
                &FindOperationsRequest {
                    library: library.clone(),
                    selection: SelectionInput::default(),
                    page: PageRequest::default(),
                },
            )
            .await
            .unwrap();
        drop(request_execution);
    }
    assert_eq!(
        fixture.service.memory_reserved(),
        memory,
        "prepared metadata is shared between requests"
    );
}

async fn canonical_search_filters_before_ranking_and_retains_original_contexts(fixture: &ServingFixture) {
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
}

async fn mandatory_packet_preserves_defaults_formals_contexts_and_set_hydration(fixture: &ServingFixture) {
    let execution = fixture.service.execution().await.unwrap();
    let r = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        comparison: Optional::default(),
        reference_parameter: Optional::default(),
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
    let signature = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == lctx_model::domain::calls::SignatureRole::Source)
        .expect("actual declared source signature");
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
}
