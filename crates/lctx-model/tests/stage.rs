//! The stage scheduler (cutover plan WP0.7): a derived order and its refusal controls.

use lctx_model::stage::{
    Effect, Profile, Profiles, RelRef, StageDecl, StageError, StageTable, plan, published,
};

const fn stage(
    id: &'static str,
    inputs: &'static [RelRef],
    outputs: &'static [RelRef],
    profiles: Profiles,
) -> StageDecl {
    StageDecl {
        id,
        inputs,
        outputs,
        context: &[],
        effect: Effect::Pure,
        profiles,
        code: &[],
        legacy: false,
    }
}

use RelRef::{Legacy as L, Transient as T};

const RAW: &[RelRef] = &[L("syntax")];

#[test]
fn the_order_is_derived_with_declaration_order_as_tie_break() {
    const STAGES: &[StageDecl] = &[
        stage("summaries", &[L("flows"), T("flow_model")], &[L("summaries")], Profiles::BEHAVIORAL),
        stage("derive", &[L("syntax")], &[L("flows")], Profiles::ALL),
        stage("flow_model", &[L("flows")], &[T("flow_model"), L("value_flows")], Profiles::BEHAVIORAL),
        stage("catalog", &[L("flows")], &[L("catalog")], Profiles::ALL),
        stage("briefs", &[L("summaries"), L("catalog")], &[L("briefs")], Profiles::ALL),
    ];
    let table = StageTable { stages: STAGES, external: RAW };
    let behavioral = plan(&table, Profile::Behavioral).unwrap();
    assert_eq!(behavioral.order, ["derive", "flow_model", "summaries", "catalog", "briefs"]);
    assert!(behavioral.empty.is_empty());
    // The catalog profile skips the behavioral stages: their published outputs are empty, and a
    // reader of one still runs.
    let catalog = plan(&table, Profile::Catalog).unwrap();
    assert_eq!(catalog.order, ["derive", "catalog", "briefs"]);
    assert_eq!(catalog.empty, [L("summaries"), L("value_flows")]);
    assert_eq!(published(&table).len(), 6);
}

#[test]
fn a_published_relation_without_a_writer_is_refused() {
    const STAGES: &[StageDecl] = &[stage("a", &[L("nowhere")], &[L("x")], Profiles::ALL)];
    let errors = plan(&StageTable { stages: STAGES, external: RAW }, Profile::Catalog).unwrap_err();
    assert_eq!(errors, [StageError::MissingWriter { relation: L("nowhere"), reader: "a" }]);
}

#[test]
fn two_writers_in_one_profile_are_refused_but_one_per_profile_is_not() {
    const DOUBLE: &[StageDecl] = &[
        stage("a", &[], &[L("runs")], Profiles::ALL),
        stage("b", &[], &[L("runs")], Profiles::ALL),
    ];
    let errors = plan(&StageTable { stages: DOUBLE, external: &[] }, Profile::Catalog).unwrap_err();
    assert!(matches!(&errors[0], StageError::DoubleWriter { writers, .. } if writers == &["a", "b"]));
    const PER_PROFILE: &[StageDecl] = &[
        stage("behavior", &[], &[L("operations")], Profiles::BEHAVIORAL),
        stage("populate", &[], &[L("operations")], Profiles::CATALOG),
    ];
    let table = StageTable { stages: PER_PROFILE, external: &[] };
    assert_eq!(plan(&table, Profile::Catalog).unwrap().order, ["populate"]);
    assert_eq!(plan(&table, Profile::Behavioral).unwrap().order, ["behavior"]);
}

#[test]
fn a_cycle_and_an_unwritten_transient_are_refused() {
    const CYCLE: &[StageDecl] = &[
        stage("a", &[L("y")], &[L("x")], Profiles::ALL),
        stage("b", &[L("x")], &[L("y")], Profiles::ALL),
    ];
    let errors = plan(&StageTable { stages: CYCLE, external: &[] }, Profile::Catalog).unwrap_err();
    assert_eq!(errors, [StageError::Cycle(vec!["a", "b"])]);
    // A transient whose writer the profile skips is never read as empty.
    const TRANSIENT: &[StageDecl] = &[
        stage("flow_model", &[], &[T("flow_model")], Profiles::BEHAVIORAL),
        stage("behavior", &[T("flow_model")], &[L("behaviors")], Profiles::ALL),
    ];
    let errors = plan(&StageTable { stages: TRANSIENT, external: &[] }, Profile::Catalog).unwrap_err();
    assert_eq!(errors, [StageError::TransientUnwritten { relation: T("flow_model"), reader: "behavior" }]);
}

#[test]
fn duplicate_stages_and_written_externals_are_refused() {
    const STAGES: &[StageDecl] = &[
        stage("a", &[], &[L("syntax")], Profiles::ALL),
        stage("a", &[], &[L("z")], Profiles::ALL),
    ];
    let errors = plan(&StageTable { stages: STAGES, external: RAW }, Profile::Catalog).unwrap_err();
    assert!(errors.contains(&StageError::DuplicateStage("a")));
    assert!(errors.contains(&StageError::ExternalWritten(L("syntax"))));
}
