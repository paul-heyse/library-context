//! Native retained trace qualification for the conditional C7 supply decision. The IDE
//! prediction API is deliberately not substituted for the recorded provider observation.
#[test]
fn retained_expected_trace_supports_arguments_without_proving_selection() {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        use pyrefly::state::{require::Require,state::State};
        use pyrefly_config::{config::{ConfigFile,ConfigSource},finder::ConfigFinder};
        use pyrefly_python::{module_path::ModulePath,sys_info::{PythonPlatform,PythonVersion}};
        use pyrefly_util::{arc_id::ArcId,thread_pool::ThreadCount};
        use ruff_text_size::{TextRange,TextSize};
        let root=tempfile::tempdir().unwrap();
        let text=include_str!("../../../fixtures/python/contextual_expected_arguments/cases.py");
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
        let state=State::new(ConfigFinder::new_constant(ArcId::new(cfg)),ThreadCount::Inline);
        let mut transaction=state.new_transaction(Require::Exports,None);
        transaction.run(std::slice::from_ref(&handle),Require::Everything,None);
        let answers=transaction.get_answers(&handle).unwrap();
        let expression=|needle:&str,expression:&str| {
            let start=text.find(needle).unwrap()+needle.find(expression).unwrap();
            TextRange::new(TextSize::new(start as u32),TextSize::new((start+expression.len()) as u32))
        };
        let assignment=expression("assigned: list[int] = []","[]");
        let argument=expression("consume([])","[]");
        let overloaded=expression("overloaded(1.5)","1.5");
        let assigned=answers.get_expected_type_trace(assignment);
        let call=answers.get_expected_type_trace(argument);
        let failed_overload=answers.get_expected_type_trace(overloaded);
        println!("assignment retained={assigned:?}; argument retained={call:?}; failed-overload retained={failed_overload:?}");
        assert!(assigned.is_some(),"positive retained Expected supply control");
        assert_eq!(call, assigned, "ordinary argument records its list[int] context");
        assert!(failed_overload.is_some(),"failed overload retains context, so Expected alone cannot prove success");
        assert_ne!(failed_overload, call,"failed-overload context is separate from the list argument");
    }).unwrap().join().unwrap();
}
