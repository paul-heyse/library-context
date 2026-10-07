use super::*;
use crate::experiment::*;

fn cases() -> Vec<Case> {
    include_str!("../../../eval/programmatic/finite-cases.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
#[test]
fn independently_authored_obligation_population() {
    let expected: BTreeMap<String, String> = serde_json::from_str(include_str!(
        "../../../eval/programmatic/finite-expectations.json"
    ))
    .unwrap();
    for case in cases() {
        let judgment = judge(&case);
        assert_eq!(
            serde_json::to_value(&judgment.epistemic)
                .unwrap()
                .as_str()
                .unwrap(),
            expected[&case.task.id],
            "{}: {}",
            case.task.id,
            judgment.reason
        );
    }
}
#[test]
fn finite_relations_agree_with_separate_exhaustive_reference() {
    let expr = Witness::Exists {
        variables: vec!["variant".into()],
        child: Box::new(Witness::All {
            children: vec![
                Witness::Leaf {
                    predicate: "signature".into(),
                },
                Witness::Any {
                    children: vec![
                        Witness::Leaf {
                            predicate: "source".into(),
                        },
                        Witness::Leaf {
                            predicate: "contract".into(),
                        },
                    ],
                },
            ],
        }),
    };
    // Every subset of two release/variant alternatives in each leaf population.
    let universe = [
        Assignment::from([
            ("release".into(), "1".into()),
            ("variant".into(), "A".into()),
        ]),
        Assignment::from([
            ("release".into(), "1".into()),
            ("variant".into(), "B".into()),
        ]),
        Assignment::from([
            ("release".into(), "2".into()),
            ("variant".into(), "A".into()),
        ]),
    ];
    for mask in 0..512_usize {
        let mut leaves = BTreeMap::new();
        for (i, name) in ["signature", "source", "contract"].iter().enumerate() {
            leaves.insert(
                (*name).into(),
                universe
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| mask & (1 << (i * 3 + j)) != 0)
                    .map(|(_, row)| row.clone())
                    .collect(),
            );
        }
        assert_eq!(
            witness::evaluate(&expr, &leaves),
            witness::exhaustive_reference(&expr, &leaves),
            "mask {mask}"
        );
    }
}
#[test]
fn projection_cannot_hide_incompatible_contexts_early() {
    let mut case = cases().remove(0);
    case.task.witness = Some(Witness::All {
        children: vec![Witness::Exists {
            variables: vec!["variant".into()],
            child: Box::new(Witness::Leaf {
                predicate: "signature".into(),
            }),
        }],
    });
    assert_eq!(judge(&case).applicability, Applicability::InvalidTask);
}
#[test]
fn expansion_requires_readable_public_reference_and_matching_realization() {
    let mut case = cases().remove(0);
    case.mode = Mode::Expandable;
    case.observation.expansions.push(Expansion {
        reference: "ref:default".into(),
        operation: "section".into(),
        realization: case.observation.realization.clone(),
        status: OperationStatus::Completed,
        response_segment: Some(1),
    });
    case.observation.segments.push(serde_json::json!({"realization": case.observation.realization, "groups": [], "references": []}).to_string());
    assert_eq!(judge(&case).epistemic, Epistemic::Inconclusive);
    let mut packet: serde_json::Value =
        serde_json::from_str(&case.observation.segments[0]).unwrap();
    packet["references"]
        .as_array_mut()
        .unwrap()
        .push("ref:default".into());
    case.observation.segments[0] = packet.to_string();
    assert_eq!(judge(&case).epistemic, Epistemic::Sufficient);
    case.observation.expansions[0].realization = "foreign".into();
    assert_eq!(judge(&case).epistemic, Epistemic::Inconclusive);
}
fn experiment() -> Experiment {
    Experiment {
        revision: "1".into(),
        split: Split::Development,
        meanings: Meanings {
            task_population: "p".into(),
            split_keys: "s".into(),
            public_requests: "r".into(),
            oracle: "o".into(),
            judgment: "j".into(),
            observation: "b".into(),
            wire_schema: "w".into(),
            completeness_applicability: "c".into(),
            journey_limits: "l".into(),
            metrics: "m".into(),
            numeric_precision_ties: "n".into(),
        },
        baseline: Realization {
            observation_realization: "finite-render-1".into(),
            source: "s".into(),
            native: "not_run".into(),
            encoder: "not_requested".into(),
            scorer: "finite".into(),
            settings: BTreeMap::new(),
        },
        candidate: Realization {
            observation_realization: "finite-render-1".into(),
            source: "s".into(),
            native: "not_run".into(),
            encoder: "not_requested".into(),
            scorer: "finite".into(),
            settings: BTreeMap::new(),
        },
        changed_variables: vec![],
    }
}
#[test]
fn freeze_refuses_mixed_meanings_and_undeclared_changes() {
    let original = experiment();
    let frozen = freeze(original.clone()).unwrap();
    admit(&frozen, &original).unwrap();
    let mut changed = original;
    changed.meanings.judgment = "j2".into();
    assert!(admit(&frozen, &changed).is_err());
    changed.candidate.scorer = "new".into();
    assert!(freeze(changed).is_err());
}
#[test]
fn both_grounded_feedback_paths_and_same_packet_rejudgment() {
    let case = cases().remove(0);
    let observation = case.observation.clone();
    let mut proposal = FeedbackProposal {
        task_id: case.task.id.clone(),
        packet_digest: blake3::hash(&serde_json::to_vec(&observation).unwrap())
            .to_hex()
            .to_string(),
        observation,
        independent_basis: case.task.oracle.clone(),
        grounding: "exact independently authored source and packet".into(),
        causes: vec![
            FeedbackCause::System,
            FeedbackCause::Evaluator,
            FeedbackCause::Usability,
        ],
        proposed_change: "add precedence interpretation requirement".into(),
        affected_tasks: vec![case.task.id.clone()],
        evaluator_revision: Some("2".into()),
    };
    assert_eq!(triage(&proposal, "1").unwrap().routes.len(), 3);
    proposal.independent_basis.kind = "agent_opinion".into();
    assert!(triage(&proposal, "1").is_err());
    let mut revised = case.clone();
    revised.task.predicates[1].accepted_text = vec!["new independent requirement".into()];
    let mut request = Rejudgment {
        old_revision: "1".into(),
        new_revision: "2".into(),
        old_case: case,
        new_case: revised,
    };
    let (old, new) = rejudge(&request).unwrap();
    assert_eq!(old.epistemic, Epistemic::Sufficient);
    assert_eq!(new.epistemic, Epistemic::Insufficient);
    request.new_case.task.request.question = "different task".into();
    assert!(rejudge(&request).is_err());
}

#[test]
fn readable_conditions_and_exact_public_containers_are_required() {
    let mut case = cases().remove(0);
    case.task.predicates[1]
        .qualifications
        .push(QualificationRequirement {
            id: "setup-condition".into(),
            accepted_text: vec!["Applies only with installed transport".into()],
        });
    let mut packet: serde_json::Value =
        serde_json::from_str(&case.observation.segments[0]).unwrap();
    packet["groups"][1]["evidence"][0]["qualifications"].as_array_mut().unwrap().push(serde_json::json!({"id": "setup-condition", "text": "Applies only with installed transport"}));
    case.observation.segments[0] = packet.to_string();
    assert_eq!(judge(&case).epistemic, Epistemic::Sufficient);
    packet["groups"][1]["evidence"][0]["qualifications"][1]["text"] = "".into();
    case.observation.segments[0] = packet.to_string();
    assert_eq!(judge(&case).epistemic, Epistemic::Insufficient);
    // The old valid-substring metadata overlay cannot be admitted in current format.
    let mut wire = serde_json::to_value(&case).unwrap();
    wire["observation"]["spans"] =
        serde_json::json!([{"context": {"variant": "A"}, "anchor": "default"}]);
    assert!(serde_json::from_value::<Case>(wire).is_err());
}
#[test]
fn escaped_text_decodes_in_its_own_container_without_substring_reattribution() {
    let mut case = cases().remove(0);
    let meaningful = "default timeout is \"ten\"\nonly here";
    case.task.predicates[1].accepted_text = vec![meaningful.into()];
    let mut packet: serde_json::Value =
        serde_json::from_str(&case.observation.segments[0]).unwrap();
    packet["groups"][1]["evidence"][0]["text"] = meaningful.into();
    case.observation.segments[0] = packet.to_string();
    assert_eq!(judge(&case).epistemic, Epistemic::Sufficient);
    packet["groups"][1]["context"]["variant"] = "B".into();
    let mut unrelated = packet["groups"][1].clone();
    unrelated["context"]["variant"] = "A".into();
    unrelated["evidence"][0]["anchor"] = "unrelated".into();
    packet["groups"].as_array_mut().unwrap().push(unrelated);
    case.observation.segments[0] = packet.to_string();
    assert_eq!(judge(&case).epistemic, Epistemic::Insufficient);
}

#[test]
fn task_free_decoder_preserves_foreign_container_and_refuses_duplicate_context() {
    let case = cases()
        .into_iter()
        .find(|case| case.task.id == "foreign-variant")
        .unwrap();
    let packet = observer::decode(
        &case.observation.observer_format,
        &case.observation.segments[0],
        &case.observation.realization,
    )
    .unwrap();
    assert_eq!(packet.groups[1].context["variant"], "B");
    assert_eq!(packet.groups[1].evidence[0].anchor, "default");
    let duplicate = r#"{"realization":"finite-render-1","groups":[{"context":{"variant":"B","variant":"A"},"evidence":[]}],"references":[]}"#;
    assert!(
        observer::decode(
            &ObserverFormat::FinitePacketV1,
            duplicate,
            "finite-render-1"
        )
        .is_err()
    );
}
