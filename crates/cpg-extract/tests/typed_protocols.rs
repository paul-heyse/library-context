//! Actual located native decisions; results do not claim unconditional runtime semantics.
#[path = "typed_driver/mod.rs"] mod typed_driver;
use lctx_model::domain::{protocols::*,source::*,types::*,*};
use typed_driver::{files,rows,run};
inspector!(Protocols,NativeExitObservation,NativeExitSupport,NativeTerminalObservation,NativeTerminalSupport,NativeExitDiagnostic,NativeExitDiagnosticSupport,TypeTerm,TypeSequence,TypeSequenceMember,lctx_model::domain::value::Literal,Occurrence,SourceArtifact);

#[tokio::test]
async fn native_exit_routes_and_guarded_terminal_decisions_remain_distinct() {
    let input=files("protocol_observations");let tables=typed_driver::Tables::default();
    run(&input,Protocols(tables.clone())).await.unwrap();
    let exits=rows::<NativeExitObservation>(&tables);let terminals=rows::<NativeTerminalObservation>(&tables);
    let occurrences=rows::<Occurrence>(&tables);let artifacts=rows::<SourceArtifact>(&tables);
    let text=|subject:Id<Occurrence>| {let o=occurrences.iter().find(|o|o.id()==subject).unwrap();let a=artifacts.iter().find(|a|a.id()==o.source).unwrap();std::str::from_utf8(&input[&a.path][o.start as usize..o.end as usize]).unwrap()};
    let exit=|name:&str| exits.iter().find(|row|text(row.subject)==name).unwrap();
    assert_eq!(exits.len(),6);
    assert_eq!(exit("Known()").normal_result,exit("Known()").exceptional_result);
    assert_ne!(exit("Overloaded()").normal_result,exit("Overloaded()").exceptional_result);
    let normal=exit("NormalOnly()");assert_eq!(normal.normal_status,NativeCallStatus::NoHardDiagnostics);assert_eq!(normal.exceptional_status,NativeCallStatus::HardDiagnostics);
    let diagnostics=rows::<NativeExitDiagnostic>(&tables);assert!(diagnostics.iter().any(|row|row.exit==normal.id()&&row.phase==ExitDiagnosticPhase::Exceptional));
    assert_eq!(exit("AsyncGood()").normal_awaitability,ExitAwaitability::Awaitable);
    assert_eq!(exit("AsyncBad()").normal_awaitability,ExitAwaitability::NotAwaitable);
    assert!(diagnostics.iter().any(|row|row.exit==exit("AsyncBad()").id()&&row.phase==ExitDiagnosticPhase::Await));
    let terms=rows::<TypeTerm>(&tables);assert!(matches!(terms.iter().find(|t|t.id()==exit("gradual").receiver).unwrap(),TypeTerm::Any {..}));
    let terminal=|name:&str|terminals.iter().find(|row|text(row.subject)==name).unwrap();
    assert_eq!(terminal("declared()").decision,TerminalDecision::DeclaredOrInferredDivergence);
    assert_eq!(terminal("inferred()").decision,TerminalDecision::DeclaredOrInferredDivergence);
    assert_eq!(terminal("m.declared_method()").decision,TerminalDecision::DeclaredOrInferredDivergence);
    assert_eq!(terminal("m.inferred_method()").decision,TerminalDecision::InferredMethodReturn);
    assert_eq!(terminal("m.placeholder()").decision,TerminalDecision::NotImplementedBody);
    assert_eq!(terminal("sock.close()").decision,TerminalDecision::NarrowedNeverCallee);
    assert!(rows::<NativeExitSupport>(&tables).iter().all(|s|exits.iter().any(|e|e.id()==s.assertion)));
    assert!(rows::<NativeTerminalSupport>(&tables).iter().all(|s|terminals.iter().any(|e|e.id()==s.assertion)));
}
