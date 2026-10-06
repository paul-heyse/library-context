fn snapshot_for(byte: u8) -> lctx_model::domain::serving::SnapshotHandle {
    use lctx_model::domain::serving::{SnapshotHandle, DatabaseIdentity, Name};
    SnapshotHandle {
        semantic: lctx_model::domain::ContentHash([byte; 32]),
        realization: lctx_model::domain::ContentHash([byte; 32]),
        database: DatabaseIdentity {namespace: Name::new("lctx").unwrap(), database: Name::new(format!("snapshot_{byte}")).unwrap()},
    }
}
use lctx_model::domain::{
    ContentHash, Id, ModelError,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    resources::ResourceBudget,
    retrieval::{Family, Fragment, OriginalAnchor, Unit},
    serving::{identity::SnapshotHandle, ranking::*},
};

fn id<T>(byte: u8) -> Id<T> {
    serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
}
fn snapshot() -> SnapshotHandle {
    snapshot_for(7)
}
fn member(byte: u8) -> Target {
    Target::Member {
        member: id::<CatalogMember>(byte),
    }
}
fn unit(byte: u8) -> Target {
    Target::Unit {
        unit: id::<Unit>(byte),
    }
}
fn occurrence(target: Target, family: Family, u: u8, fragment: u8) -> Occurrence {
    Occurrence {
        target,
        unit: id(u),
        fragment: id(fragment),
        context: id::<AnalysisContext>(u),
        anchor: Some(id::<OriginalAnchor>(u)),
        family,
    }
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(16 * 1024 * 1024).unwrap()
}
#[test]
fn member_projection_shares_independent_evidence_frequencies_and_exact_occurrences() {
    let u1 = occurrence(unit(1), Family::Source, 1, 1);
    let u2 = occurrence(unit(2), Family::Source, 2, 2);
    let source = prepare(&[unit(1), unit(2)], &[u1, u2]);
    let corpus = std::sync::Arc::new(
        source
            .prepare_lexical(&[
                TextOccurrence {
                    occurrence: u1,
                    text: "common tool".into(),
                },
                TextOccurrence {
                    occurrence: u2,
                    text: "common resource".into(),
                },
            ])
            .unwrap(),
    );
    let m1 = Occurrence {
        target: member(9),
        ..u1
    };
    let service = prepare(&[member(9)], &[m1]);
    let projection = service.project_member_lexical(corpus.clone()).unwrap();
    assert!(std::ptr::eq(projection.documents(), corpus.documents()));
    assert_eq!(
        projection
            .query_tokens(&service, Family::Source, "tool")
            .unwrap()
            .tokens(),
        ["tool"]
    );
    let common = PreparedRanking::new(
        snapshot(),
        RankingPolicy::default(),
        &bindings("common"),
        &[member(9)],
        &[m1],
        &budget(),
    )
    .unwrap();
    assert!(
        projection
            .query_tokens(&common, Family::Source, "common")
            .unwrap()
            .tokens()
            .is_empty()
    );
    let scores = corpus
        .documents()
        .iter()
        .map(|d| DocumentScore {
            document: d.id,
            score: Some(if d.text == "common tool" { 1.0 } else { 100.0 }),
        })
        .collect::<Vec<_>>();
    let rows = projection.expand_scores(&service, &scores).unwrap();
    assert_eq!(rows.rows().len(), 1);
    assert_eq!(rows.rows()[0].occurrence, m1);
    assert_eq!(rows.rows()[0].score, Some(1.0));
    let foreign = Occurrence {
        context: id(99),
        ..m1
    };
    assert!(
        prepare(&[member(9)], &[foreign])
            .project_member_lexical(corpus)
            .is_err()
    );
    assert!(
        projection
            .expand_scores(
                &service,
                &[DocumentScore {
                    document: ContentHash([99; 32]),
                    score: Some(1.0)
                }]
            )
            .is_err()
    );
}
fn bindings(query: &str) -> [ChannelBinding; 2] {
    let policy = RankingPolicy::default();
    [
        ChannelBinding::lexical(&policy, query).unwrap(),
        ChannelBinding::vector(&policy, ContentHash([3; 32]), ContentHash([4; 32])).unwrap(),
    ]
}
fn prepare(targets: &[Target], occurrences: &[Occurrence]) -> PreparedRanking {
    PreparedRanking::new(
        snapshot(),
        RankingPolicy::default(),
        &bindings("tool"),
        targets,
        occurrences,
        &budget(),
    )
    .unwrap()
}
fn score(
    prepared: &PreparedRanking,
    occurrence: Occurrence,
    channel: Channel,
    value: Option<f64>,
) -> NumericalScore {
    NumericalScore {
        snapshot: snapshot(),
        occurrence,
        channel,
        channel_identity: prepared.channel(channel).unwrap().identity(),
        score: value,
    }
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-14, "{actual} != {expected}");
}

#[test]
fn exports_pinned_numerical_settings_and_tokenization() {
    let policy = RankingPolicy::default();
    assert_eq!(policy.numerical.library, NumericalLibrary::Bm25s0311);
    assert_eq!(policy.numerical.backend, NumericalBackend::Numpy);
    assert_eq!(policy.numerical.method, Bm25Method::Lucene);
    assert_eq!(
        (policy.numerical.k1, policy.numerical.b, policy.rrf_k),
        (1.5, 0.75, 60)
    );
    assert_eq!(
        tokenize("FastMCP.custom_route(v2)"),
        ["fastmcp", "custom", "route", "v2"]
    );
    assert_eq!(tokenize("Kelvin İD café 🦀"), ["kelvin", "i", "d", "caf"]);
    let mut unsupported = policy.clone();
    unsupported.rrf_k = 59;
    assert!(unsupported.identity().is_err());
}

#[test]
fn family_normalization_has_hand_expected_scores_and_actual_witnesses() {
    let a = member(1);
    let b = member(2);
    let al = occurrence(a, Family::ApiOptions, 11, 21);
    let av = occurrence(a, Family::ApiOptions, 13, 23);
    let bl = occurrence(b, Family::ApiOptions, 12, 22);
    let bv = occurrence(b, Family::ApiOptions, 14, 24);
    let bs = occurrence(b, Family::Source, 15, 25);
    let p = prepare(&[a, b], &[al, av, bl, bv, bs]);
    let rows = [
        score(&p, al, Channel::Lexical, Some(2.0)),
        score(&p, av, Channel::Vector, Some(0.1)),
        score(&p, bl, Channel::Lexical, Some(1.0)),
        score(&p, bv, Channel::Vector, Some(0.2)),
        score(&p, bs, Channel::Vector, Some(0.5)),
    ];
    let result = p.rank(&rows, &[]).unwrap();
    assert_eq!(
        result.rows().iter().map(|r| r.target).collect::<Vec<_>>(),
        [b, a]
    );
    near(result.rows()[0].score, 1.0 / 62.0 + 1.0 / 61.0);
    near(result.rows()[1].score, 1.0 / 61.0);
    let aw = &result.rows()[1].witnesses;
    assert_eq!(aw.len(), 2);
    assert_eq!(
        (aw[0].occurrence, aw[0].channel, aw[0].rank),
        (al, Channel::Lexical, 1)
    );
    assert_eq!(
        (aw[1].occurrence, aw[1].channel, aw[1].rank),
        (av, Channel::Vector, 2)
    );
    assert_ne!(aw[0].occurrence.unit, aw[1].occurrence.unit);
    assert_ne!(aw[0].occurrence.context, aw[1].occurrence.context);
    assert!(aw.iter().all(|w| w.snapshot == snapshot()
        && w.policy == p.policy_identity()
        && w.channel_identity == p.channel(w.channel).unwrap().identity()));
}

#[test]
fn independent_evidence_has_unit_identity_and_equal_family_weight() {
    let a = unit(1);
    let b = unit(2);
    let al = occurrence(a, Family::Scenario, 1, 9);
    let av = occurrence(a, Family::Scenario, 1, 4);
    let bl = occurrence(b, Family::DocumentationDeployment, 2, 5);
    let p = prepare(&[a, b], &[al, av, bl]);
    let result = p
        .rank(
            &[
                score(&p, al, Channel::Lexical, Some(2.0)),
                score(&p, av, Channel::Vector, Some(0.8)),
                score(&p, bl, Channel::Lexical, Some(0.5)),
            ],
            &[],
        )
        .unwrap();
    assert_eq!(
        result.rows().iter().map(|r| r.target).collect::<Vec<_>>(),
        [a, b]
    );
    near(result.rows()[0].score, 1.0 / 61.0);
    assert_eq!(result.rows()[0].score, result.rows()[1].score);
    assert_eq!(result.rows()[0].witnesses.len(), 2);
    assert!(
        result
            .rows()
            .iter()
            .all(|r| matches!(r.target, Target::Unit { .. }))
    );
}

#[test]
fn duplicates_are_neutral_and_best_fragment_ties_are_stable() {
    let a = member(1);
    let b = member(2);
    let a1 = occurrence(a, Family::Scenario, 3, 9);
    let a2 = occurrence(a, Family::Scenario, 3, 4);
    let a3 = occurrence(a, Family::Scenario, 4, 1);
    let b1 = occurrence(b, Family::Scenario, 5, 5);
    let p = prepare(&[a, b], &[a1, a2, a3, b1]);
    let rows = vec![
        score(&p, a1, Channel::Lexical, Some(3.0)),
        score(&p, a2, Channel::Lexical, Some(3.0)),
        score(&p, a3, Channel::Lexical, Some(3.0)),
        score(&p, b1, Channel::Lexical, Some(3.0)),
    ];
    let result = p.rank(&rows, &[]).unwrap();
    assert_eq!(result.rows()[0].witnesses.len(), 1);
    assert_eq!(result.rows()[0].witnesses[0].occurrence, a2);
    assert_eq!(result.rows()[0].witnesses[0].rank, 1);
    assert_eq!(result.rows()[1].witnesses[0].rank, 2);
    let mut duplicates = rows.clone();
    duplicates.extend(rows.iter().cycle().take(40).cloned());
    duplicates.reverse();
    assert_eq!(p.rank(&duplicates, &[]).unwrap().rows(), result.rows());
}

#[test]
fn exact_path_promotion_does_not_change_rrf_score_or_fabricate_witnesses() {
    let a = member(1);
    let b = member(2);
    let c = member(3);
    let o = occurrence(a, Family::Source, 4, 4);
    let p = prepare(&[a, b, c], &[o]);
    let rows = [score(&p, o, Channel::Vector, Some(0.3))];
    let plain = p.rank(&rows, &[]).unwrap();
    let result = p
        .rank(&rows, &[id::<CatalogMember>(3), id(2), id(3)])
        .unwrap();
    assert_eq!(
        result.rows().iter().map(|r| r.target).collect::<Vec<_>>(),
        [b, c, a]
    );
    assert_eq!(result.rows()[0].score, 0.0);
    assert!(result.rows()[0].promoted && result.rows()[0].witnesses.is_empty());
    assert_eq!(result.rows()[2].score, plain.rows()[0].score);
    assert!(p.rank(&rows, &[id::<CatalogMember>(99)]).is_err());
}

#[test]
fn missing_and_lexical_zero_abstain_but_finite_cosine_zero_and_negative_vote() {
    let a = member(1);
    let b = member(2);
    let c = member(3);
    let a1 = occurrence(a, Family::Source, 1, 1);
    let b1 = occurrence(b, Family::Source, 2, 2);
    let c1 = occurrence(c, Family::Source, 3, 3);
    let p = prepare(&[a, b, c], &[a1, b1, c1]);
    let rows = [
        score(&p, a1, Channel::Lexical, None),
        score(&p, b1, Channel::Lexical, Some(-0.0)),
        score(&p, b1, Channel::Vector, Some(0.0)),
        score(&p, c1, Channel::Vector, Some(-0.7)),
    ];
    let result = p.rank(&rows, &[]).unwrap();
    assert_eq!(
        result.rows().iter().map(|r| r.target).collect::<Vec<_>>(),
        [b, c]
    );
    assert_eq!(result.rows()[0].witnesses[0].numerical_score, 0.0);
    assert_eq!(
        result.statistics(),
        &[
            ChannelStatistics {
                channel: Channel::Lexical,
                missing: 2,
                lexical_zero: 1,
                contributing_occurrences: 0
            },
            ChannelStatistics {
                channel: Channel::Vector,
                missing: 1,
                lexical_zero: 0,
                contributing_occurrences: 2
            }
        ]
    );
}

#[test]
fn scorer_identity_and_exact_closure_are_checked_without_partial_output() {
    let a = member(1);
    let o = occurrence(a, Family::Source, 1, 1);
    let p = prepare(&[a], &[o]);
    let good = score(&p, o, Channel::Vector, Some(0.8));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut row = good.clone();
        row.score = Some(bad);
        assert!(p.rank(&[good.clone(), row], &[]).is_err());
    }
    let mut row = good.clone();
    row.occurrence.context = id(99);
    assert!(p.rank(&[row], &[]).is_err());
    let mut row = good.clone();
    row.occurrence.fragment = id::<Fragment>(99);
    assert!(p.rank(&[row], &[]).is_err());
    let mut row = good.clone();
    row.occurrence.target = member(99);
    assert!(p.rank(&[row], &[]).is_err());
    let mut row = good.clone();
    row.snapshot = snapshot_for(8);
    assert!(p.rank(&[row], &[]).is_err());
    let mut row = good.clone();
    row.channel_identity = ChannelBinding::vector(
        &RankingPolicy::default(),
        ContentHash([3; 32]),
        ContentHash([9; 32]),
    )
    .unwrap()
    .identity();
    assert!(p.rank(&[row], &[]).is_err());
    let mut row = good.clone();
    row.score = Some(0.7);
    assert!(p.rank(&[good.clone(), row], &[]).is_err());
    assert_eq!(p.rank(&[good], &[]).unwrap().rows().len(), 1);
    let lexical = score(&p, o, Channel::Lexical, Some(-0.1));
    assert!(p.rank(&[lexical], &[]).is_err());
}

#[test]
fn inactive_channels_and_mixed_or_foreign_universes_refuse() {
    let a = member(1);
    let o = occurrence(a, Family::Source, 1, 1);
    let channels = bindings("tool");
    let p = PreparedRanking::new(
        snapshot(),
        RankingPolicy::default(),
        &channels[..1],
        &[a],
        &[o],
        &budget(),
    )
    .unwrap();
    let row = NumericalScore {
        snapshot: snapshot(),
        occurrence: o,
        channel: Channel::Vector,
        channel_identity: channels[1].identity(),
        score: Some(0.1),
    };
    assert!(p.rank(&[row], &[]).is_err());
    assert!(
        PreparedRanking::new(
            snapshot(),
            RankingPolicy::default(),
            &channels,
            &[a, a],
            &[o],
            &budget()
        )
        .is_err()
    );
    assert!(
        PreparedRanking::new(
            snapshot(),
            RankingPolicy::default(),
            &channels,
            &[a, unit(1)],
            &[o],
            &budget()
        )
        .is_err()
    );
    assert!(
        PreparedRanking::new(
            snapshot(),
            RankingPolicy::default(),
            &channels,
            &[],
            &[o],
            &budget()
        )
        .is_err()
    );
    let foreign = occurrence(unit(1), Family::Source, 2, 2);
    assert!(
        PreparedRanking::new(
            snapshot(),
            RankingPolicy::default(),
            &channels,
            &[unit(1)],
            &[foreign],
            &budget()
        )
        .is_err()
    );
    assert!(
        PreparedRanking::new(
            snapshot(),
            RankingPolicy::default(),
            &[channels[0], channels[0]],
            &[a],
            &[o],
            &budget()
        )
        .is_err()
    );
}

#[test]
fn lexical_documents_deduplicate_text_and_keep_every_contextual_occurrence() {
    let a = member(1);
    let b = member(2);
    let c = member(3);
    let a1 = occurrence(a, Family::Scenario, 1, 1);
    let b1 = occurrence(b, Family::Scenario, 2, 2);
    let c1 = occurrence(c, Family::Scenario, 3, 1);
    let p = prepare(&[a, b, c], &[a1, b1, c1]);
    let corpus = p
        .prepare_lexical(&[
            TextOccurrence {
                occurrence: a1,
                text: "register tool".into(),
            },
            TextOccurrence {
                occurrence: b1,
                text: "read resource".into(),
            },
            TextOccurrence {
                occurrence: c1,
                text: "register tool".into(),
            },
            TextOccurrence {
                occurrence: a1,
                text: "register tool".into(),
            },
        ])
        .unwrap();
    assert_eq!(corpus.documents().len(), 2);
    assert_eq!(
        corpus
            .query_tokens(&p, Family::Scenario, "tool")
            .unwrap()
            .tokens(),
        ["tool"]
    );
    let doc = corpus
        .documents()
        .iter()
        .find(|d| d.text == "register tool")
        .unwrap();
    let expanded = corpus
        .expand_scores(
            &p,
            &[DocumentScore {
                document: doc.id,
                score: Some(0.25),
            }],
        )
        .unwrap();
    assert_eq!(
        expanded
            .rows()
            .iter()
            .filter(|r| r.score == Some(0.25))
            .map(|r| r.occurrence.target)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([a, c])
    );
    let ranks = p.rank(expanded.rows(), &[]).unwrap();
    assert_eq!(
        ranks.rows().iter().map(|r| r.target).collect::<Vec<_>>(),
        [a, c]
    );
    assert_eq!(ranks.rows()[1].witnesses[0].occurrence, c1);
    assert!(
        corpus
            .query_tokens(&p, Family::Scenario, "resource")
            .is_err()
    );
}

use std::collections::BTreeSet;
#[test]
fn shared_vocabulary_abstention_is_member_policy_not_evidence_policy() {
    let channels = bindings("register");
    let a = member(1);
    let b = member(2);
    let a1 = occurrence(a, Family::ApiOptions, 1, 1);
    let b1 = occurrence(b, Family::ApiOptions, 2, 2);
    let p = PreparedRanking::new(
        snapshot(),
        RankingPolicy::default(),
        &channels,
        &[a, b],
        &[a1, b1],
        &budget(),
    )
    .unwrap();
    let corpus = p
        .prepare_lexical(&[
            TextOccurrence {
                occurrence: a1,
                text: "register tool".into(),
            },
            TextOccurrence {
                occurrence: b1,
                text: "register resource long words".into(),
            },
        ])
        .unwrap();
    assert!(
        corpus
            .query_tokens(&p, Family::ApiOptions, "register")
            .unwrap()
            .tokens()
            .is_empty()
    );
    let ua = unit(1);
    let ub = unit(2);
    let u1 = occurrence(ua, Family::DocumentationDeployment, 1, 1);
    let u2 = occurrence(ub, Family::DocumentationDeployment, 2, 1);
    let p = PreparedRanking::new(
        snapshot(),
        RankingPolicy::default(),
        &channels,
        &[ua, ub],
        &[u1, u2],
        &budget(),
    )
    .unwrap();
    let corpus = p
        .prepare_lexical(&[
            TextOccurrence {
                occurrence: u1,
                text: "register deployment".into(),
            },
            TextOccurrence {
                occurrence: u2,
                text: "register deployment".into(),
            },
        ])
        .unwrap();
    assert_eq!(corpus.documents().len(), 1);
    assert_eq!(
        corpus
            .query_tokens(&p, Family::DocumentationDeployment, "register")
            .unwrap()
            .tokens(),
        ["register"]
    );
    let scores = corpus
        .expand_scores(
            &p,
            &[DocumentScore {
                document: corpus.documents()[0].id,
                score: Some(0.5),
            }],
        )
        .unwrap();
    assert_eq!(p.rank(scores.rows(), &[]).unwrap().rows().len(), 2);
}

#[test]
fn lexical_adapter_refuses_foreign_nonfinite_conflicting_and_incomplete_inputs() {
    let a = member(1);
    let b = member(2);
    let a1 = occurrence(a, Family::Source, 1, 1);
    let b1 = occurrence(b, Family::Source, 2, 2);
    let p = prepare(&[a, b], &[a1, b1]);
    assert!(
        p.prepare_lexical(&[TextOccurrence {
            occurrence: a1,
            text: "tool".into()
        }])
        .is_err()
    );
    assert!(
        p.prepare_lexical(&[
            TextOccurrence {
                occurrence: a1,
                text: "tool".into()
            },
            TextOccurrence {
                occurrence: b1,
                text: "tool".into()
            },
            TextOccurrence {
                occurrence: a1,
                text: "other".into()
            }
        ])
        .is_err()
    );
    let corpus = p
        .prepare_lexical(&[
            TextOccurrence {
                occurrence: a1,
                text: "tool".into(),
            },
            TextOccurrence {
                occurrence: b1,
                text: "tool".into(),
            },
        ])
        .unwrap();
    let doc = corpus.documents()[0].id;
    assert!(
        corpus
            .expand_scores(
                &p,
                &[DocumentScore {
                    document: ContentHash([99; 32]),
                    score: Some(1.0)
                }]
            )
            .is_err()
    );
    for score in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(
            corpus
                .expand_scores(
                    &p,
                    &[DocumentScore {
                        document: doc,
                        score: Some(score)
                    }]
                )
                .is_err()
        );
    }
    assert!(
        corpus
            .expand_scores(
                &p,
                &[
                    DocumentScore {
                        document: doc,
                        score: Some(1.0)
                    },
                    DocumentScore {
                        document: doc,
                        score: None
                    }
                ]
            )
            .is_err()
    );
    let missing = corpus.expand_scores(&p, &[]).unwrap();
    assert!(missing.rows().iter().all(|r| r.score.is_none()));
    assert!(p.rank(missing.rows(), &[]).unwrap().rows().is_empty());
}

#[test]
fn service_wide_corpus_is_reused_and_filters_before_contiguous_request_ranks() {
    let a = member(1);
    let b = member(2);
    let c = member(3);
    let a1 = occurrence(a, Family::Source, 1, 1);
    let b1 = occurrence(b, Family::Source, 2, 2);
    let c1 = occurrence(c, Family::Source, 3, 3);
    // Preparation has no request/query channel. The index and DF belong to this full corpus.
    let service = PreparedRanking::new(
        snapshot(),
        RankingPolicy::default(),
        &[],
        &[a, b, c],
        &[a1, b1, c1],
        &budget(),
    )
    .unwrap();
    let corpus = service
        .prepare_lexical(&[
            TextOccurrence {
                occurrence: a1,
                text: "tool source".into(),
            },
            TextOccurrence {
                occurrence: b1,
                text: "tool other".into(),
            },
            TextOccurrence {
                occurrence: c1,
                text: "resource".into(),
            },
        ])
        .unwrap();
    let request = prepare(&[b], &[b1]);
    assert_eq!(
        corpus
            .query_tokens(&request, Family::Source, "tool")
            .unwrap()
            .tokens(),
        ["tool"],
        "DF must not shrink to the one eligible member"
    );
    let scores: Vec<_> = corpus
        .documents()
        .iter()
        .map(|doc| DocumentScore {
            document: doc.id,
            score: Some(if doc.text == "tool source" { 10.0 } else { 1.0 }),
        })
        .collect();
    let expanded = corpus.expand_scores(&request, &scores).unwrap();
    assert_eq!(expanded.rows().len(), 1);
    assert_eq!(expanded.rows()[0].occurrence, b1);
    let result = request.rank(expanded.rows(), &[]).unwrap();
    assert_eq!(result.rows()[0].target, b);
    assert_eq!(
        result.rows()[0].witnesses[0].rank,
        1,
        "ineligible numerical winners cannot leave rank gaps"
    );
    let mut corrupt = scores.clone();
    corrupt.push(DocumentScore {
        document: ContentHash([99; 32]),
        score: Some(100.0),
    });
    assert!(
        corpus.expand_scores(&request, &corrupt).is_err(),
        "foreign scores refuse even if they would be filtered out"
    );
    let ineligible = corpus
        .documents()
        .iter()
        .find(|d| d.text == "tool source")
        .unwrap()
        .id;
    assert!(
        corpus
            .expand_scores(
                &request,
                &[DocumentScore {
                    document: ineligible,
                    score: Some(f64::NAN)
                }]
            )
            .is_err()
    );
    let other_query = PreparedRanking::new(
        snapshot(),
        RankingPolicy::default(),
        &bindings("resource"),
        &[c],
        &[c1],
        &budget(),
    )
    .unwrap();
    assert_eq!(
        corpus
            .query_tokens(&other_query, Family::Source, "resource")
            .unwrap()
            .tokens(),
        ["resource"]
    );
    let foreign_snapshot = PreparedRanking::new(
        snapshot_for(8),
        RankingPolicy::default(),
        &bindings("tool"),
        &[b],
        &[b1],
        &budget(),
    )
    .unwrap();
    assert!(corpus.expand_scores(&foreign_snapshot, &scores).is_err());
}

#[test]
fn channel_identity_binds_policy_query_and_actual_query_vector() {
    let policy = RankingPolicy::default();
    let channels = bindings("tool");
    assert_ne!(channels[0].identity(), channels[1].identity());
    assert_ne!(
        channels[0].identity(),
        ChannelBinding::lexical(&policy, "resource")
            .unwrap()
            .identity()
    );
    assert_ne!(
        channels[1].identity(),
        ChannelBinding::vector(&policy, ContentHash([8; 32]), ContentHash([4; 32]))
            .unwrap()
            .identity()
    );
    assert_ne!(
        channels[1].identity(),
        ChannelBinding::vector(&policy, ContentHash([3; 32]), ContentHash([8; 32]))
            .unwrap()
            .identity()
    );
}

#[test]
fn preparation_fusion_and_conversion_reservations_refuse_and_release() {
    let a = member(1);
    let o = occurrence(a, Family::Source, 1, 1);
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        PreparedRanking::new(
            snapshot(),
            RankingPolicy::default(),
            &bindings("tool"),
            &[a],
            &[o],
            &tiny
        ),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
    let budget = budget();
    let p = PreparedRanking::new(
        snapshot(),
        RankingPolicy::default(),
        &bindings("tool"),
        &[a],
        &[o],
        &budget,
    )
    .unwrap();
    let prepared_bytes = budget.reserved();
    assert!(prepared_bytes > 0);
    let row = score(&p, o, Channel::Vector, Some(0.5));
    let held = budget
        .reserve("test competing work", budget.limit() - budget.reserved())
        .unwrap();
    assert!(matches!(
        p.rank(std::slice::from_ref(&row), &[]),
        Err(ModelError::Resource { .. })
    ));
    assert!(matches!(
        p.prepare_lexical(&[TextOccurrence {
            occurrence: o,
            text: "tool".into()
        }]),
        Err(ModelError::Resource { .. })
    ));
    drop(held);
    assert_eq!(budget.reserved(), prepared_bytes);
    let result = p.rank(&[row], &[]).unwrap();
    assert!(budget.reserved() > prepared_bytes);
    drop(result);
    assert_eq!(budget.reserved(), prepared_bytes);
    let corpus = p
        .prepare_lexical(&[TextOccurrence {
            occurrence: o,
            text: "tool".into(),
        }])
        .unwrap();
    let corpus_bytes = budget.reserved();
    let held = budget
        .reserve("test competing work", budget.limit() - budget.reserved())
        .unwrap();
    assert!(matches!(
        corpus.expand_scores(&p, &[]),
        Err(ModelError::Resource { .. })
    ));
    assert!(matches!(
        corpus.query_tokens(&p, Family::Source, "tool"),
        Err(ModelError::Resource { .. })
    ));
    drop(held);
    let converted = corpus.expand_scores(&p, &[]).unwrap();
    assert!(budget.reserved() > corpus_bytes);
    drop(converted);
    assert_eq!(budget.reserved(), corpus_bytes);
    drop(corpus);
    drop(p);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn retained_tokenizer_known_answers_use_the_single_model_owner() {
    let answers: serde_json::Value =
        serde_json::from_str(include_str!("../../../specs/serving/tokens.json")).unwrap();
    for case in answers["text"].as_array().unwrap() {
        let expected: Vec<String> = serde_json::from_value(case["tokens"].clone()).unwrap();
        assert_eq!(tokenize(case["text"].as_str().unwrap()), expected);
    }
    for case in answers["names"].as_array().unwrap() {
        let tokens: Vec<String> = serde_json::from_value(case["tokens"].clone()).unwrap();
        assert_eq!(
            tokens
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            tokens.len()
        );
        for token in tokens {
            assert_eq!(tokenize(&token), vec![token]);
        }
    }
}
