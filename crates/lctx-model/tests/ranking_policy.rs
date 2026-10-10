fn snapshot_for(byte: u8) -> lctx_model::domain::serving::SnapshotHandle {
    use lctx_model::domain::serving::{DatabaseIdentity, Name, SnapshotHandle};
    let mut handle=SnapshotHandle {
        publication: lctx_model::domain::ContentHash([0;32]),
        view: lctx_model::domain::ContentHash([byte;32]),
        service_generation: lctx_model::domain::ContentHash([byte;32]),
        definition_epoch: lctx_model::domain::ContentHash([byte;32]),
        semantic: lctx_model::domain::ContentHash([byte; 32]),
        realization: lctx_model::domain::ContentHash([byte; 32]),
        database: DatabaseIdentity {
            namespace: Name::new("lctx").unwrap(),
            database: Name::new(format!("snapshot_{byte}")).unwrap(),
        },
    };
    handle.publication=handle.expected_publication();handle
}
use lctx_model::domain::{
    ContentHash, Id, ModelError,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    resources::ResourceBudget,
    retrieval::{ContentPart, Family, OriginalAnchor, SearchWindow, Unit, WindowBinding},
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
        window: id(fragment),
        part: id::<ContentPart>(fragment),
        binding: Some(id::<WindowBinding>(fragment)),
        context: id::<AnalysisContext>(match target {
            Target::Member { member } => member.bytes()[0],
            Target::Unit { unit } => unit.bytes()[0],
        }),
        anchor: Some(id::<OriginalAnchor>(u)),
        family,
    }
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(16 * 1024 * 1024).unwrap()
}

fn bindings(query: &str) -> [ChannelBinding; 2] {
    let policy = RankingPolicy::default();
    [
        ChannelBinding::lexical(&policy, query).unwrap(),
        ChannelBinding::vector(
            &policy,
            ContentHash([3; 32]),
            ContentHash([4; 32]),
            ContentHash([5; 32]),
            id(6),
        )
        .unwrap(),
    ]
}
fn prepare(targets: &[Target], occurrences: &[Occurrence]) -> CandidateFusion {
    CandidateFusion::new(
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
    prepared: &CandidateFusion,
    occurrence: Occurrence,
    channel: Channel,
    value: Option<f64>,
) -> CandidateScore {
    CandidateScore {
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
    assert_eq!(aw[0].occurrence.context, aw[1].occurrence.context);
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
        [a]
    );
    assert!(!result.rows()[0].promoted);
    assert_eq!(result.rows()[0].score, plain.rows()[0].score);
    assert!(p.rank(&rows, &[id::<CatalogMember>(99)]).is_err());
}

#[test]
fn missing_abstains_but_matched_lexical_zero_and_finite_cosine_vote() {
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
    assert_eq!(result.rows()[0].witnesses[0].channel_score, 0.0);
    assert_eq!(
        result.statistics(),
        &[
            ChannelStatistics {
                channel: Channel::Lexical,
                missing: 2,
                lexical_zero: 1,
                contributing_occurrences: 1
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
    row.occurrence.window = id::<SearchWindow>(99);
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
        ContentHash([5; 32]),
        id(6),
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
    let p = CandidateFusion::new(
        snapshot(),
        RankingPolicy::default(),
        &channels[..1],
        &[a],
        &[o],
        &budget(),
    )
    .unwrap();
    let row = CandidateScore {
        snapshot: snapshot(),
        occurrence: o,
        channel: Channel::Vector,
        channel_identity: channels[1].identity(),
        score: Some(0.1),
    };
    assert!(p.rank(&[row], &[]).is_err());
    assert!(
        CandidateFusion::new(
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
        CandidateFusion::new(
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
        CandidateFusion::new(
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
        CandidateFusion::new(
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
        CandidateFusion::new(
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
        ChannelBinding::vector(
            &policy,
            ContentHash([8; 32]),
            ContentHash([4; 32]),
            ContentHash([5; 32]),
            id(6)
        )
        .unwrap()
        .identity()
    );
    assert_ne!(
        channels[1].identity(),
        ChannelBinding::vector(
            &policy,
            ContentHash([3; 32]),
            ContentHash([8; 32]),
            ContentHash([5; 32]),
            id(6)
        )
        .unwrap()
        .identity()
    );
}

#[test]
fn bounded_candidate_fusion_reservations_refuse_and_release() {
    let a = member(1);
    let o = occurrence(a, Family::Source, 1, 1);
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        CandidateFusion::new(
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
    let p = CandidateFusion::new(
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
    drop(held);
    assert_eq!(budget.reserved(), prepared_bytes);
    let result = p.rank(&[row], &[]).unwrap();
    assert!(budget.reserved() > prepared_bytes);
    drop(result);
    assert_eq!(budget.reserved(), prepared_bytes);
    drop(p);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn native_analyzer_identity_is_independent_of_fusion_rules() {
    let policy = RankingPolicy::default();
    assert_eq!(policy.rrf_k, 60);
    assert_eq!((policy.lexical.k1, policy.lexical.b), (1.5, 0.75));
    let mut changed = policy.clone();
    changed.lexical.definition = ContentHash::of(b"another installed analyzer");
    assert!(changed.validate().is_err(), "unqualified analyzer cannot reuse the native scorer");
    assert_eq!(policy.revision, 5);
    assert_eq!(policy.lexical.scoring.as_str(), "view-family-bm25-distinct-terms-v1");
    changed.rrf_k = 59;
    assert!(changed.validate().is_err());
    changed = policy.clone();
    changed.lexical.k1 = f64::NAN;
    assert!(changed.validate().is_err());
}

#[test]
fn actual_contexts_remain_independent_through_family_fusion() {
    let target = member(1);
    let first = occurrence(target, Family::ApiOptions, 2, 3);
    let mut second = occurrence(target, Family::ApiOptions, 4, 5);
    second.context = id(9);
    let p = prepare(&[target], &[first, second]);
    let result = p
        .rank(
            &[
                score(&p, first, Channel::Lexical, Some(0.0)),
                score(&p, second, Channel::Vector, Some(0.5)),
            ],
            &[],
        )
        .unwrap();
    assert_eq!(result.rows().len(), 2);
    for row in result.rows() {
        assert!(
            row.witnesses
                .iter()
                .all(|w| w.occurrence.context == row.context)
        );
    }
    assert_ne!(result.rows()[0].context, result.rows()[1].context);
}
