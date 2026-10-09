//! Actual pinned native origin vectors and conservative overload-selection states.
#[test]
fn native_overload_origins_preserve_original_vectors_and_selection_limits() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use pyrefly::state::{require::Require, state::State};
            use pyrefly_config::{
                config::{ConfigFile, ConfigSource},
                finder::ConfigFinder,
            };
            use pyrefly_python::{
                module_path::ModulePath,
                sys_info::{PythonPlatform, PythonVersion},
            };
            use pyrefly_util::{arc_id::ArcId, thread_pool::ThreadCount};
            use ruff_text_size::{TextRange, TextSize};
            let root = tempfile::tempdir().unwrap();
            let text = include_str!("../../../../fixtures/python/native_overload_origins/cases.py");
            let file = root.path().join("cases.py");
            std::fs::write(&file, text).unwrap();
            let mut cfg = ConfigFile {
                source: ConfigSource::File(root.path().join("pyrefly.toml")),
                search_path_from_args: vec![root.path().to_path_buf()],
                disable_search_path_heuristics: true,
                disable_project_excludes_heuristics: true,
                enable_fallback_search_path: false,
                ..ConfigFile::default()
            };
            cfg.python_environment.python_version = Some(PythonVersion::new(3, 14, 7));
            cfg.python_environment.python_platform = Some(PythonPlatform::new("linux"));
            cfg.python_environment.site_package_path = Some(vec![]);
            cfg.interpreters.skip_interpreter_query = true;
            assert!(cfg.configure().is_empty());
            let handle = cfg.handle_from_module_path(ModulePath::filesystem(file));
            let without_trace = State::new(
                ConfigFinder::new_constant(ArcId::new(cfg.clone())),
                ThreadCount::Inline,
            );
            let mut plain = without_trace.new_transaction(Require::Exports, None);
            plain.run(std::slice::from_ref(&handle), Require::Errors, None);
            let state = State::new(
                ConfigFinder::new_constant(ArcId::new(cfg)),
                ThreadCount::Inline,
            );
            let mut transaction = state.new_transaction(Require::Exports, None);
            transaction.run(std::slice::from_ref(&handle), Require::Everything, None);
            let answers = transaction.get_answers(&handle).unwrap();
            let plain_solutions = plain
                .get_solutions(&handle)
                .expect("checking retains exported solutions");
            let traced_solutions = transaction.get_solutions(&handle).unwrap();
            assert!(
                plain_solutions
                    .first_difference(&traced_solutions)
                    .is_none(),
                "observing traces changed exported inferred types"
            );
            let diagnoses = |transaction: &pyrefly::state::state::Transaction<'_>| {
                transaction
                    .get_errors([&handle])
                    .collect_display_errors()
                    .into_iter()
                    .map(|e| (e.error_kind().to_name().to_owned(), e.msg()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                diagnoses(&plain),
                diagnoses(&transaction),
                "observing origin membership changed diagnoses"
            );
            assert!(
                !diagnoses(&transaction).is_empty(),
                "failed-overload diagnostic control is real"
            );
            use pyrefly::alt::answers::NativeOverloadSelection as S;
            let ranges = |needle: &str| {
                let call = text.rfind(needle).unwrap();
                let start = call + needle.rfind('(').unwrap();
                (
                    TextRange::new(
                        TextSize::new(call as u32),
                        TextSize::new((call + needle.len()) as u32),
                    ),
                    TextRange::new(
                        TextSize::new(start as u32),
                        TextSize::new((call + needle.len()) as u32),
                    ),
                )
            };
            let trace = |needle: &str| {
                let (_, range) = ranges(needle);
                answers.get_native_overload_trace(range).unwrap_or_else(|| {
                    panic!("missing retained original trace: {needle} {range:?}")
                })
            };
            let selected = trace("choose(1)");
            assert_eq!(selected.selection, S::Selected);
            assert_eq!(selected.candidates.len(), 2);
            assert!(selected.candidates.iter().all(|c| c.origin.is_some()));
            assert_ne!(selected.candidates[0].origin, selected.candidates[1].origin);
            let failed = trace("choose(1.5)");
            assert!(
                matches!(failed.selection, S::ClosestOnly | S::Recovered),
                "{failed:?}"
            );
            let expanded = trace("choose(value)");
            assert!(
                matches!(
                    expanded.selection,
                    S::ExpandedRepresentative | S::AmbiguousRepresentative
                ),
                "{expanded:?}"
            );
            let equal = trace("equal_shape(1)");
            assert_eq!(equal.candidates.len(), 2);
            assert_eq!(equal.candidates[0].callable, equal.candidates[1].callable);
            assert_ne!(
                equal.candidates[0].origin, equal.candidates[1].origin,
                "equal structural terms retain distinct original source identity"
            );
            let recovered = trace("recover(1.5)");
            assert!(
                matches!(recovered.selection, S::Recovered | S::ClosestOnly),
                "{recovered:?}"
            );
            for (name, t) in [
                ("selected", selected),
                ("failed", failed),
                ("expanded", expanded),
                ("equal", equal),
                ("recovered", recovered),
            ] {
                assert!(t.closest_ordinal < t.candidates.len());
                assert!(t.candidates.iter().enumerate().all(|(i, c)| c.ordinal == i));
                println!(
                    "{name}: selection={:?}; representative={}; candidates={:?}",
                    t.selection, t.closest_ordinal, t.candidates
                );
            }
            // These actual calls retain inference, but the supplier does not populate its
            // overload map for them. Missing specialization/receiver basis stays unavailable.
            for needle in ["identity(1)", "Reader().read(1)"] {
                let (call, arguments) = ranges(needle);
                assert!(answers.get_type_trace(call).is_some());
                assert!(answers.get_all_overload_trace(arguments).is_none());
                assert!(
                    answers.get_native_overload_trace(arguments).is_none(),
                    "{needle}: missing native member basis cannot be invented"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{types::*, *};
inspector!(
    OriginFacts,
    NativeOverloadObservation,
    NativeOverloadCandidate,
    NativeOverloadSupport
);
#[tokio::test]
async fn native_origin_vectors_survive_canonical_attachment_and_replay() {
    let tables = typed_driver::Tables::default();
    typed_driver::run(
        &typed_driver::files("native_overload_origins"),
        OriginFacts(tables.clone()),
    )
    .await
    .unwrap();
    let observations = typed_driver::rows::<NativeOverloadObservation>(&tables);
    let candidates = typed_driver::rows::<NativeOverloadCandidate>(&tables);
    assert!(!observations.is_empty());
    assert!(
        observations
            .iter()
            .any(|r| r.selection == OverloadSelection::Selected)
    );
    assert!(observations.iter().any(|r| matches!(
        r.selection,
        OverloadSelection::ClosestOnly | OverloadSelection::Recovered
    )));
    let model = lctx_model::domain::model().unwrap();
    let budget = typed_driver::budget();
    let verify = |mutation: &str| {
        let invariant =
            &lctx_model::domain::validation::invariants_for::<NativeOverloadObservation>()[0];
        let mut check = (invariant.create)(&budget);
        let source = tables.lock().unwrap();
        let mut traces = observations.clone();
        let original = traces[0].id();
        if mutation == "arguments" {
            traces[0].arguments = observations
                .iter()
                .find(|t| t.arguments != traces[0].arguments)
                .expect("multiple native call sites")
                .arguments;
        }
        let changed = traces[0].id();
        for input in &invariant.inputs {
            if input.name() == NativeOverloadCandidate::NAME {
                let mut rows = candidates.clone();
                if mutation == "arguments" {
                    for row in &mut rows {
                        if row.trace == original {
                            row.trace = changed;
                        }
                    }
                }
                if mutation == "drop-member" {
                    rows.remove(0);
                }
                if mutation == "origin" {
                    rows[0].origin = if rows[0].origin.is_some() {
                        None
                    } else {
                        candidates.iter().find_map(|c| c.origin)
                    };
                }
                if mutation == "qualification" {
                    rows[0].qualification = observations
                        .iter()
                        .find(|r| r.qualification != rows[0].qualification)
                        .map(|r| r.qualification)
                        .unwrap_or_else(|| {
                            assertion::AssertionQualification {
                                context:
                                    serde::Deserialize::deserialize(
                                        serde::de::value::SeqDeserializer::<
                                            _,
                                            serde::de::value::Error,
                                        >::new(
                                            [99u8; 16].into_iter()
                                        ),
                                    )
                                    .unwrap(),
                                scope: rows[0].scope,
                                condition: conditions::Diagram::always().id(),
                                modality: attribution::Modality::Definite,
                                approximation: assertion::Approximation::Exact,
                                assumptions: assumptions::AssumptionSet::empty_id(),
                            }
                            .id()
                        });
                }
                if mutation == "drop-vector" {
                    rows.clear();
                }
                let batch = Batch::new(&model, rows, &budget).unwrap();
                check.visit(input.name(), batch.arrow()).unwrap();
            } else if input.name() == NativeOverloadObservation::NAME {
                if mutation == "drop-vector" {
                    traces.clear();
                }
                check
                    .visit(
                        input.name(),
                        Batch::<NativeOverloadObservation>::new(&model, traces.clone(), &budget)
                            .unwrap()
                            .arrow(),
                    )
                    .unwrap();
            } else if let Some(batch) = source.get(input.name()) {
                check.visit(input.name(), batch).unwrap();
            }
        }
        check.finish()
    };
    verify("none").unwrap();
    for mutation in [
        "drop-member",
        "origin",
        "qualification",
        "arguments",
        "drop-vector",
    ] {
        assert!(
            verify(mutation).is_err(),
            "{mutation} escaped original native vector closure"
        );
    }
}

#[tokio::test]
async fn actual_equal_shapes_associate_only_through_original_declaration_origins() {
    use lctx_model::domain::normalized::{
        callable_normalization, entities::ResolutionStatus, entity_normalization,
        relation_normalization,
    };
    let tables = typed_driver::Tables::default();
    typed_driver::run(
        &typed_driver::files("native_overload_origins"),
        OriginFacts(tables.clone()),
    )
    .await
    .unwrap();
    let budget = typed_driver::budget();
    let mut relations = relation_normalization::RelationData::new(&budget);
    macro_rules! facts {($($field:ident:$ty:ty => $family:ident,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){relations.facts.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(facts);
    relations.entities =
        entity_normalization::normalize(relations.facts.inputs(), &budget).unwrap();
    macro_rules! additional {($($field:ident:$ty:ty => $family:ident,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){relations.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_relation_inputs!(additional);
    let links = relation_normalization::normalize(&relations, &budget).unwrap();
    let mut data = callable_normalization::CallableData::new(&budget);
    macro_rules! native {($($field:ident:$ty:ty,)*)=>{$(for row in typed_driver::rows::<$ty>(&tables){data.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_callable_inputs!(native);
    macro_rules! entities {($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&relations.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(entities);
    macro_rules! normalized {($($field:ident:$ty:ty,)*)=>{$(
        let batch=<$ty as Record>::encode(&links.$field.iter().cloned().collect::<Vec<_>>()).unwrap();
        data.visit(<$ty>::NAME,&batch).unwrap();
    )*};}
    lctx_model::normalized_relation_outputs!(normalized);
    let output = callable_normalization::normalize(&data, &budget).unwrap();
    let start = include_str!("../../../../fixtures/python/native_overload_origins/cases.py")
        .find("equal_shape(1)")
        .unwrap();
    let trace = data
        .overload_traces
        .iter()
        .find(|trace| {
            data.occurrences
                .get(trace.site)
                .is_some_and(|site| site.start as usize == start)
        })
        .unwrap();
    let mut candidates = data
        .overload_candidates
        .iter()
        .filter(|candidate| candidate.trace == trace.id())
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.ordinal);
    assert_eq!(candidates.len(), 2);
    assert_ne!(candidates[0].origin, candidates[1].origin);
    let assessments = candidates
        .iter()
        .map(|candidate| {
            output
                .overload_assessments
                .iter()
                .find(|row| row.candidate == candidate.id())
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(
        assessments
            .iter()
            .all(|row| row.status == ResolutionStatus::Resolved),
        "actual original declaration association: {assessments:?}"
    );
    assert_ne!(
        assessments[0].variant, assessments[1].variant,
        "identical callable shapes cannot select the same declaration origin"
    );
    for row in assessments {
        assert!(
            output
                .overload_variant_candidates
                .iter()
                .any(|member| member.assessment == row.id()),
            "association lacks exact native declaration and trace support"
        );
    }
}
