//! Supplier-independent checker receipt. Run unchanged before and after a native
//! observation patch; compare the JSON, including Infer/Any kinds and old getters.
#[test]
fn overload_checker_parity() {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        use pyrefly::state::{require::Require,state::State};
        use pyrefly_config::{config::{ConfigFile,ConfigSource},finder::ConfigFinder};
        use pyrefly_python::{module_path::ModulePath,sys_info::{PythonPlatform,PythonVersion}};
        use pyrefly_util::{arc_id::ArcId,thread_pool::ThreadCount};
        use ruff_python_ast_latest::{Expr,visitor::{Visitor,walk_expr}};
        use ruff_text_size_latest::Ranged;
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
        let state=State::new(ConfigFinder::new_constant(ArcId::new(cfg)),ThreadCount::Inline);
        let mut transaction=state.new_transaction(Require::Exports,None);
        transaction.run(std::slice::from_ref(&handle),Require::Everything,None);
        let answers=transaction.get_answers(&handle).unwrap();
        let canonical=|value:String|value.replace(&root.path().display().to_string(),"<fixture-root>");
        let native=|range:ruff_text_size_latest::TextRange|TextRange::new(TextSize::new(range.start().to_u32()),TextSize::new(range.end().to_u32()));
        #[derive(Default)]
        struct Calls(Vec<(ruff_text_size_latest::TextRange,ruff_text_size_latest::TextRange)>);
        impl<'a> Visitor<'a> for Calls {
            fn visit_expr(&mut self,expression:&'a Expr) {
                if let Expr::Call(call)=expression {self.0.push((call.range(),call.arguments.range()));}
                walk_expr(self,expression);
            }
        }
        let parsed=ruff_python_parser_latest::parse_module(text).unwrap();
        let mut calls=Calls::default();
        for statement in &parsed.syntax().body {calls.visit_stmt(statement);}
        calls.0.sort_by_key(|(range,_)|(range.start(),range.end()));
        let cases=[("selected","choose(1)"),("failed","choose(1.5)"),("expanded","choose(value)"),("equal","equal_shape(1)"),("recovered","recover(1.5)"),("generic","identity(1)"),("bound","Reader().read(1)"),("constructor","Reader()")];
        assert_eq!(calls.0.len(),cases.len(),"all fixture calls must carry a case ID");
        let mut observations=Vec::new();
        for (full,arguments) in calls.0 {
            let expression=&text[full.start().to_usize()..full.end().to_usize()];
            let case=cases.iter().find(|(_,source)|*source==expression).unwrap().0;
            let inferred=answers.get_type_trace(native(full));
            assert!(inferred.is_some(),"{case}: full call trace is missing");
            let chosen=answers.get_chosen_overload_trace(native(arguments));
            let all=answers.get_all_overload_trace(native(arguments));
            observations.push(serde_json::json!({"case":case,"expression":expression,"start":full.start().to_u32(),"end":full.end().to_u32(),"inferred":canonical(format!("{inferred:?}")),"chosen":canonical(format!("{chosen:?}")),"all":canonical(format!("{all:?}"))}));
        }
        let mut diagnostics=transaction.get_errors([&handle]).collect_display_errors().into_iter().map(|e|(e.error_kind().to_name().to_owned(),canonical(e.msg()))).collect::<Vec<_>>();
        diagnostics.sort();
        assert!(!diagnostics.is_empty(),"real failed-call diagnostics must be retained");
        println!("OVERLOAD_CHECKER_PARITY={}",serde_json::to_string(&serde_json::json!({"calls":observations,"diagnostics":diagnostics})).unwrap());
    }).unwrap().join().unwrap();
}
