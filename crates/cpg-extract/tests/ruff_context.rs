//! The current linked consumer reuses a verified snapshot, not a scratch-only fork API.
use cpg_extract::ruff_context::{CanonicalSyntax, ContextSettings};
use lctx_model::domain::{attribution::AnalysisContext, resources::ResourceBudget, source::SourceArtifact, *};
use ruff_linter::semantic_facts::{Fact, Sink, StopReason};
use ruff_text_size_latest::Ranged;
#[derive(Default)]
struct Rows(Vec<Fact>);
impl Sink for Rows {
    fn observe(&mut self, row: Fact) -> Result<(), StopReason> { self.0.push(row); Ok(()) }
}
fn context() -> AnalysisContext {
    AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec!["release".into()], site_package_path: vec![], config_digest: ContentHash::of(b"explicit"), environment_digest: ContentHash::of(b"frozen"), lock_digest: None }
}
fn artifact(text: &str) -> SourceArtifact {
    SourceArtifact::from_bytes(serde_json::from_value(serde_json::json!(vec![1;16])).unwrap(), "pkg/api.py".into(), text.as_bytes()).unwrap()
}
#[test]
fn populated_context_keeps_alias_resolution_final_tables_and_exact_snapshot() {
    let source = "from typing import TYPE_CHECKING as TC\nfrom typing import final as fin\nif TC:\n    annotation_only = 1\n@fin\ndef f():\n    return missing\nfin = 0\nfin\n__all__ = ['f']\n";
    let b = ResourceBudget::fixed(16 << 20).unwrap();
    let syntax = CanonicalSyntax::parse(&artifact(source), source, &context(), ContextSettings::default(), &b).unwrap();
    assert_eq!(syntax.module().body.len(), 7);
    let mut rows = Rows::default();
    let stats = syntax.observe(&mut rows).unwrap();
    assert_eq!(stats.rows, rows.0.len());
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Branch { type_checking: true, .. })));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Node(n) if n.qualified_name.as_deref() == Some(&["typing".to_owned(), "final".to_owned()]))));
    let imported = rows.0.iter().find_map(|r| match r { Fact::Binding(b) if b.name == "fin" && b.kind == "FromImport" => Some(b.id), _ => None }).unwrap();
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Reference(r) if r.binding == imported)));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Binding(b) if b.kind == "Assignment" && b.shadowed == Some(imported))));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Unresolved { name, .. } if name == "missing")));
    assert!(rows.0.iter().any(|r| matches!(r, Fact::Export { name, .. } if name == "f")));
    assert!(syntax.module().range().end().to_u32() <= source.len() as u32);
    assert!(CanonicalSyntax::parse(&artifact(source), "different", &context(), ContextSettings::default(), &b).is_err());
    drop(syntax);
    assert_eq!(b.reserved(), 0);
}
#[test]
fn cancellation_and_limits_never_certify_complete_context() {
    struct Cancel;
    impl Sink for Cancel {
        fn checkpoint(&mut self) -> Result<(), StopReason> { Err(StopReason::Cancelled) }
        fn observe(&mut self, _: Fact) -> Result<(), StopReason> { panic!("cancelled before emission") }
    }
    let source = "x = 1\n";
    let b = ResourceBudget::fixed(1 << 20).unwrap();
    let syntax = CanonicalSyntax::parse(&artifact(source), source, &context(), ContextSettings { maximum_rows: 1, ..Default::default() }, &b).unwrap();
    assert_eq!(syntax.observe(&mut Cancel).unwrap_err().reason, StopReason::Cancelled);
    assert_eq!(syntax.observe(&mut Rows::default()).unwrap_err().reason, StopReason::RowBudget);
}

#[test]
fn contextual_attachment_refuses_foreign_source_and_context() {
    use cpg_extract::{syntax_records::Spans,typed_syntax::{self,SyntaxInvocation,SyntaxLimits}};
    use lctx_model::domain::{assertion::*,attribution::*,conditions::Diagram,source::*};
    let source="x = 1\nx\n";
    let artifact=artifact(source);
    let context=context();
    let budget=ResourceBudget::fixed(16<<20).unwrap();
    let syntax=CanonicalSyntax::parse(&artifact,source,&context,ContextSettings::default(),&budget).unwrap();
    let provider=cpg_extract::ruff_context::provider();
    let (run,_)=ProviderRun::new(provider.id(),context.id(),artifact.input,context.config_digest,[FactFamily::Syntax]).unwrap();
    let surface=ProviderSurface {provider:provider.id(),family:FactFamily::Syntax,name:"control".into()};
    let qualification=AssertionQualification {assumptions:lctx_model::domain::assumptions::AssumptionSet::empty_id(),context:context.id(),scope:CoverageScope::Artifact {artifact:artifact.id()}.id(),condition:Diagram::always().id(),modality:Modality::Definite,approximation:Approximation::Exact};
    let mut spans=Spans::new(&budget);
    typed_syntax::emit(syntax.module(),source,SyntaxInvocation {source:&artifact,qualification:&qualification,run:&run,surface:&surface},SyntaxLimits::default(),|event|spans.insert(&event.occurrence)).unwrap();
    let contexts=syntax.context_rows(&spans,&qualification,&budget).unwrap();
    assert!(contexts.incomplete.is_none());
    assert_eq!(contexts.unlocated,0);
    assert!(!contexts.rows.is_empty());
    let mut foreign=artifact.clone();foreign.path="foreign.py".into();
    let foreign_syntax=CanonicalSyntax::parse(&foreign,source,&context,ContextSettings::default(),&budget).unwrap();
    assert!(foreign_syntax.context_rows(&spans,&qualification,&budget).is_err());
    let mut other=qualification.clone();other.context=serde_json::from_value(serde_json::json!(vec![2;16])).unwrap();
    assert!(syntax.context_rows(&spans,&other,&budget).is_err());
    let occurrence=Occurrence { source:foreign.id(),start:0,end:1,syntax_kind:SyntaxKind::ExprName,role:OccurrenceRole::Read,structural_path:vec![999] };
    assert!(spans.insert(&occurrence).is_err());
}
