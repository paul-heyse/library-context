//! Actual pinned native origin vectors and conservative overload-selection states.
#[test]
fn native_overload_origins_preserve_original_vectors_and_selection_limits() {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        use pyrefly::state::{require::Require,state::State};
        use pyrefly_config::{config::{ConfigFile,ConfigSource},finder::ConfigFinder};
        use pyrefly_python::{module_path::ModulePath,sys_info::{PythonPlatform,PythonVersion}};
        use pyrefly_util::{arc_id::ArcId,thread_pool::ThreadCount};
        use ruff_text_size::{TextRange,TextSize};
        let root=tempfile::tempdir().unwrap();
        let text=include_str!("../../../fixtures/python/native_overload_origins/cases.py");
        let file=root.path().join("cases.py"); std::fs::write(&file,text).unwrap();
        let mut cfg=ConfigFile {
            source:ConfigSource::File(root.path().join("pyrefly.toml")),
            search_path_from_args:vec![root.path().to_path_buf()],
            disable_search_path_heuristics:true,disable_project_excludes_heuristics:true,
            enable_fallback_search_path:false,..ConfigFile::default()
        };
        cfg.python_environment.python_version=Some(PythonVersion::new(3,14,7));
        cfg.python_environment.python_platform=Some(PythonPlatform::new("linux"));
        cfg.python_environment.site_package_path=Some(vec![]);
        cfg.interpreters.skip_interpreter_query=true;
        assert!(cfg.configure().is_empty());
        let handle=cfg.handle_from_module_path(ModulePath::filesystem(file));
        let without_trace=State::new(ConfigFinder::new_constant(ArcId::new(cfg.clone())),ThreadCount::Inline);
        let mut plain=without_trace.new_transaction(Require::Exports,None);
        plain.run(std::slice::from_ref(&handle),Require::Errors,None);
        let state=State::new(ConfigFinder::new_constant(ArcId::new(cfg)),ThreadCount::Inline);
        let mut transaction=state.new_transaction(Require::Exports,None);
        transaction.run(std::slice::from_ref(&handle),Require::Everything,None);
        let answers=transaction.get_answers(&handle).unwrap();
        let plain_solutions=plain.get_solutions(&handle).expect("checking retains exported solutions");
        let traced_solutions=transaction.get_solutions(&handle).unwrap();
        assert!(plain_solutions.first_difference(&traced_solutions).is_none(),"observing traces changed exported inferred types");
        let diagnoses=|transaction:&pyrefly::state::state::Transaction<'_>| {
            transaction.get_errors([&handle]).collect_display_errors().into_iter().map(|e|(e.error_kind().to_name().to_owned(),e.msg())).collect::<Vec<_>>()
        };
        assert_eq!(diagnoses(&plain),diagnoses(&transaction),"observing origin membership changed diagnoses");
        assert!(!diagnoses(&transaction).is_empty(),"failed-overload diagnostic control is real");
        use pyrefly::alt::answers::NativeOverloadSelection as S;
        let trace=|needle:&str| {
            let call=text.find(needle).unwrap();let start=call+needle.rfind('(').unwrap();
            let range=TextRange::new(TextSize::new(start as u32),TextSize::new((call+needle.len()) as u32));
            answers.get_native_overload_trace(range).unwrap_or_else(||panic!("missing retained original trace: {needle} {range:?}"))
        };
        let selected=trace("choose(1)");
        assert_eq!(selected.selection,S::Selected);assert_eq!(selected.candidates.len(),2);
        assert!(selected.candidates.iter().all(|c|c.origin.is_some()));
        assert_ne!(selected.candidates[0].origin,selected.candidates[1].origin);
        let failed=trace("choose(1.5)");
        assert!(matches!(failed.selection,S::ClosestOnly|S::Recovered),"{failed:?}");
        let expanded=trace("choose(value)");
        assert!(matches!(expanded.selection,S::ExpandedRepresentative|S::AmbiguousRepresentative),"{expanded:?}");
        let equal=trace("equal_shape(1)");
        assert_eq!(equal.candidates.len(),2);
        assert_eq!(equal.candidates[0].callable,equal.candidates[1].callable);
        assert_ne!(equal.candidates[0].origin,equal.candidates[1].origin,"equal structural terms retain distinct original source identity");
        let recovered=trace("recover(1.5)");
        assert!(matches!(recovered.selection,S::Recovered|S::ClosestOnly),"{recovered:?}");
        for (name,t) in [("selected",selected),("failed",failed),("expanded",expanded),("equal",equal),("recovered",recovered),("generic",trace("identity(1)")),("bound",trace("Reader().read(1)"))] {
            assert!(t.closest_ordinal<t.candidates.len());
            assert!(t.candidates.iter().enumerate().all(|(i,c)|c.ordinal==i));
            println!("{name}: selection={:?}; representative={}; candidates={:?}",t.selection,t.closest_ordinal,t.candidates);
        }
    }).unwrap().join().unwrap();
}

#[path="typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{*,types::*};
inspector!(OriginFacts,NativeOverloadObservation,NativeOverloadCandidate,NativeOverloadSupport);
#[tokio::test]
async fn native_origin_vectors_survive_canonical_attachment_and_replay() {
    let tables=typed_driver::Tables::default();
    typed_driver::run(&typed_driver::files("native_overload_origins"),OriginFacts(tables.clone())).await.unwrap();
    let observations=typed_driver::rows::<NativeOverloadObservation>(&tables);
    let candidates=typed_driver::rows::<NativeOverloadCandidate>(&tables);
    assert!(!observations.is_empty());
    assert!(observations.iter().any(|r|r.selection==OverloadSelection::Selected));
    assert!(observations.iter().any(|r|matches!(r.selection,OverloadSelection::ClosestOnly|OverloadSelection::Recovered)));
    let model=lctx_model::domain::model().unwrap();
    let budget=typed_driver::budget();
    let verify=|mutation:&str| {
        let invariant=&NativeOverloadObservation::invariants()[0];
        let mut check=(invariant.create)(&budget);
        let source=tables.lock().unwrap();
        for input in &invariant.inputs {
            if input.name()==NativeOverloadCandidate::NAME {
                let mut rows=candidates.clone();
                if mutation=="drop-member" {rows.remove(0);}
                if mutation=="origin" {rows[0].origin=if rows[0].origin.is_some() {None} else {candidates.iter().find_map(|c|c.origin)};}
                if mutation=="qualification" {rows[0].qualification=observations.iter().find(|r|r.qualification!=rows[0].qualification).map(|r|r.qualification).unwrap_or_else(||assertion::AssertionQualification {context:serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new([99u8;16].into_iter())).unwrap(),scope:rows[0].scope,condition:conditions::Diagram::always().id(),modality:attribution::Modality::Definite,approximation:assertion::Approximation::Exact,assumptions:assumptions::AssumptionSet::empty_id()}.id());}
                if mutation=="drop-vector" {rows.clear();}
                let batch=Batch::new(&model,rows,&budget).unwrap();check.visit(input.name(),batch.arrow()).unwrap();
            } else if input.name()==NativeOverloadObservation::NAME && mutation=="drop-vector" {
                check.visit(input.name(),Batch::<NativeOverloadObservation>::new(&model,vec![],&budget).unwrap().arrow()).unwrap();
            } else if let Some(batch)=source.get(input.name()) {check.visit(input.name(),batch).unwrap();}
        }
        check.finish()
    };
    verify("none").unwrap();
    for mutation in ["drop-member","origin","qualification","drop-vector"] {assert!(verify(mutation).is_err(),"{mutation} escaped original native vector closure");}
}
